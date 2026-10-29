use std::sync::OnceLock;

use actix_web::{post, web, HttpResponse};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;

use crate::auth::issue_token;
use crate::config::AppConfig;
use crate::db::users::{self, User};
use crate::domain::UserStatus;
use crate::error::{ApiError, ApiResult};
use crate::validation;

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct RegisterInput {
    name: String,
    email: String,
    password: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct LoginInput {
    email: String,
    password: String,
}

#[derive(Debug, Serialize)]
struct AuthResponse {
    token: String,
    user: User,
}

static DUMMY_HASH: OnceLock<String> = OnceLock::new();

fn dummy_hash() -> String {
    DUMMY_HASH
        .get_or_init(|| bcrypt::hash(uuid::Uuid::new_v4().to_string(), bcrypt::DEFAULT_COST).unwrap_or_default())
        .clone()
}

#[post("/auth/register")]
async fn register(
    input: web::Json<RegisterInput>,
    pool: web::Data<PgPool>,
    config: web::Data<AppConfig>,
) -> ApiResult<HttpResponse> {
    let name = validation::text("name", &input.name, 2, 100)?;
    let email = validation::email(&input.email)?;
    let password = validation::password(&input.password)?.to_owned();

    if users::email_exists(&pool, &email).await? {
        return Err(ApiError::conflict("email is already registered"));
    }

    let password_hash = web::block(move || bcrypt::hash(password, bcrypt::DEFAULT_COST))
        .await
        .map_err(|error| ApiError::Internal(error.to_string()))??;
    let user = users::create(&pool, name, &email, &password_hash).await?;
    let token = issue_token(&config, user.id)?;

    Ok(HttpResponse::Created().json(AuthResponse { token, user }))
}

#[post("/auth/login")]
async fn login(
    input: web::Json<LoginInput>,
    pool: web::Data<PgPool>,
    config: web::Data<AppConfig>,
) -> ApiResult<HttpResponse> {
    let invalid = || ApiError::unauthorized("invalid email or password");
    let email = input.email.trim().to_lowercase();
    let credentials = users::find_credentials(&pool, &email).await?;

    let hash = credentials
        .as_ref()
        .map(|found| found.password_hash.clone())
        .unwrap_or_default();
    let password = input.password.clone();
    let matches = web::block(move || {
        let hash = if hash.is_empty() { dummy_hash() } else { hash };
        bcrypt::verify(password, &hash).unwrap_or(false)
    })
        .await
        .map_err(|error| ApiError::Internal(error.to_string()))?;

    let credentials = credentials.filter(|_| matches).ok_or_else(invalid)?;
    if credentials.status != UserStatus::Active {
        return Err(ApiError::unauthorized("account is not active"));
    }

    let user = users::find_by_id(pool.get_ref(), credentials.id).await?.ok_or_else(invalid)?;
    let token = issue_token(&config, user.id)?;
    Ok(HttpResponse::Ok().json(AuthResponse { token, user }))
}

pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.service(register).service(login);
}
