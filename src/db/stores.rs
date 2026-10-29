use serde::Serialize;
use sqlx::{FromRow, PgPool};
use uuid::Uuid;

use crate::domain::{StoreStatus, StoreTheme};

#[derive(Debug, Serialize, FromRow)]
#[serde(rename_all = "camelCase")]
pub struct Store {
    pub id: Uuid,
    pub name: String,
    pub description: String,
    pub logo: String,
    pub cover: String,
    pub theme: StoreTheme,
    pub primary_color: Option<String>,
    pub secondary_color: Option<String>,
    pub rating: f32,
    pub delivery_time: String,
    pub categories: Vec<String>,
    pub popularity: i32,
    pub status: StoreStatus,
    pub owner_id: Option<Uuid>,
    pub address: Option<String>,
    pub latitude: Option<f64>,
    pub longitude: Option<f64>,
}

pub struct StoreFields<'a> {
    pub name: &'a str,
    pub description: &'a str,
    pub logo: &'a str,
    pub cover: &'a str,
    pub theme: StoreTheme,
    pub primary_color: Option<&'a str>,
    pub secondary_color: Option<&'a str>,
    pub delivery_time: &'a str,
    pub categories: &'a [String],
    pub status: StoreStatus,
    pub owner_id: Option<Uuid>,
    pub address: Option<&'a str>,
    pub latitude: Option<f64>,
    pub longitude: Option<f64>,
}

const STORE_COLUMNS: &str = "id, name, description, logo, cover, theme, primary_color, secondary_color, rating, \
     delivery_time, categories, popularity, status, owner_id, address, latitude, longitude";

pub async fn list_public(pool: &PgPool, category: Option<&str>, search: Option<&str>) -> sqlx::Result<Vec<Store>> {
    sqlx::query_as::<_, Store>(&format!(
        "SELECT {STORE_COLUMNS} FROM stores \
         WHERE status = 'Active' \
           AND ($1::text IS NULL OR $1 = ANY(categories)) \
           AND ($2::text IS NULL OR name ILIKE '%' || $2 || '%' OR description ILIKE '%' || $2 || '%') \
         ORDER BY popularity DESC, name"
    ))
    .bind(category)
    .bind(search)
    .fetch_all(pool)
    .await
}

pub async fn list_all(pool: &PgPool) -> sqlx::Result<Vec<Store>> {
    sqlx::query_as::<_, Store>(&format!(
        "SELECT {STORE_COLUMNS} FROM stores WHERE status <> 'Deleted' ORDER BY popularity DESC, name"
    ))
    .fetch_all(pool)
    .await
}

pub async fn list_by_owner(pool: &PgPool, owner_id: Uuid) -> sqlx::Result<Vec<Store>> {
    sqlx::query_as::<_, Store>(&format!(
        "SELECT {STORE_COLUMNS} FROM stores WHERE owner_id = $1 AND status <> 'Deleted' ORDER BY name"
    ))
    .bind(owner_id)
    .fetch_all(pool)
    .await
}

pub async fn find_by_id(pool: &PgPool, id: Uuid) -> sqlx::Result<Option<Store>> {
    sqlx::query_as::<_, Store>(&format!("SELECT {STORE_COLUMNS} FROM stores WHERE id = $1"))
        .bind(id)
        .fetch_optional(pool)
        .await
}

pub async fn create(pool: &PgPool, fields: &StoreFields<'_>) -> sqlx::Result<Store> {
    sqlx::query_as::<_, Store>(&format!(
        "INSERT INTO stores (id, name, description, logo, cover, theme, primary_color, secondary_color, rating, \
             delivery_time, categories, popularity, status, owner_id, address, latitude, longitude) \
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, 0, $9, $10, 0, $11, $12, $13, $14, $15) \
         RETURNING {STORE_COLUMNS}"
    ))
    .bind(Uuid::new_v4())
    .bind(fields.name)
    .bind(fields.description)
    .bind(fields.logo)
    .bind(fields.cover)
    .bind(fields.theme)
    .bind(fields.primary_color)
    .bind(fields.secondary_color)
    .bind(fields.delivery_time)
    .bind(fields.categories)
    .bind(fields.status)
    .bind(fields.owner_id)
    .bind(fields.address)
    .bind(fields.latitude)
    .bind(fields.longitude)
    .fetch_one(pool)
    .await
}

pub async fn update(pool: &PgPool, id: Uuid, fields: &StoreFields<'_>) -> sqlx::Result<Option<Store>> {
    sqlx::query_as::<_, Store>(&format!(
        "UPDATE stores SET name = $2, description = $3, logo = $4, cover = $5, theme = $6, primary_color = $7, \
             secondary_color = $8, delivery_time = $9, categories = $10, status = $11, owner_id = $12, \
             address = $13, latitude = $14, longitude = $15, updated_at = CURRENT_TIMESTAMP \
         WHERE id = $1 AND status <> 'Deleted' RETURNING {STORE_COLUMNS}"
    ))
    .bind(id)
    .bind(fields.name)
    .bind(fields.description)
    .bind(fields.logo)
    .bind(fields.cover)
    .bind(fields.theme)
    .bind(fields.primary_color)
    .bind(fields.secondary_color)
    .bind(fields.delivery_time)
    .bind(fields.categories)
    .bind(fields.status)
    .bind(fields.owner_id)
    .bind(fields.address)
    .bind(fields.latitude)
    .bind(fields.longitude)
    .fetch_optional(pool)
    .await
}

pub async fn soft_delete(pool: &PgPool, id: Uuid) -> sqlx::Result<bool> {
    let result = sqlx::query(
        "UPDATE stores SET status = 'Deleted', updated_at = CURRENT_TIMESTAMP WHERE id = $1 AND status <> 'Deleted'",
    )
    .bind(id)
    .execute(pool)
    .await?;
    Ok(result.rows_affected() > 0)
}

pub async fn is_owned_by(pool: &PgPool, store_id: Uuid, owner_id: Uuid) -> sqlx::Result<bool> {
    sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM stores WHERE id = $1 AND owner_id = $2 AND status <> 'Deleted')")
        .bind(store_id)
        .bind(owner_id)
        .fetch_one(pool)
        .await
}
