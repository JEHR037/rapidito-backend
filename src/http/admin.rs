use actix_web::{get, put, web, HttpResponse};
use serde::Deserialize;
use sqlx::PgPool;
use uuid::Uuid;

use crate::auth::AuthUser;
use crate::db::{stats, users};
use crate::domain::{UserRole, UserStatus};
use crate::error::{ApiError, ApiResult};

#[derive(Debug, Deserialize)]
struct UserQuery {
    role: Option<UserRole>,
    q: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct UserAdminInput {
    role: Option<UserRole>,
    status: Option<UserStatus>,
}

#[get("/admin/stats")]
async fn admin_stats(auth: AuthUser, pool: web::Data<PgPool>) -> ApiResult<HttpResponse> {
    auth.require(&[UserRole::Admin])?;
    Ok(HttpResponse::Ok().json(stats::admin_stats(&pool).await?))
}

#[get("/admin/users")]
async fn list_users(auth: AuthUser, query: web::Query<UserQuery>, pool: web::Data<PgPool>) -> ApiResult<HttpResponse> {
    auth.require(&[UserRole::Admin])?;
    let search = query.q.as_deref().map(str::trim).filter(|value| !value.is_empty());
    Ok(HttpResponse::Ok().json(users::list(&pool, query.role, search).await?))
}

#[put("/admin/users/{id}")]
async fn update_user(
    auth: AuthUser,
    path: web::Path<Uuid>,
    input: web::Json<UserAdminInput>,
    pool: web::Data<PgPool>,
) -> ApiResult<HttpResponse> {
    auth.require(&[UserRole::Admin])?;
    let id = path.into_inner();
    if id == auth.id {
        return Err(ApiError::conflict("admins cannot change their own role or status"));
    }
    if input.role == Some(UserRole::Driver) {
        return Err(ApiError::bad_request("use POST /api/drivers to create a driver"));
    }
    let current = users::find_by_id(pool.get_ref(), id).await?.ok_or_else(|| ApiError::not_found("user not found"))?;
    if current.role == UserRole::Driver && input.role.is_some_and(|role| role != UserRole::Driver) {
        return Err(ApiError::bad_request("use DELETE /api/drivers/{id} to remove a driver"));
    }
    let user = users::update_role_and_status(pool.get_ref(), id, input.role, input.status)
        .await?
        .ok_or_else(|| ApiError::not_found("user not found"))?;
    Ok(HttpResponse::Ok().json(user))
}

pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.service(admin_stats).service(list_users).service(update_user);
}
