use sqlx::{query, Pool, Postgres, Row};
use serde::{Serialize, Deserialize};
use actix_web::web;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum StoreStatus {
    Active,
    Inactive,
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

pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.service(register_store);
}
