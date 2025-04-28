use actix_web::{web, HttpResponse, Responder, post};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use sqlx::Type;
use crate::db::orders::create_order;
use sqlx::types::Uuid;
use sqlx::Pool;
use sqlx::Postgres;
use sqlx::query;
use crate::db::orders::Order;
use crate::db::orders::OrderStatus;

#[derive(Debug, Serialize, Deserialize)]
pub struct OrderRequest {
    pub description: String,
    pub store_id: Uuid,
    pub driver_id: Option<Uuid>,
    pub total: i64,
    pub products: Vec<ProductOrder>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ProductOrder {
    pub product_id: Uuid,
    pub quantity: i32,
    pub subprice: f64,
    pub price: f64,
}

#[post("/register")]
async fn register_order(
    order_data: web::Json<OrderRequest>,
    pool: web::Data<Pool<Postgres>>,
) -> impl Responder {
    // Validaciones
    if order_data.description.trim().is_empty() {
        return HttpResponse::BadRequest().json(web::Json(serde_json::json!({
            "error": "La descripción no puede estar vacía"
        })));
    }

    if order_data.total <= 0 {
        return HttpResponse::BadRequest().json(web::Json(serde_json::json!({
            "error": "El total debe ser mayor que cero"
        })));
    }

    if order_data.store_id.is_nil() {
        return HttpResponse::BadRequest().json(web::Json(serde_json::json!({
            "error": "El store_id no puede estar vacío"
        })));
    }

    if order_data.driver_id.is_none() {
        return HttpResponse::BadRequest().json(web::Json(serde_json::json!({
            "error": "El driver_id no puede estar vacío"
        })));
    }

    if order_data.products.is_empty() {
        return HttpResponse::BadRequest().json(web::Json(serde_json::json!({
            "error": "El products no puede estar vacío"
        })));
    }
    
    // Validar que los productos existan
    for product in &order_data.products {
        if product.product_id.is_nil() {
            return HttpResponse::BadRequest().json(web::Json(serde_json::json!({
                "error": "El product_id no puede estar vacío"
            })));   
        }
    }
    
    // Crear la orden
    let order = Order {
        id: Uuid::new_v4(), // Genera un nuevo UUID para la orden
        description: order_data.description.clone(),
        status: OrderStatus::Pending, // O el estado que desees
        store_id: order_data.store_id,
        customer_id: Uuid::new_v4(), // Reemplaza con el UUID del cliente si lo tienes
        driver_id: order_data.driver_id.unwrap(), // Asegúrate de manejar el caso de Option
        subtotal: order_data.total, // Calcula el subtotal según tus productos
        total: order_data.total,
        address: String::new(), // Reemplaza con la dirección si la tienes
        latitude: 0, // Reemplaza con la latitud si la tienes
        longitude: 0, // Reemplaza con la longitud si la tienes
        estimated_delivery_time: String::new(), // Reemplaza con el tiempo estimado si lo tienes
        phone: String::new(), // Reemplaza con el número de teléfono si lo tienes
        payment_method: String::new(), // Reemplaza con el método de pago si lo tienes
    };

    let order_id = create_order(&pool, order).await.unwrap();

    // Insertar productos en order_items
    for product in &order_data.products {
        query("INSERT INTO order_items (order_id, product_id, quantity, subprice, price, created_at, updated_at) VALUES ($1, $2, $3, $4, $5, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP)")
            .bind(order_id) // No necesitas generar un nuevo UUID para el item
            .bind(product.product_id)
            .bind(product.quantity)
            .bind(product.subprice)
            .bind(product.price)
            .execute(&**pool)
            .await.unwrap();
    }

    HttpResponse::Created().json(web::Json(serde_json::json!({
        "id": order_id,
        "message": "Orden registrada correctamente"
    })))
}

pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/orders")
            .service(register_order)
    );
}