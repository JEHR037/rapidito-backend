use actix_web::{delete, get, post, put, web, HttpResponse};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use uuid::Uuid;

use crate::auth::AuthUser;
use crate::db::products::{self, Product};
use crate::db::stores::{self, Store, StoreFields};
use crate::db::users;
use crate::domain::{StoreStatus, StoreTheme, UserRole};
use crate::error::{ApiError, ApiResult};
use crate::validation;

#[derive(Debug, Deserialize)]
struct StoreQuery {
    category: Option<String>,
    q: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct StoreInput {
    name: String,
    description: String,
    logo: String,
    cover: String,
    theme: StoreTheme,
    primary_color: Option<String>,
    secondary_color: Option<String>,
    delivery_time: String,
    categories: Vec<String>,
    status: Option<StoreStatus>,
    owner_id: Option<Uuid>,
    address: Option<String>,
    latitude: Option<f64>,
    longitude: Option<f64>,
}

#[derive(Debug, Serialize)]
struct StoreDetail {
    #[serde(flatten)]
    store: Store,
    products: Vec<Product>,
}

struct ValidStore {
    primary_color: Option<String>,
    secondary_color: Option<String>,
    categories: Vec<String>,
}

async fn validate(input: &StoreInput, pool: &PgPool) -> ApiResult<ValidStore> {
    validation::text("name", &input.name, 2, 100)?;
    validation::text("description", &input.description, 2, 1000)?;
    validation::text("logo", &input.logo, 1, 2048)?;
    validation::text("cover", &input.cover, 1, 2048)?;
    validation::text("deliveryTime", &input.delivery_time, 1, 50)?;
    validation::optional_text("address", input.address.as_deref(), 255)?;
    validation::coordinates(input.latitude, input.longitude)?;
    if input.status == Some(StoreStatus::Deleted) {
        return Err(ApiError::bad_request("status cannot be deleted; use DELETE instead"));
    }

    let mut categories: Vec<String> = Vec::new();
    for category in &input.categories {
        let category = validation::text("categories", category, 1, 50)?.to_string();
        if !categories.contains(&category) {
            categories.push(category);
        }
    }
    if categories.is_empty() || categories.len() > 20 {
        return Err(ApiError::bad_request("categories must contain between 1 and 20 entries"));
    }

    let primary_color = validation::color("primaryColor", input.primary_color.as_deref())?;
    let secondary_color = validation::color("secondaryColor", input.secondary_color.as_deref())?;
    if input.theme == StoreTheme::Custom && primary_color.is_none() {
        return Err(ApiError::bad_request("primaryColor is required when theme is custom"));
    }

    if let Some(owner_id) = input.owner_id {
        let owner = users::find_by_id(pool, owner_id)
            .await?
            .ok_or_else(|| ApiError::bad_request("ownerId does not reference an existing user"))?;
        if owner.role != UserRole::Partner {
            return Err(ApiError::bad_request("ownerId must reference a partner"));
        }
    }

    Ok(ValidStore { primary_color, secondary_color, categories })
}

fn fields<'a>(input: &'a StoreInput, valid: &'a ValidStore) -> StoreFields<'a> {
    StoreFields {
        name: input.name.trim(),
        description: input.description.trim(),
        logo: input.logo.trim(),
        cover: input.cover.trim(),
        theme: input.theme,
        primary_color: valid.primary_color.as_deref(),
        secondary_color: valid.secondary_color.as_deref(),
        delivery_time: input.delivery_time.trim(),
        categories: &valid.categories,
        status: input.status.unwrap_or(StoreStatus::Active),
        owner_id: input.owner_id,
        address: input.address.as_deref().map(str::trim).filter(|address| !address.is_empty()),
        latitude: input.latitude,
        longitude: input.longitude,
    }
}

#[get("/stores")]
async fn list(query: web::Query<StoreQuery>, pool: web::Data<PgPool>) -> ApiResult<HttpResponse> {
    let category = query.category.as_deref().map(str::trim).filter(|value| !value.is_empty());
    let search = query.q.as_deref().map(str::trim).filter(|value| !value.is_empty());
    Ok(HttpResponse::Ok().json(stores::list_public(&pool, category, search).await?))
}

#[get("/stores/mine")]
async fn mine(auth: AuthUser, pool: web::Data<PgPool>) -> ApiResult<HttpResponse> {
    auth.require(&[UserRole::Partner, UserRole::Admin])?;
    let result = if auth.is(UserRole::Admin) {
        stores::list_all(&pool).await?
    } else {
        stores::list_by_owner(&pool, auth.id).await?
    };
    Ok(HttpResponse::Ok().json(result))
}

#[get("/stores/{id}")]
async fn detail(path: web::Path<Uuid>, pool: web::Data<PgPool>) -> ApiResult<HttpResponse> {
    let id = path.into_inner();
    let store = stores::find_by_id(&pool, id)
        .await?
        .filter(|store| store.status == StoreStatus::Active)
        .ok_or_else(|| ApiError::not_found("store not found"))?;
    let products = products::list_purchasable_by_store(&pool, id).await?;
    Ok(HttpResponse::Ok().json(StoreDetail { store, products }))
}

#[post("/stores")]
async fn create(auth: AuthUser, input: web::Json<StoreInput>, pool: web::Data<PgPool>) -> ApiResult<HttpResponse> {
    auth.require(&[UserRole::Admin])?;
    let valid = validate(&input, &pool).await?;
    let store = stores::create(&pool, &fields(&input, &valid)).await?;
    Ok(HttpResponse::Created().json(store))
}

#[put("/stores/{id}")]
async fn update(
    auth: AuthUser,
    path: web::Path<Uuid>,
    input: web::Json<StoreInput>,
    pool: web::Data<PgPool>,
) -> ApiResult<HttpResponse> {
    auth.require(&[UserRole::Admin])?;
    let valid = validate(&input, &pool).await?;
    let store = stores::update(&pool, path.into_inner(), &fields(&input, &valid))
        .await?
        .ok_or_else(|| ApiError::not_found("store not found"))?;
    Ok(HttpResponse::Ok().json(store))
}

#[delete("/stores/{id}")]
async fn remove(auth: AuthUser, path: web::Path<Uuid>, pool: web::Data<PgPool>) -> ApiResult<HttpResponse> {
    auth.require(&[UserRole::Admin])?;
    if !stores::soft_delete(&pool, path.into_inner()).await? {
        return Err(ApiError::not_found("store not found"));
    }
    Ok(HttpResponse::NoContent().finish())
}

pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.service(list).service(mine).service(detail).service(create).service(update).service(remove);
}
