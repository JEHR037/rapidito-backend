use actix_web::{post, web, HttpResponse, Responder};
use serde::{Deserialize, Serialize};
use sqlx::{Pool, Postgres};
use crate::db::vehicles::{create_vehicle};

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

pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/vehicles")
            .service(create_vehicle_handler)
    );
}
