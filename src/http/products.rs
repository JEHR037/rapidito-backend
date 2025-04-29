use actix_web::{web, HttpResponse, Responder, post, put, delete};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use crate::db::products::create_product;
use crate::db::products::update_product as import_update;
use crate::db::products::delete_product as import_delete;

use sqlx::types::Uuid;

#[derive(Debug, Serialize, Deserialize)]
pub struct ProductRequest {
    pub name: String,
    pub description: String,
    pub subprice: i64,
    pub price: i64,
    pub image: String,
    pub store_id: Uuid,
    pub category: String,
    pub popularity: i32,
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
    if product_data.description.trim().is_empty() {
        return HttpResponse::BadRequest().json(web::Json(serde_json::json!({
            "error": "La descripción no puede estar vacía"
        })));
    }
    if product_data.image.trim().is_empty() {
        return HttpResponse::BadRequest().json(web::Json(serde_json::json!({
            "error": "La imagen no puede estar vacía"
        })));
    }
    if product_data.store_id.is_nil() {
        return HttpResponse::BadRequest().json(web::Json(serde_json::json!({
            "error": "El ID de la tienda no puede estar vacío"
        })));
    }
    if product_data.popularity < 0 {
        return HttpResponse::BadRequest().json(web::Json(serde_json::json!({
            "error": "La popularidad no puede ser negativa"
        })));
    }

    if product_data.price <= 0 {
        return HttpResponse::BadRequest().json(web::Json(serde_json::json!({
            "error": "El precio debe ser mayor que cero"
        })));
    }

    if product_data.subprice <= 0 {
        return HttpResponse::BadRequest().json(web::Json(serde_json::json!({
            "error": "El subprecio debe ser mayor que cero"
        })));
    }
    
    let product_id = create_product(
        &pool,
        &product_data.name,
        &product_data.description,
        product_data.subprice,
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

#[put("/update/{id}")]
async fn update_product(
    product_id: web::Path<Uuid>,
    product_data: web::Json<ProductRequest>,
    pool: web::Data<PgPool>,
) -> impl Responder {
    let result = import_update(
        &pool,
        product_id.into_inner(),
        Some(&product_data.name),
        Some(&product_data.description),
        Some(product_data.price),
        Some(product_data.subprice),
        Some(crate::db::products::ProductStatus::Active),
    ).await;

    match result {
        Ok(_) => HttpResponse::Ok().json(web::Json(serde_json::json!({
            "message": "Producto actualizado correctamente"
        }))),
        Err(_) => HttpResponse::InternalServerError().json("Error al actualizar el producto"),
    }
}

#[delete("/delete/{id}")]
async fn delete_product(
    product_id: web::Path<Uuid>,
    pool: web::Data<PgPool>,
) -> impl Responder {
    let result = import_delete(&pool, product_id.into_inner()).await;

    match result {
        Ok(_) => HttpResponse::Ok().json(web::Json(serde_json::json!({
            "message": "Producto eliminado correctamente"
        }))),
        Err(_) => HttpResponse::InternalServerError().json("Error al eliminar el producto"),
    }
}

pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/products")
            .service(register_product)
    );
}
