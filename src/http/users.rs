use actix_web::{delete, get, post, put, web, HttpResponse};
use serde::Deserialize;
use sqlx::PgPool;
use uuid::Uuid;

use crate::auth::AuthUser;
use crate::db::{addresses, drivers, users};
use crate::domain::{UserRole, UserStatus};
use crate::error::{ApiError, ApiResult};
use crate::validation;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct UpdateProfileInput {
    name: Option<String>,
    #[serde(default, deserialize_with = "nullable")]
    phone: Option<Option<String>>,
    #[serde(default, deserialize_with = "nullable")]
    avatar: Option<Option<String>>,
}

fn nullable<'de, D, T>(deserializer: D) -> Result<Option<Option<T>>, D::Error>
where
    D: serde::Deserializer<'de>,
    T: Deserialize<'de>,
{
    Option::<T>::deserialize(deserializer).map(Some)
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct AddressInput {
    label: String,
    street: String,
    city: String,
    state: String,
    postal_code: String,
    country: String,
    latitude: Option<f64>,
    longitude: Option<f64>,
    #[serde(default)]
    is_default: bool,
}

#[get("/users/me")]
async fn me(auth: AuthUser, pool: web::Data<PgPool>) -> ApiResult<HttpResponse> {
    let user = users::find_by_id(pool.get_ref(), auth.id).await?.ok_or_else(|| ApiError::not_found("user not found"))?;
    Ok(HttpResponse::Ok().json(user))
}

#[put("/users/me")]
async fn update_me(
    auth: AuthUser,
    input: web::Json<UpdateProfileInput>,
    pool: web::Data<PgPool>,
) -> ApiResult<HttpResponse> {
    let name = input.name.as_deref().map(|name| validation::text("name", name, 2, 100)).transpose()?;
    let phone = match &input.phone {
        None => None,
        Some(None) => Some(None),
        Some(Some(raw)) if raw.trim().is_empty() => Some(None),
        Some(Some(raw)) => Some(Some(validation::phone(raw)?)),
    };
    let avatar = match &input.avatar {
        None => None,
        Some(value) => Some(validation::optional_text("avatar", value.as_deref(), 2048)?),
    };

    let changes = users::ProfileChanges { name, phone, avatar };
    let user = users::update_profile(&pool, auth.id, &changes)
        .await?
        .ok_or_else(|| ApiError::not_found("user not found"))?;
    Ok(HttpResponse::Ok().json(user))
}

#[delete("/users/me")]
async fn delete_me(auth: AuthUser, pool: web::Data<PgPool>) -> ApiResult<HttpResponse> {
    users::update_role_and_status(pool.get_ref(), auth.id, None, Some(UserStatus::Deleted))
        .await?
        .ok_or_else(|| ApiError::not_found("user not found"))?;
    Ok(HttpResponse::NoContent().finish())
}

#[get("/users/me/addresses")]
async fn list_addresses(auth: AuthUser, pool: web::Data<PgPool>) -> ApiResult<HttpResponse> {
    Ok(HttpResponse::Ok().json(addresses::list_by_user(&pool, auth.id).await?))
}

#[post("/users/me/addresses")]
async fn create_address(
    auth: AuthUser,
    input: web::Json<AddressInput>,
    pool: web::Data<PgPool>,
) -> ApiResult<HttpResponse> {
    validation::coordinates(input.latitude, input.longitude)?;
    let new_address = addresses::NewAddress {
        label: validation::text("label", &input.label, 1, 50)?,
        street: validation::text("street", &input.street, 3, 255)?,
        city: validation::text("city", &input.city, 2, 100)?,
        state: validation::text("state", &input.state, 2, 100)?,
        postal_code: validation::text("postalCode", &input.postal_code, 3, 10)?,
        country: validation::text("country", &input.country, 2, 100)?,
        latitude: input.latitude,
        longitude: input.longitude,
        is_default: input.is_default,
    };
    let address = addresses::create(&pool, auth.id, &new_address).await?;
    Ok(HttpResponse::Created().json(address))
}

#[put("/users/me/addresses/{id}/default")]
async fn set_default_address(
    auth: AuthUser,
    path: web::Path<Uuid>,
    pool: web::Data<PgPool>,
) -> ApiResult<HttpResponse> {
    let address = addresses::set_default(&pool, auth.id, path.into_inner())
        .await?
        .ok_or_else(|| ApiError::not_found("address not found"))?;
    Ok(HttpResponse::Ok().json(address))
}

#[delete("/users/me/addresses/{id}")]
async fn delete_address(auth: AuthUser, path: web::Path<Uuid>, pool: web::Data<PgPool>) -> ApiResult<HttpResponse> {
    if !addresses::delete(&pool, auth.id, path.into_inner()).await? {
        return Err(ApiError::not_found("address not found"));
    }
    Ok(HttpResponse::NoContent().finish())
}

#[get("/users/me/driver")]
async fn my_driver_profile(auth: AuthUser, pool: web::Data<PgPool>) -> ApiResult<HttpResponse> {
    auth.require(&[UserRole::Driver])?;
    let driver = drivers::find_by_id(&pool, auth.id)
        .await?
        .ok_or_else(|| ApiError::not_found("driver profile not found"))?;
    Ok(HttpResponse::Ok().json(driver))
}

pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.service(me)
        .service(update_me)
        .service(delete_me)
        .service(list_addresses)
        .service(create_address)
        .service(set_default_address)
        .service(delete_address)
        .service(my_driver_profile);
}
