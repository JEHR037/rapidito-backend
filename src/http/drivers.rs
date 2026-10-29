use actix_web::{delete, get, post, put, web, HttpResponse};
use serde::Deserialize;
use sqlx::PgPool;
use uuid::Uuid;

use crate::auth::AuthUser;
use crate::db::drivers::{self, DriverChanges, RemoveOutcome, VehicleFields};
use crate::db::users;
use crate::domain::{UserRole, UserStatus};
use crate::error::{ApiError, ApiResult};
use crate::validation;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct VehicleInput {
    make: String,
    model: String,
    year: i32,
    license_plate: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct DriverInput {
    user_id: Uuid,
    phone: String,
    photo: String,
    vehicle: VehicleInput,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct DriverUpdateInput {
    phone: Option<String>,
    photo: Option<String>,
    status: Option<UserStatus>,
    vehicle: Option<VehicleInput>,
}

fn vehicle_fields(input: &VehicleInput) -> ApiResult<VehicleFields<'_>> {
    let max_year = chrono::Datelike::year(&chrono::Utc::now()) + 1;
    if !(1991..=max_year).contains(&input.year) {
        return Err(ApiError::bad_request(format!("vehicle.year must be between 1991 and {max_year}")));
    }
    Ok(VehicleFields {
        make: validation::text("vehicle.make", &input.make, 1, 255)?,
        model: validation::text("vehicle.model", &input.model, 1, 255)?,
        year: input.year,
        license_plate: validation::text("vehicle.licensePlate", &input.license_plate, 2, 20)?,
    })
}

async fn respond_with_driver(pool: &PgPool, id: Uuid, created: bool) -> ApiResult<HttpResponse> {
    let driver = drivers::find_by_id(pool, id).await?.ok_or_else(|| ApiError::not_found("driver not found"))?;
    Ok(if created { HttpResponse::Created().json(driver) } else { HttpResponse::Ok().json(driver) })
}

#[get("/drivers")]
async fn list(auth: AuthUser, pool: web::Data<PgPool>) -> ApiResult<HttpResponse> {
    auth.require(&[UserRole::Admin])?;
    Ok(HttpResponse::Ok().json(drivers::list(&pool).await?))
}

#[post("/drivers")]
async fn create(auth: AuthUser, input: web::Json<DriverInput>, pool: web::Data<PgPool>) -> ApiResult<HttpResponse> {
    auth.require(&[UserRole::Admin])?;
    let phone = validation::phone(&input.phone)?;
    let photo = validation::text("photo", &input.photo, 1, 2048)?;
    let vehicle = vehicle_fields(&input.vehicle)?;

    let user = users::find_by_id(pool.get_ref(), input.user_id)
        .await?
        .ok_or_else(|| ApiError::bad_request("userId does not reference an existing user"))?;
    if user.status != UserStatus::Active {
        return Err(ApiError::bad_request("userId must reference an active user"));
    }
    if matches!(user.role, UserRole::Admin | UserRole::Partner) {
        return Err(ApiError::conflict("admins and partners cannot become drivers"));
    }
    if drivers::exists(&pool, user.id).await? {
        return Err(ApiError::conflict("user already has a driver profile"));
    }

    drivers::create(&pool, user.id, phone, photo, &vehicle).await?;
    respond_with_driver(&pool, user.id, true).await
}

#[put("/drivers/{id}")]
async fn update(
    auth: AuthUser,
    path: web::Path<Uuid>,
    input: web::Json<DriverUpdateInput>,
    pool: web::Data<PgPool>,
) -> ApiResult<HttpResponse> {
    auth.require(&[UserRole::Admin])?;
    let id = path.into_inner();
    if input.status == Some(UserStatus::Deleted) {
        return Err(ApiError::bad_request("status cannot be deleted; use DELETE instead"));
    }
    let changes = DriverChanges {
        phone: input.phone.as_deref().map(validation::phone).transpose()?,
        photo: input.photo.as_deref().map(|photo| validation::text("photo", photo, 1, 2048)).transpose()?,
        status: input.status,
        vehicle: input.vehicle.as_ref().map(vehicle_fields).transpose()?,
    };
    if !drivers::update(&pool, id, &changes).await? {
        return Err(ApiError::not_found("driver not found"));
    }
    respond_with_driver(&pool, id, false).await
}

#[delete("/drivers/{id}")]
async fn remove(auth: AuthUser, path: web::Path<Uuid>, pool: web::Data<PgPool>) -> ApiResult<HttpResponse> {
    auth.require(&[UserRole::Admin])?;
    match drivers::remove(&pool, path.into_inner()).await? {
        RemoveOutcome::Removed => Ok(HttpResponse::NoContent().finish()),
        RemoveOutcome::NotFound => Err(ApiError::not_found("driver not found")),
        RemoveOutcome::HasOpenOrders => Err(ApiError::conflict("driver has orders in progress")),
    }
}

pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.service(list).service(create).service(update).service(remove);
}
