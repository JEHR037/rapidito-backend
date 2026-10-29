use serde::Serialize;
use sqlx::{FromRow, PgPool};
use uuid::Uuid;

use crate::domain::ProductStatus;

#[derive(Debug, Serialize, FromRow)]
#[serde(rename_all = "camelCase")]
pub struct Product {
    pub id: Uuid,
    pub store_id: Uuid,
    pub store_name: String,
    pub name: String,
    pub description: String,
    pub price_cents: i64,
    pub compare_at_price_cents: Option<i64>,
    pub image: String,
    pub category: String,
    pub popularity: i32,
    pub status: ProductStatus,
}

pub struct ProductFields<'a> {
    pub store_id: Uuid,
    pub name: &'a str,
    pub description: &'a str,
    pub price_cents: i64,
    pub compare_at_price_cents: Option<i64>,
    pub image: &'a str,
    pub category: &'a str,
    pub status: ProductStatus,
}

pub struct ProductFilter<'a> {
    pub store_id: Option<Uuid>,
    pub category: Option<&'a str>,
    pub search: Option<&'a str>,
    pub limit: i64,
}

const PRODUCT_SELECT: &str = "SELECT p.id, p.store_id, s.name AS store_name, p.name, p.description, p.price_cents, \
     p.compare_at_price_cents, p.image, p.category, p.popularity, p.status \
     FROM products p JOIN stores s ON s.id = p.store_id";

pub async fn list_public(pool: &PgPool, filter: &ProductFilter<'_>) -> sqlx::Result<Vec<Product>> {
    sqlx::query_as::<_, Product>(&format!(
        "{PRODUCT_SELECT} \
         WHERE p.status IN ('Active', 'Offers') AND s.status = 'Active' \
           AND ($1::uuid IS NULL OR p.store_id = $1) \
           AND ($2::text IS NULL OR p.category = $2) \
           AND ($3::text IS NULL OR p.name ILIKE '%' || $3 || '%' OR p.description ILIKE '%' || $3 || '%') \
         ORDER BY p.popularity DESC, p.name LIMIT $4"
    ))
    .bind(filter.store_id)
    .bind(filter.category)
    .bind(filter.search)
    .bind(filter.limit)
    .fetch_all(pool)
    .await
}

pub async fn list_purchasable_by_store(pool: &PgPool, store_id: Uuid) -> sqlx::Result<Vec<Product>> {
    sqlx::query_as::<_, Product>(&format!(
        "{PRODUCT_SELECT} WHERE p.store_id = $1 AND p.status IN ('Active', 'Offers') ORDER BY p.popularity DESC, p.name"
    ))
    .bind(store_id)
    .fetch_all(pool)
    .await
}

pub async fn find_by_id(pool: &PgPool, id: Uuid) -> sqlx::Result<Option<Product>> {
    sqlx::query_as::<_, Product>(&format!("{PRODUCT_SELECT} WHERE p.id = $1"))
        .bind(id)
        .fetch_optional(pool)
        .await
}

pub async fn create(pool: &PgPool, fields: &ProductFields<'_>) -> sqlx::Result<Uuid> {
    sqlx::query_scalar(
        "INSERT INTO products (id, store_id, name, description, price_cents, compare_at_price_cents, image, category, popularity, status) \
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, 0, $9) RETURNING id",
    )
    .bind(Uuid::new_v4())
    .bind(fields.store_id)
    .bind(fields.name)
    .bind(fields.description)
    .bind(fields.price_cents)
    .bind(fields.compare_at_price_cents)
    .bind(fields.image)
    .bind(fields.category)
    .bind(fields.status)
    .fetch_one(pool)
    .await
}

pub async fn update(pool: &PgPool, id: Uuid, fields: &ProductFields<'_>) -> sqlx::Result<bool> {
    let result = sqlx::query(
        "UPDATE products SET store_id = $2, name = $3, description = $4, price_cents = $5, compare_at_price_cents = $6, \
             image = $7, category = $8, status = $9, updated_at = CURRENT_TIMESTAMP \
         WHERE id = $1 AND status <> 'Deleted'",
    )
    .bind(id)
    .bind(fields.store_id)
    .bind(fields.name)
    .bind(fields.description)
    .bind(fields.price_cents)
    .bind(fields.compare_at_price_cents)
    .bind(fields.image)
    .bind(fields.category)
    .bind(fields.status)
    .execute(pool)
    .await?;
    Ok(result.rows_affected() > 0)
}

pub async fn soft_delete(pool: &PgPool, id: Uuid) -> sqlx::Result<bool> {
    let result = sqlx::query(
        "UPDATE products SET status = 'Deleted', updated_at = CURRENT_TIMESTAMP WHERE id = $1 AND status <> 'Deleted'",
    )
    .bind(id)
    .execute(pool)
    .await?;
    Ok(result.rows_affected() > 0)
}
