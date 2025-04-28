use sqlx::{query, Pool, Postgres, Row};
use serde::{Serialize, Deserialize};
use sqlx::types::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ProductStatus {
    Active,
    Inactive,
    Deleted,
    Offers,
}

pub async fn get_product_by_id(pool: &Pool<Postgres>, product_id: i32) -> Result<(i32, String), sqlx::Error> {
    let row = query("SELECT id, name FROM products WHERE id = $1")
        .bind(product_id)
        .fetch_one(pool)
        .await?;
    
    let product_id: i32 = row.get(0);
    let product_name: String = row.get(1);
    Ok((product_id, product_name))
}

pub async fn create_product(pool: &Pool<Postgres>, name: &str, description: &str, price: f64, image: &str, store_id: Uuid, category: &str, popularity: i32) -> Result<Uuid, sqlx::Error> {
    let product_id = Uuid::new_v4(); 
    query("INSERT INTO products (id, name, description, price, image, store_id, category, popularity, created_at, updated_at, status) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP, $9)")
        .bind(product_id)
        .bind(name)
        .bind(description)
        .bind(price)
        .bind(image)
        .bind(store_id)
        .bind(category)
        .bind(popularity)
        .bind(ProductStatus::Active) 
        .execute(pool)
        .await?;
    Ok(product_id)
}

pub async fn update_product_status(pool: &Pool<Postgres>, product_id: Uuid, status: ProductStatus) -> Result<(), sqlx::Error> {
    let status_str = match status {
        ProductStatus::Active => "Active",
        ProductStatus::Inactive => "Inactive",
        ProductStatus::Deleted => "Deleted",
        ProductStatus::Offers => "Offers",
    };
    query("UPDATE products SET status = $1 WHERE id = $2")
        .bind(status_str)
        .bind(product_id)
        .execute(pool)
        .await?;
    Ok(())
}
