use sqlx::{query, Pool, Postgres, Row};
use serde::{Serialize, Deserialize};
use sqlx::types::Uuid;
use sqlx::Type;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type)]
#[sqlx(type_name = "StoreStatus")]
pub enum StoreStatus {
    Active,
    Inactive,
    Deleted,
    Offers,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type)]
#[sqlx(type_name = "StoreTheme")]
pub enum StoreTheme {
    Default,
    Custom,
}

pub async fn get_store_by_id(pool: &Pool<Postgres>, store_id: i32) -> Result<(i32, String), sqlx::Error> {
    let row = query("SELECT id, name FROM stores WHERE id = $1")
        .bind(store_id)
        .fetch_one(pool)
        .await?;
    
    let store_id: i32 = row.get(0);
    let store_name: String = row.get(1);
    Ok((store_id, store_name))
}

pub async fn register_store(
    pool: &Pool<Postgres>,
    store_id: Uuid,
    store_name: String,
    store_description: String,
    store_logo: String,
    store_cover: String,
    store_theme: StoreTheme,
    store_rating: f64,
    store_delivery_time: String,
    store_categories: Vec<String>,
    status: StoreStatus,
    popularity: i32,
) -> Result<(), sqlx::Error> {  
    query("INSERT INTO stores (id, name, description, logo, cover, theme, rating, delivery_time, categories, status, popularity, created_at, updated_at) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP)")
        .bind(store_id)
        .bind(store_name)
        .bind(store_description)
        .bind(store_logo)
        .bind(store_cover)
        .bind(store_theme)
        .bind(store_rating)
        .bind(store_delivery_time)
        .bind(store_categories)
        .bind(status)
        .bind(popularity)
        .execute(pool)
        .await?;
    Ok(())
}   

pub async fn update_store_status(pool: &Pool<Postgres>, store_id: Uuid, status: StoreStatus) -> Result<(), sqlx::Error> {
    query("UPDATE stores SET status = $1 WHERE id = $2")
        .bind(status)
        .bind(store_id)
        .execute(pool)
        .await?;
    Ok(())
}


