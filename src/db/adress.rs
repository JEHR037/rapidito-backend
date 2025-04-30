use sqlx::{query, Pool, Postgres, Row};
use sqlx::types::Uuid;
use serde::{Serialize, Deserialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct Address {
    pub id: Uuid,
    pub user_id: Uuid,
    pub street: String,
    pub city: String,
    pub state: String,
    pub postal_code: String,
    pub country: String,
}

pub async fn db_create_address(
    pool: &Pool<Postgres>,
    user_id: Uuid,
    street: &str,
    city: &str,
    state: &str,
    postal_code: &str,
    country: &str,
) -> Result<Uuid, sqlx::Error> {
    let address_id = query(
        "INSERT INTO addresses (user_id, street, city, state, postal_code, country, created_at, updated_at) \
         VALUES ($1, $2, $3, $4, $5, $6, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP) RETURNING id"
    )
    .bind(user_id)
    .bind(street)
    .bind(city)
    .bind(state)
    .bind(postal_code)
    .bind(country)
    .fetch_one(pool)
    .await?
    .get(0);

    Ok(address_id)
}

pub async fn db_get_addresses_by_user(
    pool: &Pool<Postgres>,
    user_id: Uuid,
) -> Result<Vec<Address>, sqlx::Error> {
    let rows = query(
        "SELECT id, user_id, street, city, state, postal_code, country FROM addresses WHERE user_id = $1"
    )
    .bind(user_id)
    .fetch_all(pool)
    .await?;

    let addresses = rows
        .into_iter()
        .map(|row| Address {
            id: row.get(0),
            user_id: row.get(1),
            street: row.get(2),
            city: row.get(3),
            state: row.get(4),
            postal_code: row.get(5),
            country: row.get(6),
        })
        .collect();

    Ok(addresses)
}

pub async fn db_delete_address(
    pool: &Pool<Postgres>,
    address_id: Uuid,
) -> Result<(), sqlx::Error> {
    query("DELETE FROM addresses WHERE id = $1")
        .bind(address_id)
        .execute(pool)
        .await?;

    Ok(())
}