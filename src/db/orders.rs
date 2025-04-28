use sqlx::{query, Pool, Postgres, Row, Type};
use serde::{Serialize, Deserialize};
use sqlx::types::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type)]
pub enum OrderStatus {
    Pending,
    Confirmed,
    Completed,
    Canceled,
}

#[derive(Debug, Clone,PartialEq, Eq, Serialize, Deserialize, Type)]
pub struct Order {
    pub id: Uuid,
    pub description: String,
    pub status: OrderStatus,
    pub store_id: Uuid,
    pub customer_id: Uuid,
    pub driver_id: Uuid,
    pub subtotal: i64,
    pub total: i64,
    pub address: String,
    pub latitude: i64,
    pub longitude: i64,
    pub estimated_delivery_time: String,
    pub phone: String,
    pub payment_method: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ProductOrder {
    pub product_id: Uuid,
    pub quantity: i32,
    pub price: f64,
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

pub async fn create_order(pool: &Pool<Postgres>, order: Order) -> Result<Uuid, sqlx::Error> {
    let order_id = query("INSERT INTO orders (description, status, store_id, customer_id, driver_id, subtotal, total, address, latitude, longitude, estimated_delivery_time, phone, payment_method, created_at, updated_at) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP) RETURNING id")
        .bind(order.description)
        .bind(OrderStatus::Pending)
        .bind(order.store_id)
        .bind(order.customer_id)
        .bind(order.driver_id)
        .bind(order.subtotal)
        .bind(order.total)
        .bind(order.address)
        .bind(order.latitude)
        .bind(order.longitude)
        .bind(order.estimated_delivery_time)
        .bind(order.phone)
        .bind(order.payment_method)
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
