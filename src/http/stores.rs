use actix_web::{web, HttpResponse, Responder, post};
use crate::db::stores::register_store;
use crate::db::stores::StoreStatus;
use crate::db::stores::StoreTheme;
use serde::{Serialize, Deserialize};
use sqlx::types::Uuid;
use sqlx::Pool;
use sqlx::Postgres;

#[derive(Debug, Serialize, Deserialize)]
pub struct StoreRequest {
    pub name: String,
    pub description: String,
    pub logo: String,
    pub cover: String,
    pub theme: StoreTheme,
    pub rating: f64,
    pub delivery_time: String,
    pub categories: Vec<String>,
    pub status: StoreStatus,
    pub popularity: i32,
}

#[post("/register")]
async fn register_store_handler(
    store_data: web::Json<StoreRequest>,
    pool: web::Data<Pool<Postgres>>,
) -> impl Responder {
    let store_id = Uuid::new_v4();
    let status = StoreStatus::Active;
    let result = register_store(
        &pool,
        store_id,
        store_data.name.clone(),
        store_data.description.clone(),
        store_data.logo.clone(),
        store_data.cover.clone(),
        store_data.theme.clone(),
        store_data.rating,
        store_data.delivery_time.clone(),
        store_data.categories.clone(),
        status,
        store_data.popularity,
    ).await;

    match result {
        Ok(_) => HttpResponse::Created().json("Store registered successfully"),
        Err(e) => HttpResponse::InternalServerError().json(format!("Error: {}", e)),
    }
}

pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/stores")
            .service(register_store_handler)
    );
}
