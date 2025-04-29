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


pub async fn update_order(pool: &Pool<Postgres>, order_id: Uuid, status: Option<OrderStatus>, total: Option<i64>) -> Result<(), sqlx::Error> {
    let mut query = query("UPDATE orders SET updated_at = CURRENT_TIMESTAMP");
    
    if let Some(s) = status {
        query = query.bind(s);
    }
    if let Some(t) = total {
        query = query.bind(t);
    }
    
    query.bind(order_id).execute(pool).await?;
    Ok(())
}

pub async fn delete_order(pool: &Pool<Postgres>, order_id: Uuid) -> Result<(), sqlx::Error> {
    query("DELETE FROM orders WHERE id = $1")
        .bind(order_id)
        .execute(pool)
        .await?;
    Ok(())
}
