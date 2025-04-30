pub mod users;
pub mod stores;
pub mod products;
pub mod orders;
pub mod drivers;
pub mod vehicles;
pub mod payments;
pub mod adress;


use serde::{Serialize, Deserialize};
use sqlx::{Pool, Postgres};
use std::env;
use log::{info, error};

pub type DbPool = Pool<Postgres>;

pub async fn init_db() -> Result<DbPool, sqlx::Error> {
    let database_url = env::var("DATABASE_URL").expect("DATABASE_URL no está configurado");
    match Pool::<Postgres>::connect(&database_url).await {
        Ok(pool) => {
            info!("Conexión a la base de datos establecida correctamente.");
            Ok(pool)
        }
        Err(e) => {
            error!("Error al conectar a la base de datos: {}", e);
            Err(e)
        }
    }
}


#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum UserRole {
    Admin,
    Partner,
    Driver,
    Customer,
} 