use sqlx::{query, Pool, Postgres};
use sqlx::types::Uuid;
use serde::{Serialize, Deserialize};
use crate::db::users::{UserStatus};
use sqlx::PgPool;


#[derive(Debug, Serialize, Deserialize)]
pub struct Vehicle {
    pub id: Uuid,
    pub make: String,
    pub model: String,
    pub year: i32,
    pub license_plate: String,
}

pub async fn create_vehicle(pool: &Pool<Postgres>, make: &str, model: &str, year: i32, license_plate: &str) -> Result<Uuid, sqlx::Error> {
    let vehicle_id = Uuid::new_v4();
    query("INSERT INTO vehicles (id, make, model, year, license_plate, created_at, updated_at) VALUES ($1, $2, $3, $4, $5, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP)")
        .bind(vehicle_id)
        .bind(make)
        .bind(model)
        .bind(year)
        .bind(license_plate)
        .execute(pool)
        .await?;
    Ok(vehicle_id)
}

pub async fn create_driver_from_user(pool: &PgPool, user_id: Uuid, vehicle_id: Uuid, license_plate: &str) -> Result<(), sqlx::Error> {
    query("INSERT INTO drivers (id, vehicle, license_plate, status, deliveries, created_at, updated_at) VALUES ($1, $2, $3, $4, $5, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP)")
        .bind(user_id)
        .bind(vehicle_id)
        .bind(license_plate)
        .bind(UserStatus::Active)
        .bind(0)
        .execute(pool)
        .await?;

    Ok(())
}
