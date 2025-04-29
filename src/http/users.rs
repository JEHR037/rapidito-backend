use crate::db::users::{db_get_user_by_id,db_update_user,db_delete_user, UserStatus};
use crate::db::drivers::{get_driver_by_user_id, create_driver_from_user};
use actix_web::{web, post, put, delete, HttpResponse, Responder, get, HttpRequest};
use crate::db::vehicles::{create_vehicle, get_vehicles_by_user_id};
use crate::validators::middleware::{verify_token};
use crate::http::drivers::CreateDriverRequest;
use serde::{Deserialize, Serialize};
use sqlx::types::Uuid;
use dotenv::dotenv;
use sqlx::PgPool;

#[derive(Debug, Serialize, Deserialize)]
pub struct UpdateUserRequest {
    pub user_id: Option<Uuid>,
    pub vehicle: Option<Option<Uuid>>,
    pub email: Option<String>,
    pub name: Option<String>,
    pub status: Option<UserStatus>,
    pub make: Option<String>,
    pub model: Option<String>,
    pub year: Option<i32>,
    pub license_plate: Option<String>,
    pub photo: Option<String>,
    pub phone: Option<String>,
    pub rating: Option<f32>,
}


#[get("/me")]
async fn get_user_info(
    req: HttpRequest,
    pool: web::Data<PgPool>,
) -> impl Responder {
    match verify_token(req).await {
        Ok(claims) => {
            let user_id = claims.sub;

            // Obtener información del usuario
            let user_info = match db_get_user_by_id(&pool, user_id).await {
                Ok((email, name, status)) => {
                    // Aquí puedes agregar más información si es necesario
                    (email, name, status)
                },
                Err(_) => return HttpResponse::NotFound().json(web::Json(serde_json::json!({
                    "error": "Usuario no encontrado"
                }))),
            };

            // Obtener información del conductor
            let driver_info = match get_driver_by_user_id(&pool, user_id).await {
                Ok(profile) => {
                    Some(profile)
                },
                Err(_) => None, // Si no se encuentra información del conductor
            };

            // Construir la respuesta
            let response = serde_json::json!({
                "user": {
                    "email": user_info.0,
                    "name": user_info.1,
                    "status": user_info.2,
                },
                "driver": driver_info.map(|profile| serde_json::json!({
                    "profile": profile,
                })),
            });

            HttpResponse::Ok().json(response)
        },
        Err(err_response) => err_response,
    }
}

#[put("/me/update")]
async fn update_user(
    user_data: web::Json<UpdateUserRequest>,
    req: HttpRequest,
    pool: web::Data<PgPool>,
) -> impl Responder {
    match verify_token(req).await {
        Ok(data) => {
            let user_id = data.sub;

            let result = db_update_user(
                &pool,
                user_id,
                user_data.email.as_deref(),
                user_data.name.as_deref(),
                None,
                user_data.vehicle,
                user_data.make.as_deref(),
                user_data.model.as_deref(),
                user_data.year,
                user_data.license_plate.as_deref(),
                user_data.photo.as_deref(),
                user_data.phone.as_deref(),
                user_data.rating,
            ).await;

            match result {
                Ok(_) => HttpResponse::Ok().json(web::Json(serde_json::json!({
                    "message": "Usuario actualizado correctamente"
                }))),
                Err(_) => HttpResponse::InternalServerError().json(web::Json(serde_json::json!({
                    "error": "Error al actualizar el usuario"
                }))),
            }
        },
        Err(_) => HttpResponse::Unauthorized().json(web::Json(serde_json::json!({
            "error": "Token inválido"
        }))),
    }
}

#[get("/me/driver")]
async fn get_driver_info(
    req: HttpRequest,
    pool: web::Data<PgPool>,
) -> impl Responder {
    match verify_token(req).await {
        Ok(claims) => {
            let user_id = claims.sub; 
            match get_driver_by_user_id(&pool, user_id).await {
                Ok(profile) => HttpResponse::Ok().json(web::Json(serde_json::json!({
                    "profile": profile,
                }))),
                Err(_) => HttpResponse::NotFound().json(web::Json(serde_json::json!({
                    "error": "No se encontró información del conductor"
                }))),
            }
        },
        Err(err_response) => err_response, 
    }
}

#[delete("/me/delete")]
async fn delete_user(
    req: HttpRequest,
    pool: web::Data<PgPool>,
) -> impl Responder {
    match verify_token(req).await {
        Ok(data) => {
            let user_id = data.sub; // Extraemos el user_id del token

            let result = db_delete_user(&pool, user_id).await;

            match result {
                Ok(_) => HttpResponse::Ok().json(web::Json(serde_json::json!({
                    "message": "Usuario eliminado correctamente"
                }))),
                Err(_) => HttpResponse::InternalServerError().json(web::Json(serde_json::json!({
                    "error": "Error al eliminar el usuario"
                }))),
            }
        },
        Err(_) => HttpResponse::Unauthorized().json(web::Json(serde_json::json!({
            "error": "Token inválido"
        }))),
    }
}


#[get("/me/vehicles")]
async fn get_vehicles(
    req: HttpRequest,
    pool: web::Data<PgPool>,
) -> impl Responder {
    match verify_token(req).await {
        Ok(data) => {
            let user_id = data.sub; 

            match get_vehicles_by_user_id(&pool, user_id).await {
                Ok(vehicles) => HttpResponse::Ok().json(web::Json(vehicles)),
                Err(_) => HttpResponse::NotFound().json(web::Json(serde_json::json!({
                    "error": "No se encontraron vehículos para este usuario"
                }))),
            }
        },
        Err(_) => HttpResponse::Unauthorized().json(web::Json(serde_json::json!({
            "error": "Token inválido"
        }))),
    }
}


#[post("/make/driver")]
async fn create_driver(
    driver_data: web::Json<CreateDriverRequest>,
    req: HttpRequest,
    pool: web::Data<PgPool>,
) -> impl Responder {
    dotenv().ok();

    // Verificar el token usando el middleware
    match verify_token(req).await {
        Ok(data) => {
            let user_id = data.sub;

            // Obtener información del usuario
            let user_info = match db_get_user_by_id(&pool, user_id).await {
                Ok((email, name, status)) => (email, name, status),
                Err(_) => return HttpResponse::NotFound().json(web::Json(serde_json::json!({
                    "error": "Usuario no encontrado"
                }))),
            };

            // Obtener vehículos del usuario
            let vehicles_result = get_vehicles_by_user_id(&pool, user_id).await;

            let vehicle_id = match vehicles_result {
                Ok(vehicles) if !vehicles.is_empty() => vehicles[0].id, // Usar el primer vehículo
                _ => {
                    // Si no hay vehículos, crear uno nuevo
                    create_vehicle(&pool, &driver_data.make, &driver_data.model, driver_data.year, &driver_data.license_plate, &user_id).await.unwrap()
                }
            };

            match create_driver_from_user(
                &pool,
                user_id,
                vehicle_id,
                &driver_data.license_plate,
                &user_info.1, // Nombre del usuario
                &driver_data.photo,
                &driver_data.phone,
                driver_data.rating,
                &user_info.0, // Email del usuario
                UserStatus::Active, 
            ).await {
                Ok(_) => HttpResponse::Created().json(web::Json(serde_json::json!({
                    "message": "Conductor creado correctamente"
                }))),
                Err(_) => HttpResponse::InternalServerError().json(web::Json(serde_json::json!({
                    "error": "Error al crear el conductor"
                }))),
            }
        },
        Err(_) => HttpResponse::Unauthorized().json(web::Json(serde_json::json!({
            "error": "Token inválido"
        }))),
    }
}

pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/users")
            .service(update_user)
            .service(delete_user)
            .service(get_vehicles)
            .service(get_driver_info)
            .service(create_driver)
            .service(get_user_info)
    );
}