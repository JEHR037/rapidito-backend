use sqlx::{query, Pool, Postgres};
use sqlx::types::Uuid;
use serde::{Serialize, Deserialize};
use sqlx::Row;


#[derive(Debug, Serialize, Deserialize)]
pub struct Vehicle {
    pub id: Uuid,
    pub make: String,
    pub model: String,
    pub year: i32,
    pub license_plate: String,
}

pub async fn create_vehicle(pool: &Pool<Postgres>, make: &str, model: &str, year: i32, license_plate: &str, user_id: &Uuid) -> Result<Uuid, sqlx::Error> {
    let vehicle_id = Uuid::new_v4();
    query("INSERT INTO vehicles (id, make, model, year, license_plate, user_id, created_at, updated_at, ) VALUES ($1, $2, $3, $4, $5, &6 CURRENT_TIMESTAMP, CURRENT_TIMESTAMP)")
        .bind(vehicle_id)
        .bind(make)
        .bind(model)
        .bind(year)
        .bind(license_plate)
        .bind(user_id)
        .execute(pool)
        .await?;
    Ok(vehicle_id)
}


pub async fn update_vehicle(pool: &Pool<Postgres>, vehicle_id: Uuid, make: Option<&str>, model: Option<&str>, year: Option<i32>, license_plate: Option<&str>) -> Result<(), sqlx::Error> {
    let mut query = query("UPDATE vehicles SET updated_at = CURRENT_TIMESTAMP");
    
    if let Some(m) = make {
        query = query.bind(m);
    }
    if let Some(m) = model {
        query = query.bind(m);
    }
    if let Some(y) = year {
        query = query.bind(y);
    }
    if let Some(lp) = license_plate {
        query = query.bind(lp);
    }
    
    query.bind(vehicle_id).execute(pool).await?;
    Ok(())
}

pub async fn delete_vehicle(pool: &Pool<Postgres>, vehicle_id: Uuid) -> Result<(), sqlx::Error> {
    query("DELETE FROM vehicles WHERE id = $1")
        .bind(vehicle_id)
        .execute(pool)
        .await?;
    Ok(())
}

pub async fn get_vehicles_by_user_id(pool: &Pool<Postgres>, user_id: Uuid) -> Result<Vec<Vehicle>, sqlx::Error> {
    let rows = query("SELECT id, make, model, year, license_plate FROM vehicles WHERE user_id = $1")
        .bind(user_id)
        .fetch_all(pool)
        .await?;

    let vehicles: Vec<Vehicle> = rows.iter().map(|row| {
        Vehicle {
            id: row.get(0),
            make: row.get(1),
            model: row.get(2),
            year: row.get(3),
            license_plate: row.get(4),
        }
    }).collect();

    Ok(vehicles)
}

pub async fn get_vehicle_by_id(pool: &Pool<Postgres>, vehicle_id: Uuid) -> Result<Vehicle, sqlx::Error> {
    let row = query("SELECT id, make, model, year, license_plate FROM vehicles WHERE id = $1")
        .bind(vehicle_id)
        .fetch_one(pool)
        .await?;

    let vehicle = Vehicle {
        id: row.get(0),
        make: row.get(1),
        model: row.get(2),
        year: row.get(3),
        license_plate: row.get(4),
    };

    Ok(vehicle)
}