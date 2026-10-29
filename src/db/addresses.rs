use serde::Serialize;
use sqlx::{FromRow, PgPool};
use uuid::Uuid;

#[derive(Debug, Serialize, FromRow)]
#[serde(rename_all = "camelCase")]
pub struct Address {
    pub id: Uuid,
    pub label: String,
    pub street: String,
    pub city: String,
    pub state: String,
    pub postal_code: String,
    pub country: String,
    pub latitude: Option<f64>,
    pub longitude: Option<f64>,
    pub is_default: bool,
}

pub struct NewAddress<'a> {
    pub label: &'a str,
    pub street: &'a str,
    pub city: &'a str,
    pub state: &'a str,
    pub postal_code: &'a str,
    pub country: &'a str,
    pub latitude: Option<f64>,
    pub longitude: Option<f64>,
    pub is_default: bool,
}

const ADDRESS_COLUMNS: &str =
    "id, label, street, city, state, postal_code, country, latitude, longitude, is_default";

pub async fn list_by_user(pool: &PgPool, user_id: Uuid) -> sqlx::Result<Vec<Address>> {
    sqlx::query_as::<_, Address>(&format!(
        "SELECT {ADDRESS_COLUMNS} FROM addresses WHERE user_id = $1 ORDER BY is_default DESC, created_at"
    ))
    .bind(user_id)
    .fetch_all(pool)
    .await
}

pub async fn create(pool: &PgPool, user_id: Uuid, input: &NewAddress<'_>) -> sqlx::Result<Address> {
    let mut tx = pool.begin().await?;

    let has_default: bool =
        sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM addresses WHERE user_id = $1 AND is_default)")
            .bind(user_id)
            .fetch_one(&mut *tx)
            .await?;
    let make_default = input.is_default || !has_default;
    if make_default && has_default {
        sqlx::query("UPDATE addresses SET is_default = false, updated_at = CURRENT_TIMESTAMP WHERE user_id = $1 AND is_default")
            .bind(user_id)
            .execute(&mut *tx)
            .await?;
    }

    let address = sqlx::query_as::<_, Address>(&format!(
        "INSERT INTO addresses (user_id, label, street, city, state, postal_code, country, latitude, longitude, is_default) \
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10) RETURNING {ADDRESS_COLUMNS}"
    ))
    .bind(user_id)
    .bind(input.label)
    .bind(input.street)
    .bind(input.city)
    .bind(input.state)
    .bind(input.postal_code)
    .bind(input.country)
    .bind(input.latitude)
    .bind(input.longitude)
    .bind(make_default)
    .fetch_one(&mut *tx)
    .await?;

    tx.commit().await?;
    Ok(address)
}

pub async fn set_default(pool: &PgPool, user_id: Uuid, address_id: Uuid) -> sqlx::Result<Option<Address>> {
    let mut tx = pool.begin().await?;

    let owned: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM addresses WHERE id = $1 AND user_id = $2)")
        .bind(address_id)
        .bind(user_id)
        .fetch_one(&mut *tx)
        .await?;
    if !owned {
        return Ok(None);
    }

    sqlx::query("UPDATE addresses SET is_default = false, updated_at = CURRENT_TIMESTAMP WHERE user_id = $1 AND is_default AND id <> $2")
        .bind(user_id)
        .bind(address_id)
        .execute(&mut *tx)
        .await?;
    let address = sqlx::query_as::<_, Address>(&format!(
        "UPDATE addresses SET is_default = true, updated_at = CURRENT_TIMESTAMP WHERE id = $1 RETURNING {ADDRESS_COLUMNS}"
    ))
    .bind(address_id)
    .fetch_one(&mut *tx)
    .await?;

    tx.commit().await?;
    Ok(Some(address))
}

pub async fn delete(pool: &PgPool, user_id: Uuid, address_id: Uuid) -> sqlx::Result<bool> {
    let mut tx = pool.begin().await?;

    let deleted_default: Option<bool> =
        sqlx::query_scalar("DELETE FROM addresses WHERE id = $1 AND user_id = $2 RETURNING is_default")
            .bind(address_id)
            .bind(user_id)
            .fetch_optional(&mut *tx)
            .await?;
    let Some(was_default) = deleted_default else {
        return Ok(false);
    };

    if was_default {
        sqlx::query(
            "UPDATE addresses SET is_default = true, updated_at = CURRENT_TIMESTAMP \
             WHERE id = (SELECT id FROM addresses WHERE user_id = $1 ORDER BY created_at LIMIT 1)",
        )
        .bind(user_id)
        .execute(&mut *tx)
        .await?;
    }

    tx.commit().await?;
    Ok(true)
}
