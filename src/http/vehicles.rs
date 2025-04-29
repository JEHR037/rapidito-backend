use actix_web::{post, web, put, delete, get, HttpResponse, Responder};
use serde::{Deserialize, Serialize};
use sqlx::{Pool, Postgres};
use crate::db::vehicles::{create_vehicle, update_vehicle, delete_vehicle, get_vehicle_by_id};
use uuid::Uuid;

#[derive(Debug, Serialize, Deserialize)]
pub struct CreateVehicleRequest {
    pub make: String,
    pub model: String,
    pub year: i32,
    pub license_plate: String,
}

#[post("/create")]
async fn create_vehicle_handler(
    vehicle_data: web::Json<CreateVehicleRequest>,
    pool: web::Data<Pool<Postgres>>,
) -> impl Responder {
    let make = &vehicle_data.make.clone();
    let model = &vehicle_data.model.clone();
    let year = vehicle_data.year;
    let license_plate = &vehicle_data.license_plate.clone();

    let vehicle_id = create_vehicle(&pool, &make, &model, year, &license_plate).await.unwrap();

    HttpResponse::Created().json(web::Json(serde_json::json!({
        "message": "Vehiculo creado correctamente",
        "vehicle_id": vehicle_id
    })))
}

#[put("/update/{id}")]
async fn update_vehicle_handler(
    vehicle_id: web::Path<Uuid>,
    vehicle_data: web::Json<CreateVehicleRequest>,
    pool: web::Data<Pool<Postgres>>,
) -> impl Responder {
    let result = update_vehicle(
        &pool,
        vehicle_id.into_inner(),
        Some(&vehicle_data.make),
        Some(&vehicle_data.model),
        Some(vehicle_data.year),
        Some(&vehicle_data.license_plate),
    ).await;

    match result {
        Ok(_) => HttpResponse::Ok().json(web::Json(serde_json::json!({
            "message": "Vehículo actualizado correctamente"
        }))),
        Err(_) => HttpResponse::InternalServerError().json("Error al actualizar el vehículo"),
    }
}

#[delete("/delete/{id}")]
async fn delete_vehicle_handler(
    vehicle_id: web::Path<Uuid>,
    pool: web::Data<Pool<Postgres>>,
) -> impl Responder {
    let result = delete_vehicle(&pool, vehicle_id.into_inner()).await;

    match result {
        Ok(_) => HttpResponse::Ok().json(web::Json(serde_json::json!({
            "message": "Vehículo eliminado correctamente"
        }))),
        Err(_) => HttpResponse::InternalServerError().json("Error al eliminar el vehículo"),
    }
}

#[get("/{id}")]
async fn get_vehicle_info(
    vehicle_id: web::Path<Uuid>,
    pool: web::Data<Pool<Postgres>>,
) -> impl Responder {
    match get_vehicle_by_id(&pool, vehicle_id.into_inner()).await {
        Ok(vehicle) => HttpResponse::Ok().json(vehicle),
        Err(_) => HttpResponse::NotFound().json(web::Json(serde_json::json!({
            "error": "Vehículo no encontrado"
        }))),
    }
}

pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/vehicles")
            .service(create_vehicle_handler)
            .service(update_vehicle_handler)
            .service(delete_vehicle_handler)
            .service(get_vehicle_info)
    );
}
