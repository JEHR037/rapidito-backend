use actix_web::{web, HttpResponse, Responder, post};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use sqlx::types::Uuid;
use crate::db::users::{create_user, get_user_by_email};
use crate::db::users::UserStatus;
use crate::db::users::UserRole;
use bcrypt::{hash, verify, DEFAULT_COST};
#[derive(Debug, Serialize, Deserialize)]
pub struct LoginRequest {
    pub email: String,
    pub password: String,
}


#[derive(Debug,Serialize, Deserialize)]
pub struct LoginResponse {
    pub token: String,
    pub user_id: Uuid,
    pub email: String,
    pub name: String,
    pub role: UserRole,
    pub status: UserStatus,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RegisterRequest {
    pub name: String,
    pub email: String,
    pub password: String,
    pub role: UserRole,
    pub status: UserStatus,
}

#[post("/login")]
async fn login(
    login_data: web::Json<LoginRequest>,
    pool: web::Data<PgPool>,
) -> impl Responder {
    match get_user_by_email(&pool, &login_data.email).await {
        Ok((user_id, email, password_hash, name, status, role)) => {
            if status == UserStatus::Inactive {
                return HttpResponse::Unauthorized().json(web::Json(serde_json::json!({
                    "error": "Usuario BANEADO o INACTIVO"
                })));
            }
            if status == UserStatus::Active && verify(&login_data.password, &password_hash).unwrap_or(false) {
                let response = LoginResponse {
                    token: format!("jwt-token-{}", Uuid::new_v4()),
                    user_id,
                    email,
                    name,
                    role,
                    status: UserStatus::Active,
                };
                return HttpResponse::Ok().json(response);
            }
        }
        Err(_) => {
            return HttpResponse::Unauthorized().json(web::Json(serde_json::json!({
                "error": "Credenciales inválidas"
            })));
        }
    }
    
    HttpResponse::Unauthorized().json(web::Json(serde_json::json!({
        "error": "Credenciales inválidas"
    })))
}

#[post("/admin/login")]
async fn admin_login(
    login_data: web::Json<LoginRequest>,
    pool: web::Data<PgPool>,
) -> impl Responder {
    match get_user_by_email(&pool, &login_data.email).await {
        Ok((user_id, email, password_hash, name, status, role)) => {
            if status == UserStatus::Inactive {
                return HttpResponse::Unauthorized().json(web::Json(serde_json::json!({
                    "error": "Usuario BANEADO o INACTIVO"
                })));
            }
            if login_data.password == password_hash && status == UserStatus::Active && role == UserRole::Admin {
                let response = LoginResponse {
                    token: format!("jwt-token-{}", Uuid::new_v4()),
                    user_id,
                    email,
                    name,
                    role,
                    status: UserStatus::Active,
                };
                return HttpResponse::Ok().json(response);
            }
        }
        Err(_) => {
            return HttpResponse::Unauthorized().json(web::Json(serde_json::json!({
                "error": "Credenciales inválidas"
            })));
        }
    }
    
    HttpResponse::Unauthorized().json(web::Json(serde_json::json!({
        "error": "Credenciales inválidas"
    })))
}

#[post("/register")]
async fn register(
    register_data: web::Json<RegisterRequest>,
    pool: web::Data<PgPool>,
) -> impl Responder {
    if !register_data.email.contains('@') {
        return HttpResponse::BadRequest().json(web::Json(serde_json::json!({
            "error": "El email no tiene un formato válido"
        })));
    }
    
    if register_data.password.len() < 6 {
        return HttpResponse::BadRequest().json(web::Json(serde_json::json!({
            "error": "La contraseña debe tener al menos 6 caracteres"
        })));
    }
    if register_data.password.contains(" ") {
        return HttpResponse::BadRequest().json(web::Json(serde_json::json!({
            "error": "La contraseña no puede contener espacios"
        })));
    }

    if register_data.name.len() < 3 {
        return HttpResponse::BadRequest().json(web::Json(serde_json::json!({
            "error": "El nombre debe tener al menos 3 caracteres"
        })));
    }
    if register_data.email.contains(" ") {
        return HttpResponse::BadRequest().json(web::Json(serde_json::json!({
            "error": "El email no puede contener espacios"
        })));
    }
    
    if register_data.email.len() < 3 {
        return HttpResponse::BadRequest().json(web::Json(serde_json::json!({
            "error": "El email debe tener al menos 3 caracteres"
        })));
    }
    if register_data.password.len() < 6 || !register_data.password.contains(|c: char| c.is_uppercase()) || !register_data.password.contains(|c: char| c.is_lowercase()) {
        return HttpResponse::BadRequest().json(web::Json(serde_json::json!({
            "error": "La contraseña debe tener al menos 6 caracteres, incluyendo al menos una mayúscula y una minúscula"
        })));
    }
    
    if register_data.name.trim().is_empty() {
        return HttpResponse::BadRequest().json(web::Json(serde_json::json!({
            "error": "El nombre no puede estar vacío"
        })));
    }
    
    match get_user_by_email(&pool, &register_data.email).await {
        Ok(_) => {
            return HttpResponse::BadRequest().json(web::Json(serde_json::json!({
                "error": "El email ya está registrado"
            })));
        }
        Err(_) => {
            let password_hash = hash(&register_data.password, DEFAULT_COST).unwrap();
            let user_id = create_user(&pool, &register_data.email, &register_data.name, register_data.role, register_data.status, &password_hash).await.unwrap();
            return HttpResponse::Created().json(web::Json(serde_json::json!({
                "id": user_id,
                "message": "Usuario registrado correctamente"
            })));
        }
    }
}

pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/auth")
            .service(login)
            .service(admin_login)
            .service(register)
    );
}