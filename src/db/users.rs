use chrono::{DateTime, Utc};
use serde::Serialize;
use sqlx::{FromRow, PgExecutor, PgPool};
use uuid::Uuid;

use crate::domain::{UserRole, UserStatus};

#[derive(Debug, Clone, Serialize, FromRow)]
#[serde(rename_all = "camelCase")]
pub struct User {
    pub id: Uuid,
    pub name: String,
    pub email: String,
    pub role: UserRole,
    pub status: UserStatus,
    pub phone: Option<String>,
    pub avatar: Option<String>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, FromRow)]
pub struct Credentials {
    pub id: Uuid,
    pub password_hash: String,
    pub status: UserStatus,
}

#[derive(Debug, Default)]
pub struct ProfileChanges<'a> {
    pub name: Option<&'a str>,
    pub phone: Option<Option<&'a str>>,
    pub avatar: Option<Option<&'a str>>,
}

const USER_COLUMNS: &str = "id, name, email, role, status, phone, avatar, created_at";

pub async fn find_by_id<'e>(executor: impl PgExecutor<'e>, id: Uuid) -> sqlx::Result<Option<User>> {
    sqlx::query_as::<_, User>(&format!("SELECT {USER_COLUMNS} FROM users WHERE id = $1"))
        .bind(id)
        .fetch_optional(executor)
        .await
}

pub async fn find_role_and_status(pool: &PgPool, id: Uuid) -> sqlx::Result<Option<(UserRole, UserStatus)>> {
    sqlx::query_as("SELECT role, status FROM users WHERE id = $1")
        .bind(id)
        .fetch_optional(pool)
        .await
}

pub async fn find_credentials(pool: &PgPool, email: &str) -> sqlx::Result<Option<Credentials>> {
    sqlx::query_as::<_, Credentials>(
        "SELECT id, password_hash, status FROM users WHERE lower(email) = lower($1)",
    )
    .bind(email)
    .fetch_optional(pool)
    .await
}

pub async fn email_exists(pool: &PgPool, email: &str) -> sqlx::Result<bool> {
    sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM users WHERE lower(email) = lower($1))")
        .bind(email)
        .fetch_one(pool)
        .await
}

pub async fn create(pool: &PgPool, name: &str, email: &str, password_hash: &str) -> sqlx::Result<User> {
    sqlx::query_as::<_, User>(&format!(
        "INSERT INTO users (id, name, email, password_hash, role, status) \
         VALUES ($1, $2, $3, $4, 'Customer', 'Active') RETURNING {USER_COLUMNS}"
    ))
    .bind(Uuid::new_v4())
    .bind(name)
    .bind(email)
    .bind(password_hash)
    .fetch_one(pool)
    .await
}

pub async fn update_profile(pool: &PgPool, id: Uuid, changes: &ProfileChanges<'_>) -> sqlx::Result<Option<User>> {
    sqlx::query_as::<_, User>(&format!(
        "UPDATE users SET \
            name = COALESCE($2, name), \
            phone = CASE WHEN $3 THEN $4 ELSE phone END, \
            avatar = CASE WHEN $5 THEN $6 ELSE avatar END, \
            updated_at = CURRENT_TIMESTAMP \
         WHERE id = $1 RETURNING {USER_COLUMNS}"
    ))
    .bind(id)
    .bind(changes.name)
    .bind(changes.phone.is_some())
    .bind(changes.phone.flatten())
    .bind(changes.avatar.is_some())
    .bind(changes.avatar.flatten())
    .fetch_optional(pool)
    .await
}

pub async fn update_role_and_status<'e>(
    executor: impl PgExecutor<'e>,
    id: Uuid,
    role: Option<UserRole>,
    status: Option<UserStatus>,
) -> sqlx::Result<Option<User>> {
    sqlx::query_as::<_, User>(&format!(
        "UPDATE users SET \
            role = COALESCE($2, role), \
            status = COALESCE($3, status), \
            updated_at = CURRENT_TIMESTAMP \
         WHERE id = $1 RETURNING {USER_COLUMNS}"
    ))
    .bind(id)
    .bind(role)
    .bind(status)
    .fetch_optional(executor)
    .await
}

pub async fn list(pool: &PgPool, role: Option<UserRole>, search: Option<&str>) -> sqlx::Result<Vec<User>> {
    sqlx::query_as::<_, User>(&format!(
        "SELECT {USER_COLUMNS} FROM users \
         WHERE ($1::userrole IS NULL OR role = $1) \
           AND ($2::text IS NULL OR name ILIKE '%' || $2 || '%' OR email ILIKE '%' || $2 || '%') \
         ORDER BY created_at DESC"
    ))
    .bind(role)
    .bind(search)
    .fetch_all(pool)
    .await
}
