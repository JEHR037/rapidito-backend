use crate::db::users::{UserStatus};
use sqlx::{query,Row};
use serde::{Serialize, Deserialize};
use sqlx::types::Uuid;  
use sqlx::PgPool;


#[derive(Debug, Serialize, Deserialize)]
pub struct DriverProfile {
    pub name: String,
    pub photo: String,
    pub phone: String,
    pub rating: f32,
}


pub async fn create_driver_from_user(pool: &PgPool, user_id: Uuid, vehicle_id: Uuid, license_plate: &str, name: &str, photo: &str, phone: &str, rating: f32, email: &str, status: UserStatus) -> Result<(), sqlx::Error> {
    query("INSERT INTO drivers (id, vehicle, license_plate, name, photo, phone, rating, email, status, deliveries, created_at, updated_at) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP)")
        .bind(user_id)
        .bind(vehicle_id)
        .bind(license_plate)
        .bind(name)
        .bind(photo)
        .bind(phone)
        .bind(rating)
        .bind(email)
        .bind(status)
        .bind(0)
        .execute(pool)
        .await?;

    Ok(())
}

pub async fn update_driver(pool: &PgPool, user_id: Uuid, name: Option<&str>, photo: Option<&str>, phone: Option<&str>, rating: Option<f32>) -> Result<(), sqlx::Error> {
    let mut query = query("UPDATE drivers SET updated_at = CURRENT_TIMESTAMP");
    
    if let Some(n) = name {
        query = query.bind(n);
    }
    if let Some(p) = photo {
        query = query.bind(p);
    }
    if let Some(ph) = phone {
        query = query.bind(ph);
    }
    if let Some(r) = rating {
        query = query.bind(r);
    }
    
    query.bind(user_id).execute(pool).await?;
    Ok(())
}

pub async fn delete_driver(pool: &PgPool, user_id: Uuid) -> Result<(), sqlx::Error> {
    query("DELETE FROM drivers WHERE id = $1")
        .bind(user_id)
        .execute(pool)
        .await?;
    Ok(())
}

pub async fn get_driver_by_user_id(pool: &PgPool, user_id: Uuid) -> Result<(f32, DriverProfile), sqlx::Error> {
    let row = query("SELECT name, photo, phone, rating FROM drivers WHERE id = $1")
        .bind(user_id)
        .fetch_one(pool)
        .await?;

    let name: String = row.get(0);
    let photo: String = row.get(1);
    let phone: String = row.get(2);
    let rating: f32 = row.get(3);

    let profile = DriverProfile {
        name,
        photo,
        phone,
        rating,
    };

    Ok((rating, profile))
}
