use sqlx::{query, Pool, Postgres, Row};
use serde::{Serialize, Deserialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum OrderStatus {
    Pending,
    Confirmed,
    Completed,
    Canceled,
}
pub async fn get_order_by_id(pool: &Pool<Postgres>, order_id: i32) -> Result<(i32, String), sqlx::Error> {
    let row = query("SELECT id, description FROM orders WHERE id = $1")
        .bind(order_id)
        .fetch_one(pool)
        .await?;
    
    let order_id: i32 = row.get(0);
    let order_description: String = row.get(1);
    Ok((order_id, order_description))
}

pub async fn create_order(pool: &Pool<Postgres>, description: &str) -> Result<i32, sqlx::Error> {
    let order_id = query("INSERT INTO orders (description) VALUES ($1) RETURNING id")
        .bind(description)
        .fetch_one(pool)
        .await?
        .get(0);
    Ok(order_id)
}

pub async fn update_order_status(pool: &Pool<Postgres>, order_id: i32, status: OrderStatus) -> Result<(), sqlx::Error> {
    let status_str = match status {
        OrderStatus::Pending => "pending",
        OrderStatus::Confirmed => "confirmed",
        OrderStatus::Completed => "completed",
        OrderStatus::Canceled => "canceled",
    };
    query("UPDATE orders SET status = $1 WHERE id = $2")
        .bind(status_str)
        .bind(order_id)
        .execute(pool)
        .await?;
    Ok(())
}
