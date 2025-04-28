use actix_web::{web, HttpResponse, Responder, post};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use sqlx::types::Uuid;
use sqlx::Type;
use crate::db::products::create_product;

#[derive(Debug, Serialize, Deserialize)]
pub struct ProductRequest {
    pub name: String,
    pub description: String,
    pub price: f64,
    pub image: String,
    pub store_id: Uuid,
    pub category: String,
    pub popularity: i32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type)]
#[sqlx(type_name = "ProductStatus")]
pub enum ProductStatus {
    Active,
    Inactive,
    Deleted,
    Offers,
}

#[post("/register")]
async fn register_product(
    product_data: web::Json<ProductRequest>,
    pool: web::Data<PgPool>,
) -> impl Responder {
    // Validaciones
    if product_data.name.trim().is_empty() {
        return HttpResponse::BadRequest().json(web::Json(serde_json::json!({
            "error": "El nombre del producto no puede estar vacío"
        })));
    }

    if product_data.price <= 0.0 {
        return HttpResponse::BadRequest().json(web::Json(serde_json::json!({
            "error": "El precio debe ser mayor que cero"
        })));
    }

    if product_data.description.trim().is_empty() {
        return HttpResponse::BadRequest().json(web::Json(serde_json::json!({
            "error": "La descripción no puede estar vacía"
        })));
    }
    
    let product_id = create_product(
        &pool,
        &product_data.name,
        &product_data.description,
        product_data.price,
        &product_data.image,
        product_data.store_id,
        &product_data.category,
        product_data.popularity,
    ).await.unwrap();

    HttpResponse::Created().json(web::Json(serde_json::json!({
        "id": product_id,
        "message": "Producto registrado correctamente"
    })))
}

pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.service(register_product);
}
