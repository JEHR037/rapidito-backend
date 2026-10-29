use actix_web::{delete, get, post, put, web, HttpResponse};
use serde::Deserialize;
use sqlx::PgPool;
use uuid::Uuid;

use crate::auth::AuthUser;
use crate::db::products::{self, ProductFields, ProductFilter};
use crate::db::stores;
use crate::domain::{ProductStatus, UserRole};
use crate::error::{ApiError, ApiResult};
use crate::validation;

const DEFAULT_LIMIT: i64 = 50;
const MAX_LIMIT: i64 = 200;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ProductQuery {
    store_id: Option<Uuid>,
    category: Option<String>,
    q: Option<String>,
    limit: Option<i64>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct ProductInput {
    store_id: Uuid,
    name: String,
    description: String,
    price_cents: i64,
    compare_at_price_cents: Option<i64>,
    image: String,
    category: String,
    status: Option<ProductStatus>,
}

async fn authorize_store(auth: &AuthUser, pool: &PgPool, store_id: Uuid) -> ApiResult<()> {
    auth.require(&[UserRole::Admin, UserRole::Partner])?;
    if auth.is(UserRole::Partner) && !stores::is_owned_by(pool, store_id, auth.id).await? {
        return Err(ApiError::forbidden("you do not own this store"));
    }
    Ok(())
}

fn fields(input: &ProductInput) -> ApiResult<ProductFields<'_>> {
    let price_cents = validation::cents("priceCents", input.price_cents)?;
    let compare_at_price_cents = input
        .compare_at_price_cents
        .map(|value| validation::cents("compareAtPriceCents", value))
        .transpose()?;
    if input.status == Some(ProductStatus::Deleted) {
        return Err(ApiError::bad_request("status cannot be deleted; use DELETE instead"));
    }
    Ok(ProductFields {
        store_id: input.store_id,
        name: validation::text("name", &input.name, 2, 120)?,
        description: validation::text("description", &input.description, 2, 1000)?,
        price_cents,
        compare_at_price_cents,
        image: validation::text("image", &input.image, 1, 2048)?,
        category: validation::text("category", &input.category, 1, 50)?,
        status: input.status.unwrap_or(ProductStatus::Active),
    })
}

#[get("/products")]
async fn list(query: web::Query<ProductQuery>, pool: web::Data<PgPool>) -> ApiResult<HttpResponse> {
    let filter = ProductFilter {
        store_id: query.store_id,
        category: query.category.as_deref().map(str::trim).filter(|value| !value.is_empty()),
        search: query.q.as_deref().map(str::trim).filter(|value| !value.is_empty()),
        limit: query.limit.unwrap_or(DEFAULT_LIMIT).clamp(1, MAX_LIMIT),
    };
    Ok(HttpResponse::Ok().json(products::list_public(&pool, &filter).await?))
}

#[get("/products/{id}")]
async fn detail(path: web::Path<Uuid>, pool: web::Data<PgPool>) -> ApiResult<HttpResponse> {
    let product = products::find_by_id(&pool, path.into_inner())
        .await?
        .filter(|product| product.status.is_purchasable())
        .ok_or_else(|| ApiError::not_found("product not found"))?;
    Ok(HttpResponse::Ok().json(product))
}

#[post("/products")]
async fn create(auth: AuthUser, input: web::Json<ProductInput>, pool: web::Data<PgPool>) -> ApiResult<HttpResponse> {
    let fields = fields(&input)?;
    authorize_store(&auth, &pool, fields.store_id).await?;
    let id = products::create(&pool, &fields).await?;
    let product = products::find_by_id(&pool, id).await?.ok_or_else(|| ApiError::Internal("created product vanished".to_string()))?;
    Ok(HttpResponse::Created().json(product))
}

#[put("/products/{id}")]
async fn update(
    auth: AuthUser,
    path: web::Path<Uuid>,
    input: web::Json<ProductInput>,
    pool: web::Data<PgPool>,
) -> ApiResult<HttpResponse> {
    let id = path.into_inner();
    let fields = fields(&input)?;
    let current = products::find_by_id(&pool, id)
        .await?
        .filter(|product| product.status != ProductStatus::Deleted)
        .ok_or_else(|| ApiError::not_found("product not found"))?;
    authorize_store(&auth, &pool, current.store_id).await?;
    authorize_store(&auth, &pool, fields.store_id).await?;
    if !products::update(&pool, id, &fields).await? {
        return Err(ApiError::not_found("product not found"));
    }
    let product = products::find_by_id(&pool, id).await?.ok_or_else(|| ApiError::not_found("product not found"))?;
    Ok(HttpResponse::Ok().json(product))
}

#[delete("/products/{id}")]
async fn remove(auth: AuthUser, path: web::Path<Uuid>, pool: web::Data<PgPool>) -> ApiResult<HttpResponse> {
    let id = path.into_inner();
    let current = products::find_by_id(&pool, id)
        .await?
        .filter(|product| product.status != ProductStatus::Deleted)
        .ok_or_else(|| ApiError::not_found("product not found"))?;
    authorize_store(&auth, &pool, current.store_id).await?;
    products::soft_delete(&pool, id).await?;
    Ok(HttpResponse::NoContent().finish())
}

pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.service(list).service(detail).service(create).service(update).service(remove);
}
