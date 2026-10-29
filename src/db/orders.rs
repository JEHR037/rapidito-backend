use std::collections::HashMap;

use chrono::{DateTime, Utc};
use serde::Serialize;
use sqlx::{FromRow, PgPool, Postgres, Transaction};
use uuid::Uuid;

use crate::domain::{OrderStatus, ProductStatus};
use crate::pricing::{CatalogEntry, PricedLine};

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OrderItem {
    pub product_id: Uuid,
    pub name: String,
    pub quantity: i32,
    pub unit_price_cents: i64,
    pub line_total_cents: i64,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OrderStore {
    pub id: Uuid,
    pub name: String,
    pub address: Option<String>,
    pub latitude: Option<f64>,
    pub longitude: Option<f64>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OrderDriver {
    pub id: Uuid,
    pub name: String,
    pub phone: Option<String>,
    pub photo: String,
    pub rating: f32,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Order {
    pub id: Uuid,
    pub status: OrderStatus,
    pub customer_id: Uuid,
    pub customer_name: String,
    pub store: OrderStore,
    pub driver: Option<OrderDriver>,
    pub items: Vec<OrderItem>,
    pub subtotal_cents: i64,
    pub delivery_fee_cents: i64,
    pub total_cents: i64,
    pub delivery_address: String,
    pub delivery_latitude: Option<f64>,
    pub delivery_longitude: Option<f64>,
    pub phone: String,
    pub payment_method: String,
    pub payment_reference: Option<String>,
    pub notes: Option<String>,
    pub estimated_delivery_time: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, FromRow)]
struct OrderRow {
    id: Uuid,
    status: OrderStatus,
    customer_id: Uuid,
    customer_name: String,
    store_id: Uuid,
    store_name: String,
    store_address: Option<String>,
    store_latitude: Option<f64>,
    store_longitude: Option<f64>,
    store_owner_id: Option<Uuid>,
    driver_id: Option<Uuid>,
    driver_name: Option<String>,
    driver_phone: Option<String>,
    driver_photo: Option<String>,
    driver_rating: Option<f32>,
    subtotal_cents: i64,
    delivery_fee_cents: i64,
    total_cents: i64,
    delivery_address: String,
    delivery_latitude: Option<f64>,
    delivery_longitude: Option<f64>,
    phone: String,
    payment_method: Option<String>,
    payment_reference: Option<String>,
    notes: Option<String>,
    estimated_delivery_time: Option<String>,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

#[derive(Debug, FromRow)]
struct ItemRow {
    order_id: Uuid,
    product_id: Uuid,
    product_name: String,
    quantity: i32,
    unit_price_cents: i64,
}

#[derive(Debug, Clone, Copy)]
pub struct OrderParties {
    pub status: OrderStatus,
    pub customer_id: Uuid,
    pub store_owner_id: Option<Uuid>,
    pub driver_id: Option<Uuid>,
}

#[derive(Debug, Clone, Copy, Default)]
pub struct OrderFilter {
    pub customer_id: Option<Uuid>,
    pub store_owner_id: Option<Uuid>,
    pub driver_id: Option<Uuid>,
    pub status: Option<OrderStatus>,
    pub only_available: bool,
}

pub struct NewOrder<'a> {
    pub customer_id: Uuid,
    pub store_id: Uuid,
    pub lines: &'a [PricedLine],
    pub subtotal_cents: i64,
    pub delivery_fee_cents: i64,
    pub delivery_address: &'a str,
    pub delivery_latitude: Option<f64>,
    pub delivery_longitude: Option<f64>,
    pub phone: &'a str,
    pub payment_reference: &'a str,
    pub notes: Option<&'a str>,
    pub estimated_delivery_time: &'a str,
}

const ORDER_SELECT: &str = "SELECT o.id, o.status, o.customer_id, COALESCE(cu.name, '') AS customer_name, \
         s.id AS store_id, s.name AS store_name, s.address AS store_address, \
         s.latitude AS store_latitude, s.longitude AS store_longitude, s.owner_id AS store_owner_id, \
         o.driver_id, du.name AS driver_name, d.phone AS driver_phone, d.photo AS driver_photo, \
         d.rating AS driver_rating, o.subtotal_cents, o.delivery_fee_cents, o.total_cents, \
         o.delivery_address, o.delivery_latitude, o.delivery_longitude, o.phone, o.payment_method, \
         o.payment_reference, o.notes, o.estimated_delivery_time, o.created_at, o.updated_at \
     FROM orders o \
     LEFT JOIN users cu ON cu.id = o.customer_id \
     JOIN stores s ON s.id = o.store_id \
     LEFT JOIN drivers d ON d.id = o.driver_id \
     LEFT JOIN users du ON du.id = d.id";

impl OrderRow {
    fn into_order(self, items: Vec<OrderItem>) -> Order {
        let driver = match (self.driver_id, self.driver_name) {
            (Some(id), Some(name)) => Some(OrderDriver {
                id,
                name,
                phone: self.driver_phone,
                photo: self.driver_photo.unwrap_or_default(),
                rating: self.driver_rating.unwrap_or_default(),
            }),
            _ => None,
        };
        Order {
            id: self.id,
            status: self.status,
            customer_id: self.customer_id,
            customer_name: self.customer_name,
            store: OrderStore {
                id: self.store_id,
                name: self.store_name,
                address: self.store_address,
                latitude: self.store_latitude,
                longitude: self.store_longitude,
            },
            driver,
            items,
            subtotal_cents: self.subtotal_cents,
            delivery_fee_cents: self.delivery_fee_cents,
            total_cents: self.total_cents,
            delivery_address: self.delivery_address,
            delivery_latitude: self.delivery_latitude,
            delivery_longitude: self.delivery_longitude,
            phone: self.phone,
            payment_method: self.payment_method.unwrap_or_else(|| "bank_transfer".to_string()),
            payment_reference: self.payment_reference,
            notes: self.notes,
            estimated_delivery_time: self.estimated_delivery_time,
            created_at: self.created_at,
            updated_at: self.updated_at,
        }
    }

    fn parties(&self) -> OrderParties {
        OrderParties {
            status: self.status,
            customer_id: self.customer_id,
            store_owner_id: self.store_owner_id,
            driver_id: self.driver_id,
        }
    }
}

async fn attach_items(pool: &PgPool, rows: Vec<OrderRow>) -> sqlx::Result<Vec<(OrderParties, Order)>> {
    let ids: Vec<Uuid> = rows.iter().map(|row| row.id).collect();
    let item_rows = sqlx::query_as::<_, ItemRow>(
        "SELECT order_id, product_id, product_name, quantity, unit_price_cents \
         FROM order_items WHERE order_id = ANY($1) ORDER BY position, created_at, id",
    )
    .bind(&ids)
    .fetch_all(pool)
    .await?;

    let mut items_by_order: HashMap<Uuid, Vec<OrderItem>> = HashMap::new();
    for item in item_rows {
        items_by_order.entry(item.order_id).or_default().push(OrderItem {
            product_id: item.product_id,
            name: item.product_name,
            quantity: item.quantity,
            unit_price_cents: item.unit_price_cents,
            line_total_cents: item.unit_price_cents * i64::from(item.quantity),
        });
    }

    Ok(rows
        .into_iter()
        .map(|row| {
            let items = items_by_order.remove(&row.id).unwrap_or_default();
            (row.parties(), row.into_order(items))
        })
        .collect())
}

pub async fn list(pool: &PgPool, filter: OrderFilter) -> sqlx::Result<Vec<Order>> {
    let rows = sqlx::query_as::<_, OrderRow>(&format!(
        "{ORDER_SELECT} \
         WHERE ($1::uuid IS NULL OR o.customer_id = $1) \
           AND ($2::uuid IS NULL OR s.owner_id = $2) \
           AND ($3::uuid IS NULL OR o.driver_id = $3) \
           AND ($4::orderstatus IS NULL OR o.status = $4) \
           AND (NOT $5 OR (o.driver_id IS NULL AND o.status = ANY($6))) \
         ORDER BY o.created_at DESC"
    ))
    .bind(filter.customer_id)
    .bind(filter.store_owner_id)
    .bind(filter.driver_id)
    .bind(filter.status)
    .bind(filter.only_available)
    .bind(OrderStatus::ASSIGNABLE.to_vec())
    .fetch_all(pool)
    .await?;

    Ok(attach_items(pool, rows).await?.into_iter().map(|(_, order)| order).collect())
}

pub async fn find_with_parties(pool: &PgPool, id: Uuid) -> sqlx::Result<Option<(OrderParties, Order)>> {
    let Some(row) = sqlx::query_as::<_, OrderRow>(&format!("{ORDER_SELECT} WHERE o.id = $1"))
        .bind(id)
        .fetch_optional(pool)
        .await?
    else {
        return Ok(None);
    };
    Ok(attach_items(pool, vec![row]).await?.into_iter().next())
}

pub async fn load_catalog(
    tx: &mut Transaction<'_, Postgres>,
    product_ids: &[Uuid],
) -> sqlx::Result<Vec<CatalogEntry>> {
    let rows: Vec<(Uuid, Uuid, String, i64, ProductStatus)> = sqlx::query_as(
        "SELECT p.id, p.store_id, p.name, p.price_cents, p.status FROM products p WHERE p.id = ANY($1)",
    )
    .bind(product_ids)
    .fetch_all(&mut **tx)
    .await?;
    Ok(rows
        .into_iter()
        .map(|(id, store_id, name, price_cents, status)| CatalogEntry { id, store_id, name, price_cents, status })
        .collect())
}

pub async fn insert(tx: &mut Transaction<'_, Postgres>, order: &NewOrder<'_>) -> sqlx::Result<Uuid> {
    let order_id: Uuid = sqlx::query_scalar(
        "INSERT INTO orders (customer_id, store_id, status, subtotal_cents, delivery_fee_cents, total_cents, \
             delivery_address, delivery_latitude, delivery_longitude, phone, payment_method, payment_reference, \
             notes, estimated_delivery_time) \
         VALUES ($1, $2, 'Processing', $3, $4, $5, $6, $7, $8, $9, 'bank_transfer', $10, $11, $12) RETURNING id",
    )
    .bind(order.customer_id)
    .bind(order.store_id)
    .bind(order.subtotal_cents)
    .bind(order.delivery_fee_cents)
    .bind(order.subtotal_cents + order.delivery_fee_cents)
    .bind(order.delivery_address)
    .bind(order.delivery_latitude)
    .bind(order.delivery_longitude)
    .bind(order.phone)
    .bind(order.payment_reference)
    .bind(order.notes)
    .bind(order.estimated_delivery_time)
    .fetch_one(&mut **tx)
    .await?;

    for (position, line) in (0_i16..).zip(order.lines) {
        sqlx::query(
            "INSERT INTO order_items (order_id, product_id, product_name, quantity, unit_price_cents, position) \
             VALUES ($1, $2, $3, $4, $5, $6)",
        )
        .bind(order_id)
        .bind(line.product_id)
        .bind(&line.name)
        .bind(line.quantity)
        .bind(line.unit_price_cents)
        .bind(position)
        .execute(&mut **tx)
        .await?;
    }

    sqlx::query("UPDATE stores SET popularity = popularity + 1 WHERE id = $1")
        .bind(order.store_id)
        .execute(&mut **tx)
        .await?;
    let product_ids: Vec<Uuid> = order.lines.iter().map(|line| line.product_id).collect();
    sqlx::query("UPDATE products SET popularity = popularity + 1 WHERE id = ANY($1)")
        .bind(&product_ids)
        .execute(&mut **tx)
        .await?;

    Ok(order_id)
}

pub async fn transition_status(
    pool: &PgPool,
    id: Uuid,
    from: OrderStatus,
    to: OrderStatus,
) -> sqlx::Result<bool> {
    let mut tx = pool.begin().await?;
    let driver_id: Option<Option<Uuid>> = sqlx::query_scalar(
        "UPDATE orders SET status = $3, updated_at = CURRENT_TIMESTAMP WHERE id = $1 AND status = $2 RETURNING driver_id",
    )
    .bind(id)
    .bind(from)
    .bind(to)
    .fetch_optional(&mut *tx)
    .await?;
    let Some(driver_id) = driver_id else {
        return Ok(false);
    };

    if let (OrderStatus::Delivered, Some(driver_id)) = (to, driver_id) {
        sqlx::query("UPDATE drivers SET deliveries = deliveries + 1, updated_at = CURRENT_TIMESTAMP WHERE id = $1")
            .bind(driver_id)
            .execute(&mut *tx)
            .await?;
    }

    tx.commit().await?;
    Ok(true)
}

pub async fn assign_driver(pool: &PgPool, id: Uuid, driver_id: Uuid) -> sqlx::Result<bool> {
    let result = sqlx::query(
        "UPDATE orders SET driver_id = $2, updated_at = CURRENT_TIMESTAMP \
         WHERE id = $1 AND driver_id IS NULL AND status = ANY($3)",
    )
    .bind(id)
    .bind(driver_id)
    .bind(OrderStatus::ASSIGNABLE.to_vec())
    .execute(pool)
    .await?;
    Ok(result.rows_affected() > 0)
}
