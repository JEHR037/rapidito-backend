use actix_web::{web, HttpResponse, Responder, post};
use crate::db::stores::register_store;
use crate::db::stores::StoreStatus;
use crate::db::stores::StoreTheme;
use serde::{Serialize, Deserialize};
use sqlx::types::Uuid;
use sqlx::Pool;
use sqlx::Postgres;
use serde_json;

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
    // Validaciones
    if store_data.name.trim().is_empty() {
        return HttpResponse::BadRequest().json(web::Json(serde_json::json!({
            "error": "El nombre de la tienda no puede estar vacío"
        })));
    }
    if store_data.description.trim().is_empty() {
        return HttpResponse::BadRequest().json(web::Json(serde_json::json!({
            "error": "La descripción no puede estar vacía"
        })));
    }
    if store_data.logo.trim().is_empty() {
        return HttpResponse::BadRequest().json(web::Json(serde_json::json!({
            "error": "El logo no puede estar vacío"
        })));
    }
    if store_data.cover.trim().is_empty() {
        return HttpResponse::BadRequest().json(web::Json(serde_json::json!({
            "error": "La portada no puede estar vacía"
        })));
    }
    if store_data.delivery_time.trim().is_empty() {
        return HttpResponse::BadRequest().json(web::Json(serde_json::json!({
            "error": "El tiempo de entrega no puede estar vacío"
        })));
    }
    if store_data.categories.is_empty() {
        return HttpResponse::BadRequest().json(web::Json(serde_json::json!({
            "error": "Las categorías no pueden estar vacías"
        })));
    }

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
