use sqlx::{query, Pool, Postgres, Row};
use sqlx::PgPool;
use sqlx::types::Uuid;  
use serde::{Serialize, Deserialize};
use sqlx::Type;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type)]
pub enum UserStatus {
    Active,
    Inactive,
    Deleted,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type)]
pub enum UserRole {
    Admin,
    Partner,
    Driver,
    Customer,
}

pub async fn db_get_user_by_email(pool: &Pool<Postgres>, email: &str) -> Result<(Uuid, String, String, String, UserStatus, UserRole), sqlx::Error> {
    let row = query("SELECT id, email, password_hash, name, status, role FROM users WHERE email = $1")
        .bind(email)
        .fetch_one(pool)
        .await?;
    let user_id: Uuid = row.get(0);
    let user_email: String = row.get(1);
    let user_password_hash: String = row.get(2);
    let user_name: String = row.get(3);
    let user_status: UserStatus = row.get(4);
    let user_role: UserRole = row.get(5);
    Ok((user_id, user_email, user_password_hash, user_name, user_status, user_role))
}

pub async fn db_get_user_by_id(pool: &Pool<Postgres>, user_id: Uuid) -> Result<(String, String, UserStatus), sqlx::Error> {
    let row = query("SELECT email, name, status FROM users WHERE id = $1")
        .bind(user_id)
        .fetch_one(pool)
        .await.map_err(|e| {
            eprintln!("Error al obtener el usuario: {}", e);
            e
        })?;

    let email: String = row.get(0);
    let name: String = row.get(1);
    let status: UserStatus = row.get(2);

    Ok((email, name, status))
}

pub async fn db_create_user(pool: &PgPool, email: &str, name: &str, role: UserRole, status: UserStatus, password_hash: &str) -> Result<Uuid, sqlx::Error> {
    let user_id = Uuid::new_v4(); 
    query("INSERT INTO users (id, email, name, role, status, password_hash, created_at, updated_at) VALUES ($1, $2, $3, $4, $5, $6, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP)")
        .bind(user_id)
        .bind(email)
        .bind(name)
        .bind(role)
        .bind(status)
        .bind(password_hash)
        .execute(pool)
        .await?;
    Ok(user_id)
}

pub async fn db_update_user(
    pool: &PgPool,
    user_id: Uuid,
    email: Option<&str>,
    name: Option<&str>,
    _status: Option<UserStatus>,
    vehicle: Option<Option<Uuid>>,
    make: Option<&str>,
    model: Option<&str>,
    year: Option<i32>,
    license_plate: Option<&str>,
    photo: Option<&str>,
    phone: Option<&str>,
    rating: Option<f32>,
) -> Result<(), sqlx::Error> {
    let mut query = query("UPDATE users SET updated_at = CURRENT_TIMESTAMP");

    if let Some(e) = email {
        query = query.bind(e);
    }
    if let Some(n) = name {
        query = query.bind(n);
    }
    if let Some(v) = vehicle {
        query = query.bind(v);
    }
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

pub async fn db_delete_user(pool: &Pool<Postgres>, user_id: Uuid) -> Result<(), sqlx::Error> {
    query("DELETE FROM users WHERE id = $1")
        .bind(user_id)
        .execute(pool)
        .await?;
    Ok(())
}
