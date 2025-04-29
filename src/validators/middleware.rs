use actix_web::{ HttpResponse,HttpRequest};
use serde::{Deserialize, Serialize};
use sqlx::types::Uuid;
use crate::db::users::{UserStatus,UserRole};
use std::env;
use jsonwebtoken::{decode, DecodingKey, Validation};

#[derive(Debug, Serialize, Deserialize)]
pub struct LoginRequest {
    pub email: String,
    pub password: String,
}
#[derive(Debug, Serialize, Deserialize)]
pub struct Claims {
    pub sub: Uuid,          // ID del usuario
    pub exp: usize,        // Fecha de expiración
    pub iat: usize,        // Fecha de emisión
    pub email: String,     // Correo electrónico
    pub name: String,      // Nombre del usuario
    pub role: UserRole,    // Rol del usuario
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

#[derive(Serialize, Deserialize, Debug)]
pub struct Post {
    post_type: String,
    data: serde_json::Value,
}



// Función para validar el JWT
pub fn validate_token(token: &str) -> Result<Claims, jsonwebtoken::errors::Error> {
    let jwt_secret = env::var("JWT_SECRET").expect("JWT_SECRET no encontrada");
    let key = DecodingKey::from_secret(jwt_secret.as_bytes());
    let validation = Validation::default();
    
    let token_data = decode::<Claims>(token, &key, &validation)?;
    Ok(token_data.claims)
}

// Middleware para verificar el JWT
pub async fn verify_token(req: HttpRequest) -> Result<Claims, HttpResponse> {
    let auth_header = req.headers()
        .get("Authorization")
        .and_then(|h| h.to_str().ok());

    match auth_header {
        Some(auth_str) if auth_str.starts_with("Bearer ") => {
            let token = &auth_str[7..];
            match validate_token(token) {
                Ok(claims) => Ok(claims),
                Err(_) => Err(HttpResponse::Unauthorized().json("Token inválido"))
            }
        },
        _ => Err(HttpResponse::Unauthorized().json("Token no proporcionado"))
    }
}