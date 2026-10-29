use serde::Serialize;
use sqlx::{FromRow, PgPool};

#[derive(Debug, Serialize, FromRow)]
#[serde(rename_all = "camelCase")]
pub struct AdminStats {
    pub users: i64,
    pub customers: i64,
    pub partners: i64,
    pub drivers: i64,
    pub stores: i64,
    pub products: i64,
    pub orders: i64,
    pub active_orders: i64,
    pub revenue_cents: i64,
}

pub async fn admin_stats(pool: &PgPool) -> sqlx::Result<AdminStats> {
    sqlx::query_as::<_, AdminStats>(
        "SELECT \
            (SELECT COUNT(*) FROM users WHERE status <> 'Deleted') AS users, \
            (SELECT COUNT(*) FROM users WHERE status <> 'Deleted' AND role = 'Customer') AS customers, \
            (SELECT COUNT(*) FROM users WHERE status <> 'Deleted' AND role = 'Partner') AS partners, \
            (SELECT COUNT(*) FROM users WHERE status <> 'Deleted' AND role = 'Driver') AS drivers, \
            (SELECT COUNT(*) FROM stores WHERE status <> 'Deleted') AS stores, \
            (SELECT COUNT(*) FROM products WHERE status <> 'Deleted') AS products, \
            (SELECT COUNT(*) FROM orders) AS orders, \
            (SELECT COUNT(*) FROM orders WHERE status NOT IN ('Delivered', 'Cancelled')) AS active_orders, \
            (SELECT COALESCE(SUM(total_cents), 0)::bigint FROM orders WHERE status = 'Delivered') AS revenue_cents",
    )
    .fetch_one(pool)
    .await
}
