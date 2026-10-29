use actix_web::dev::Payload;
use actix_web::{web, FromRequest, HttpRequest};
use chrono::{Duration, Utc};
use futures_util::future::LocalBoxFuture;
use sqlx::PgPool;
use jsonwebtoken::{decode, encode, Algorithm, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::config::AppConfig;
use crate::db;
use crate::domain::{UserRole, UserStatus};
use crate::error::{ApiError, ApiResult};

#[derive(Debug, Serialize, Deserialize)]
struct Claims {
    sub: Uuid,
    iat: i64,
    exp: i64,
}

#[derive(Debug, Clone, Copy)]
pub struct AuthUser {
    pub id: Uuid,
    pub role: UserRole,
}

impl AuthUser {
    pub fn require(&self, roles: &[UserRole]) -> ApiResult<()> {
        if roles.contains(&self.role) {
            Ok(())
        } else {
            Err(ApiError::forbidden("Your role cannot perform this action"))
        }
    }

    pub fn is(&self, role: UserRole) -> bool {
        self.role == role
    }
}

pub fn issue_token(config: &AppConfig, user_id: Uuid) -> ApiResult<String> {
    let now = Utc::now();
    let claims = Claims {
        sub: user_id,
        iat: now.timestamp(),
        exp: (now + Duration::hours(config.jwt_ttl_hours)).timestamp(),
    };
    Ok(encode(
        &Header::new(Algorithm::HS256),
        &claims,
        &EncodingKey::from_secret(config.jwt_secret.as_bytes()),
    )?)
}

fn bearer_subject(request: &HttpRequest, config: &AppConfig) -> ApiResult<Uuid> {
    let token = request
        .headers()
        .get(actix_web::http::header::AUTHORIZATION)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.strip_prefix("Bearer "))
        .map(str::trim)
        .filter(|token| !token.is_empty())
        .ok_or_else(|| ApiError::unauthorized("Missing bearer token"))?;

    let data = decode::<Claims>(
        token,
        &DecodingKey::from_secret(config.jwt_secret.as_bytes()),
        &Validation::new(Algorithm::HS256),
    )
    .map_err(|_| ApiError::unauthorized("Invalid or expired token"))?;

    Ok(data.claims.sub)
}

impl FromRequest for AuthUser {
    type Error = ApiError;
    type Future = LocalBoxFuture<'static, Result<Self, Self::Error>>;

    fn from_request(request: &HttpRequest, _payload: &mut Payload) -> Self::Future {
        let config = request.app_data::<web::Data<AppConfig>>().cloned();
        let pool = request.app_data::<web::Data<PgPool>>().cloned();
        let subject = config
            .as_ref()
            .ok_or_else(|| ApiError::Internal("AppConfig is not registered".to_string()))
            .and_then(|config| bearer_subject(request, config));

        Box::pin(async move {
            let user_id = subject?;
            let pool = pool.ok_or_else(|| ApiError::Internal("PgPool is not registered".to_string()))?;
            let (role, status) = db::users::find_role_and_status(&pool, user_id)
                .await?
                .ok_or_else(|| ApiError::unauthorized("Invalid or expired token"))?;
            if status != UserStatus::Active {
                return Err(ApiError::unauthorized("Account is not active"));
            }
            Ok(AuthUser { id: user_id, role })
        })
    }
}
