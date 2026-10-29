use serde::Serialize;
use sqlx::{FromRow, PgPool};
use uuid::Uuid;

use crate::domain::{UserRole, UserStatus};

#[derive(Debug, Serialize, FromRow)]
#[serde(rename_all = "camelCase")]
pub struct Vehicle {
    pub id: Uuid,
    pub make: String,
    pub model: String,
    pub year: i32,
    pub license_plate: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Driver {
    pub id: Uuid,
    pub name: String,
    pub email: String,
    pub phone: Option<String>,
    pub photo: String,
    pub rating: f32,
    pub deliveries: i32,
    pub status: UserStatus,
    pub vehicle: Option<Vehicle>,
}

#[derive(Debug, FromRow)]
struct DriverRow {
    id: Uuid,
    name: String,
    email: String,
    phone: Option<String>,
    photo: String,
    rating: f32,
    deliveries: i32,
    status: UserStatus,
    vehicle_id: Option<Uuid>,
    vehicle_make: Option<String>,
    vehicle_model: Option<String>,
    vehicle_year: Option<i32>,
    vehicle_license_plate: Option<String>,
}

impl From<DriverRow> for Driver {
    fn from(row: DriverRow) -> Self {
        let vehicle = match (row.vehicle_id, row.vehicle_make, row.vehicle_model, row.vehicle_year, row.vehicle_license_plate) {
            (Some(id), Some(make), Some(model), Some(year), Some(license_plate)) => {
                Some(Vehicle { id, make, model, year, license_plate })
            }
            _ => None,
        };
        Self {
            id: row.id,
            name: row.name,
            email: row.email,
            phone: row.phone,
            photo: row.photo,
            rating: row.rating,
            deliveries: row.deliveries,
            status: row.status,
            vehicle,
        }
    }
}

pub struct VehicleFields<'a> {
    pub make: &'a str,
    pub model: &'a str,
    pub year: i32,
    pub license_plate: &'a str,
}

pub struct DriverChanges<'a> {
    pub phone: Option<&'a str>,
    pub photo: Option<&'a str>,
    pub status: Option<UserStatus>,
    pub vehicle: Option<VehicleFields<'a>>,
}

const DRIVER_SELECT: &str = "SELECT d.id, u.name, u.email, d.phone, d.photo, d.rating, d.deliveries, d.status, \
         v.id AS vehicle_id, v.make AS vehicle_make, v.model AS vehicle_model, v.year AS vehicle_year, \
         v.license_plate AS vehicle_license_plate \
     FROM drivers d \
     JOIN users u ON u.id = d.id \
     LEFT JOIN vehicles v ON v.id = d.vehicle";

pub async fn list(pool: &PgPool) -> sqlx::Result<Vec<Driver>> {
    let rows = sqlx::query_as::<_, DriverRow>(&format!("{DRIVER_SELECT} ORDER BY u.name"))
        .fetch_all(pool)
        .await?;
    Ok(rows.into_iter().map(Driver::from).collect())
}

pub async fn find_by_id(pool: &PgPool, id: Uuid) -> sqlx::Result<Option<Driver>> {
    let row = sqlx::query_as::<_, DriverRow>(&format!("{DRIVER_SELECT} WHERE d.id = $1"))
        .bind(id)
        .fetch_optional(pool)
        .await?;
    Ok(row.map(Driver::from))
}

pub async fn exists(pool: &PgPool, id: Uuid) -> sqlx::Result<bool> {
    sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM drivers WHERE id = $1)")
        .bind(id)
        .fetch_one(pool)
        .await
}

pub async fn create(
    pool: &PgPool,
    user_id: Uuid,
    phone: &str,
    photo: &str,
    vehicle: &VehicleFields<'_>,
) -> sqlx::Result<bool> {
    let mut tx = pool.begin().await?;

    let promoted = crate::db::users::update_role_and_status(&mut *tx, user_id, Some(UserRole::Driver), None).await?;
    if promoted.is_none() {
        return Ok(false);
    }

    let vehicle_id: Uuid = sqlx::query_scalar(
        "INSERT INTO vehicles (make, model, year, license_plate, user_id) VALUES ($1, $2, $3, $4, $5) RETURNING id",
    )
    .bind(vehicle.make)
    .bind(vehicle.model)
    .bind(vehicle.year)
    .bind(vehicle.license_plate)
    .bind(user_id)
    .fetch_one(&mut *tx)
    .await?;

    sqlx::query(
        "INSERT INTO drivers (id, vehicle, photo, phone, rating, deliveries, status) \
         VALUES ($1, $2, $3, $4, 5.0, 0, 'Active')",
    )
    .bind(user_id)
    .bind(vehicle_id)
    .bind(photo)
    .bind(phone)
    .execute(&mut *tx)
    .await?;

    tx.commit().await?;
    Ok(true)
}

pub async fn update(pool: &PgPool, id: Uuid, changes: &DriverChanges<'_>) -> sqlx::Result<bool> {
    let mut tx = pool.begin().await?;

    let vehicle_id: Option<Option<Uuid>> = sqlx::query_scalar(
        "UPDATE drivers SET phone = COALESCE($2, phone), photo = COALESCE($3, photo), \
             status = COALESCE($4, status), updated_at = CURRENT_TIMESTAMP \
         WHERE id = $1 RETURNING vehicle",
    )
    .bind(id)
    .bind(changes.phone)
    .bind(changes.photo)
    .bind(changes.status)
    .fetch_optional(&mut *tx)
    .await?;
    let Some(vehicle_id) = vehicle_id else {
        return Ok(false);
    };

    if let Some(vehicle) = &changes.vehicle {
        match vehicle_id {
            Some(vehicle_id) => {
                sqlx::query(
                    "UPDATE vehicles SET make = $2, model = $3, year = $4, license_plate = $5, \
                         updated_at = CURRENT_TIMESTAMP WHERE id = $1",
                )
                .bind(vehicle_id)
                .bind(vehicle.make)
                .bind(vehicle.model)
                .bind(vehicle.year)
                .bind(vehicle.license_plate)
                .execute(&mut *tx)
                .await?;
            }
            None => {
                let new_id: Uuid = sqlx::query_scalar(
                    "INSERT INTO vehicles (make, model, year, license_plate, user_id) VALUES ($1, $2, $3, $4, $5) RETURNING id",
                )
                .bind(vehicle.make)
                .bind(vehicle.model)
                .bind(vehicle.year)
                .bind(vehicle.license_plate)
                .bind(id)
                .fetch_one(&mut *tx)
                .await?;
                sqlx::query("UPDATE drivers SET vehicle = $2 WHERE id = $1")
                    .bind(id)
                    .bind(new_id)
                    .execute(&mut *tx)
                    .await?;
            }
        }
    }

    tx.commit().await?;
    Ok(true)
}

#[derive(Debug, PartialEq, Eq)]
pub enum RemoveOutcome {
    Removed,
    NotFound,
    HasOpenOrders,
}

pub async fn remove(pool: &PgPool, id: Uuid) -> sqlx::Result<RemoveOutcome> {
    let mut tx = pool.begin().await?;

    let vehicle_id: Option<Option<Uuid>> = sqlx::query_scalar("SELECT vehicle FROM drivers WHERE id = $1 FOR UPDATE")
        .bind(id)
        .fetch_optional(&mut *tx)
        .await?;
    let Some(vehicle_id) = vehicle_id else {
        return Ok(RemoveOutcome::NotFound);
    };

    let (open_orders, total_orders): (i64, i64) = sqlx::query_as(
        "SELECT COUNT(*) FILTER (WHERE status NOT IN ('Delivered', 'Cancelled')), COUNT(*) FROM orders WHERE driver_id = $1",
    )
    .bind(id)
    .fetch_one(&mut *tx)
    .await?;
    if open_orders > 0 {
        return Ok(RemoveOutcome::HasOpenOrders);
    }

    if total_orders > 0 {
        sqlx::query("UPDATE drivers SET status = 'Inactive', updated_at = CURRENT_TIMESTAMP WHERE id = $1")
            .bind(id)
            .execute(&mut *tx)
            .await?;
    } else {
        sqlx::query("DELETE FROM drivers WHERE id = $1").bind(id).execute(&mut *tx).await?;
        if let Some(vehicle_id) = vehicle_id {
            sqlx::query("DELETE FROM vehicles WHERE id = $1").bind(vehicle_id).execute(&mut *tx).await?;
        }
    }

    crate::db::users::update_role_and_status(&mut *tx, id, Some(UserRole::Customer), None).await?;

    tx.commit().await?;
    Ok(RemoveOutcome::Removed)
}
