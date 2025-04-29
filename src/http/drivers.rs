use actix_web::{post, web, put, delete, HttpResponse, Responder};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use sqlx::{Pool, Postgres};
use crate::db::drivers::create_driver_from_user;
use crate::db::drivers::update_driver as import_update; 
use crate::db::drivers::delete_driver as import_delete;
use crate::db::users::db_get_user_by_id;
use crate::db::vehicles::create_vehicle;


#[derive(Debug, Serialize, Deserialize)]
pub struct CreateDriverRequest {
    pub user_id: Uuid,
    pub vehicle: Option<Uuid>,
    pub make: String,
    pub model: String,
    pub year: i32,
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
 
    let user_id = driver_data.user_id;
    let license_plate = &driver_data.license_plate;

    let rating = if driver_data.rating > 0.0 {
        driver_data.rating
    } else {
        0.5
    };

    let (email, name, status) = match db_get_user_by_id(&pool, user_id).await {
        Ok(user_data) => user_data,
        Err(_) => return HttpResponse::BadRequest().json("Usuario no encontrado"),
    };

    let vehicle_id = match driver_data.vehicle {
        Some(vehicle) => vehicle,
        None => {
            create_vehicle(
                &pool,
                &driver_data.make,
                &driver_data.model,
                if driver_data.year > 0 { driver_data.year } else { 2020 },
                &license_plate
            ).await.unwrap()
        }
    };

    create_driver_from_user(&pool, user_id, vehicle_id, license_plate, &name, &driver_data.photo, &driver_data.phone, rating , &email, status).await.unwrap();

    HttpResponse::Created().json(web::Json(serde_json::json!({
        "message": "Conductor creado correctamente",
        "name": name,
        "email": email,
        "status": status,
    })))
}

#[put("/update/{id}")]
async fn update_driver(
    driver_id: web::Path<Uuid>,
    driver_data: web::Json<CreateDriverRequest>,
    pool: web::Data<Pool<Postgres>>,
) -> impl Responder {
    let result = import_update(
        &pool,
        driver_id.into_inner(),
        Some(&driver_data.make),
        Some(&driver_data.photo),
        Some(&driver_data.phone),
        Some(driver_data.rating),
    ).await;

    match result {
        Ok(_) => HttpResponse::Ok().json(web::Json(serde_json::json!({
            "message": "Conductor actualizado correctamente"
        }))),
        Err(_) => HttpResponse::InternalServerError().json("Error al actualizar el conductor"),
    }
}

#[delete("/delete/{id}")]
async fn delete_driver(
    driver_id: web::Path<Uuid>,
    pool: web::Data<Pool<Postgres>>,
) -> impl Responder {
    let result = import_delete(&pool, driver_id.into_inner()).await;

    match result {
        Ok(_) => HttpResponse::Ok().json(web::Json(serde_json::json!({
            "message": "Conductor eliminado correctamente"
        }))),
        Err(_) => HttpResponse::InternalServerError().json("Error al eliminar el conductor"),
    }
}

pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/drivers")
            .service(create_driver)
            .service(update_driver)
            .service(delete_driver)
    );
}

