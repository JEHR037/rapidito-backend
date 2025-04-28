use sqlx::{query, Pool, Postgres, Row};
use sqlx::PgPool;
use sqlx::types::Uuid;  
use serde::{Serialize, Deserialize};
use sqlx::Type;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type)]
pub enum UserStatus {
    Active,
    Inactive,
    Deleted,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type)]
pub enum UserRole {
    Admin,
    Partner,
    Driver,
    Customer,
}

pub async fn get_user_by_email(pool: &Pool<Postgres>, email: &str) -> Result<(Uuid, String, String, String, UserStatus, UserRole), sqlx::Error> {
    let row = query("SELECT id, email, password_hash, name, status, role FROM users WHERE email = $1")
        .bind(email)
        .fetch_one(pool)
        .await?;
    let user_id: Uuid = row.get(0);
    let user_email: String = row.get(1);
    let user_password_hash: String = row.get(2);
    let user_name: String = row.get(3);
    let user_status: UserStatus = row.get(4);
    let user_role: UserRole = row.get(5);
    Ok((user_id, user_email, user_password_hash, user_name, user_status, user_role))
}

pub async fn get_user_by_id(pool: &Pool<Postgres>, user_id: Uuid) -> Result<(String, String, UserStatus), sqlx::Error> {
    let row = query("SELECT email, name, status FROM users WHERE id = $1")
        .bind(user_id)
        .fetch_one(pool)
        .await.map_err(|e| {
            eprintln!("Error al obtener el usuario: {}", e);
            e
        })?;

    let email: String = row.get(0);
    let name: String = row.get(1);
    let status: UserStatus = row.get(2);

    Ok((email, name, status))
}

pub async fn create_user(pool: &PgPool, email: &str, name: &str, role: UserRole, status: UserStatus, password_hash: &str) -> Result<Uuid, sqlx::Error> {
    let user_id = Uuid::new_v4(); 
    query("INSERT INTO users (id, email, name, role, status, password_hash, created_at, updated_at) VALUES ($1, $2, $3, $4, $5, $6, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP)")
        .bind(user_id)
        .bind(email)
        .bind(name)
        .bind(role)
        .bind(status)
        .bind(password_hash)
        .execute(pool)
        .await?;
    Ok(user_id)
}

pub async fn update_user_status(pool: &Pool<Postgres>, user_id: Uuid, status: UserStatus) -> Result<(), sqlx::Error> {
    let status_str = match status {
        UserStatus::Active => "active",
        UserStatus::Inactive => "inactive",
        UserStatus::Deleted => "deleted",
    };
    query("UPDATE users SET status = $1 WHERE id = $2")
        .bind(status_str)
        .bind(user_id)
        .execute(pool)
        .await?;
    Ok(())
}
