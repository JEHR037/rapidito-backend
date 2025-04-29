use sqlx::{query, Pool, Postgres};
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



pub async fn update_product(pool: &Pool<Postgres>, product_id: Uuid, name: Option<&str>, description: Option<&str>, price: Option<i64>, subprice: Option<i64>, status: Option<ProductStatus>) -> Result<(), sqlx::Error> {
    let mut query = query("UPDATE products SET updated_at = CURRENT_TIMESTAMP");
    
    if let Some(n) = name {
        query = query.bind(n);
    }
    if let Some(d) = description {
        query = query.bind(d);
    }
    if let Some(p) = price {
        query = query.bind(p);
    }
    if let Some(sp) = subprice {
        query = query.bind(sp);
    }
    if let Some(s) = status {
        query = query.bind(s);
    }
    
    query.bind(product_id).execute(pool).await?;
    Ok(())
}

pub async fn delete_product(pool: &Pool<Postgres>, product_id: Uuid) -> Result<(), sqlx::Error> {
    query("DELETE FROM products WHERE id = $1")
        .bind(product_id)
        .execute(pool)
        .await?;
    Ok(())
}
