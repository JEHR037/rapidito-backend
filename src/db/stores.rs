use sqlx::{query, Pool, Postgres};
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



pub async fn update_store(pool: &Pool<Postgres>, store_id: Uuid, name: Option<&str>, description: Option<&str>, logo: Option<&str>, cover: Option<&str>, status: Option<StoreStatus>) -> Result<(), sqlx::Error> {
    let mut query = query("UPDATE stores SET updated_at = CURRENT_TIMESTAMP");
    
    if let Some(n) = name {
        query = query.bind(n);
    }
    if let Some(d) = description {
        query = query.bind(d);
    }
    if let Some(l) = logo {
        query = query.bind(l);
    }
    if let Some(c) = cover {
        query = query.bind(c);
    }
    if let Some(s) = status {
        query = query.bind(s);
    }
    
    query.bind(store_id).execute(pool).await?;
    Ok(())
}

pub async fn delete_store(pool: &Pool<Postgres>, store_id: Uuid) -> Result<(), sqlx::Error> {
    query("DELETE FROM stores WHERE id = $1")
        .bind(store_id)
        .execute(pool)
        .await?;
    Ok(())
}


