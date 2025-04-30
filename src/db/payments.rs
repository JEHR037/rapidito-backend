use sqlx::{query, Pool, Postgres, Row};
use sqlx::types::Uuid;
use serde::{Serialize, Deserialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct PaymentMethod {
    pub id: Uuid,
    pub user_id: Uuid,
    pub card_number: String,
    pub card_holder_name: String,
    pub expiration_date: String,
    pub cvv: String,
}

pub async fn db_create_payment_method(
    pool: &Pool<Postgres>,
    user_id: Uuid,
    card_number: &str,
    card_holder_name: &str,
    expiration_date: &str,
    cvv: &str,
) -> Result<Uuid, sqlx::Error> {
    let payment_id = query(
        "INSERT INTO payment_methods (user_id, card_number, card_holder_name, expiration_date, cvv, created_at, updated_at) \
         VALUES ($1, $2, $3, $4, $5, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP) RETURNING id"
    )
    .bind(user_id)
    .bind(card_number)
    .bind(card_holder_name)
    .bind(expiration_date)
    .bind(cvv)
    .fetch_one(pool)
    .await?
    .get(0);

    Ok(payment_id)
}

pub async fn db_get_payment_methods_by_user(
    pool: &Pool<Postgres>,
    user_id: Uuid,
) -> Result<Vec<PaymentMethod>, sqlx::Error> {
    let rows = query(
        "SELECT id, user_id, card_number, card_holder_name, expiration_date, cvv FROM payment_methods WHERE user_id = $1"
    )
    .bind(user_id)
    .fetch_all(pool)
    .await?;

    let payment_methods = rows
        .into_iter()
        .map(|row| PaymentMethod {
            id: row.get(0),
            user_id: row.get(1),
            card_number: row.get(2),
            card_holder_name: row.get(3),
            expiration_date: row.get(4),
            cvv: row.get(5),
        })
        .collect();

    Ok(payment_methods)
}

pub async fn db_delete_payment_method(
    pool: &Pool<Postgres>,
    payment_id: Uuid,
) -> Result<(), sqlx::Error> {
    query("DELETE FROM payment_methods WHERE id = $1")
        .bind(payment_id)
        .execute(pool)
        .await?;

    Ok(())
}
