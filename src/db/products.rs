use sqlx::{query, Pool, Postgres, Row};
use serde::{Serialize, Deserialize};
use sqlx::Type;
use sqlx::types::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type)]
#[sqlx(type_name = "ProductStatus")]
pub enum ProductStatus {
    Active,
    Inactive,
    Deleted,
    Offers,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]    
pub struct Product {
    pub id: Uuid,
    pub name: String,
    pub description: String,
    pub subprice: i64,
    pub price: i64,
    pub image: String,
    pub store_id: Uuid,
    pub category: String,
    pub popularity: i32,
    pub status: ProductStatus,
}

pub async fn get_product_by_id(pool: &Pool<Postgres>, product_id: Uuid) -> Result<(Uuid, String, String, i64, i64, String, Uuid, String, i32, ProductStatus), sqlx::Error> {
    let row = query("SELECT id, name, description, subprice, price, image, store_id, category, popularity, status FROM products WHERE id = $1")
        .bind(product_id)
        .fetch_one(pool)
        .await?;
    
    let product_id: Uuid = row.get(0);
    let product_name: String = row.get(1);
    let product_description: String = row.get(2);
    let product_subprice: i64 = row.get(3);
    let product_price: i64 = row.get(4);
    let product_image: String = row.get(5);
    let product_store_id: Uuid = row.get(6);
    let product_category: String = row.get(7);
    let product_popularity: i32 = row.get(8);
    let product_status: ProductStatus = row.get(9);
    
    Ok((product_id, product_name, product_description, product_subprice, product_price, product_image, product_store_id, product_category, product_popularity, product_status))
}

pub async fn create_product(pool: &Pool<Postgres>, name: &str, description: &str, subprice: i64, price: i64, image: &str, store_id: Uuid, category: &str, popularity: i32) -> Result<Uuid, sqlx::Error> {
    let product_id = Uuid::new_v4(); 
    query("INSERT INTO products (id, name, description, subprice, price, image, store_id, category, popularity, created_at, updated_at, status) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP, $10)")
        .bind(product_id)
        .bind(name)
        .bind(description)
        .bind(subprice)
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
