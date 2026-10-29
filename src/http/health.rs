use actix_web::{get, web, HttpResponse};
use serde_json::json;
use sqlx::PgPool;

use crate::error::ApiResult;

#[get("/health")]
async fn health(pool: web::Data<PgPool>) -> ApiResult<HttpResponse> {
    sqlx::query("SELECT 1").execute(pool.get_ref()).await?;
    Ok(HttpResponse::Ok().json(json!({ "status": "ok" })))
}

pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.service(health);
}
