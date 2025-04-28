use actix_web::{post, web, HttpResponse, Responder};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use sqlx::{Pool, Postgres};
use crate::db::drivers::{create_driver_from_user};
use crate::db::users::{get_user_by_id};
use crate::db::vehicles::create_vehicle;


#[derive(Debug, Serialize, Deserialize)]
pub struct CreateDriverRequest {
    pub user_id: Uuid,
    pub vehicle: Option<Uuid>,
    pub make: Option<String>,
    pub model: Option<String>,
    pub year: Option<i32>,
    pub license_plate: String,
    pub photo: String,
    pub phone: String,
    pub rating: f32,
}


#[post("/create")]
async fn create_driver(
    driver_data: web::Json<CreateDriverRequest>,
    pool: web::Data<Pool<Postgres>>,
) -> impl Responder {
    // Validaciones
    if driver_data.license_plate.trim().is_empty() {
        return HttpResponse::BadRequest().json(web::Json(serde_json::json!({
            "error": "La placa del vehículo no puede estar vacía"
        })));
    }
    if driver_data.photo.trim().is_empty() {
        return HttpResponse::BadRequest().json(web::Json(serde_json::json!({
            "error": "La foto no puede estar vacía"
        })));
    }
    if driver_data.phone.trim().is_empty() {
        return HttpResponse::BadRequest().json(web::Json(serde_json::json!({
            "error": "El número de teléfono no puede estar vacío"
        })));
    }

    let phone = &driver_data.phone;  
    let photo = &driver_data.photo;
    let user_id = driver_data.user_id;
    let license_plate = &driver_data.license_plate;

    let rating = if driver_data.rating > 0.0 {
        driver_data.rating
    } else {
        0.5
    };

    let (email, name, status) = match get_user_by_id(&pool, user_id).await {
        Ok(user_data) => user_data,
        Err(_) => return HttpResponse::BadRequest().json("Usuario no encontrado"),
    };

    let vehicle_id = if let Some(vehicle) = driver_data.vehicle {
        vehicle
    } else {
        create_vehicle(
            &pool,
            driver_data.make.as_deref().unwrap_or(""),
            driver_data.model.as_deref().unwrap_or(""),
            driver_data.year.unwrap_or(2020),
            license_plate
        ).await.unwrap()
    };

    create_driver_from_user(&pool, user_id, vehicle_id, license_plate, &name, &photo, &phone, rating , &email, status).await.unwrap();

    HttpResponse::Created().json(web::Json(serde_json::json!({
        "message": "Conductor creado correctamente",
        "name": name,
        "email": email,
        "status": status,
    })))
}




pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/drivers")
            .service(create_driver)
    );
}

