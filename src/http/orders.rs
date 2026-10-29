use actix_web::{get, patch, post, web, HttpResponse};
use serde::Deserialize;
use sqlx::PgPool;
use uuid::Uuid;

use crate::auth::AuthUser;
use crate::config::AppConfig;
use crate::db::orders::{self, NewOrder, Order, OrderFilter, OrderParties};
use crate::db::{drivers, stores};
use crate::domain::{OrderActor, OrderStatus, StoreStatus, UserRole};
use crate::error::{ApiError, ApiResult};
use crate::pricing::{self, RequestedLine};
use crate::validation;

#[derive(Debug, Deserialize)]
struct OrderQuery {
    status: Option<OrderStatus>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct OrderLineInput {
    product_id: Uuid,
    quantity: i32,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct CreateOrderInput {
    store_id: Uuid,
    items: Vec<OrderLineInput>,
    delivery_address: String,
    delivery_latitude: Option<f64>,
    delivery_longitude: Option<f64>,
    phone: String,
    payment_reference: String,
    notes: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct StatusInput {
    status: OrderStatus,
}

fn actors(auth: &AuthUser, parties: &OrderParties) -> Vec<OrderActor> {
    let mut actors = Vec::new();
    if auth.is(UserRole::Admin) {
        actors.push(OrderActor::Admin);
    }
    if parties.customer_id == auth.id {
        actors.push(OrderActor::Customer);
    }
    if auth.is(UserRole::Partner) && parties.store_owner_id == Some(auth.id) {
        actors.push(OrderActor::StoreOwner);
    }
    if auth.is(UserRole::Driver) && parties.driver_id == Some(auth.id) {
        actors.push(OrderActor::AssignedDriver);
    }
    actors
}

async fn load_visible(pool: &PgPool, auth: &AuthUser, id: Uuid) -> ApiResult<(OrderParties, Order)> {
    let not_found = || ApiError::not_found("order not found");
    let (parties, order) = orders::find_with_parties(pool, id).await?.ok_or_else(not_found)?;
    if actors(auth, &parties).is_empty() {
        return Err(not_found());
    }
    Ok((parties, order))
}

#[post("/orders")]
async fn create(
    auth: AuthUser,
    input: web::Json<CreateOrderInput>,
    pool: web::Data<PgPool>,
    config: web::Data<AppConfig>,
) -> ApiResult<HttpResponse> {
    auth.require(&[UserRole::Customer])?;
    let delivery_address = validation::text("deliveryAddress", &input.delivery_address, 5, 255)?;
    let phone = validation::phone(&input.phone)?;
    let payment_reference = validation::text("paymentReference", &input.payment_reference, 6, 64)?;
    let notes = validation::optional_text("notes", input.notes.as_deref(), 500)?;
    validation::coordinates(input.delivery_latitude, input.delivery_longitude)?;

    let store = stores::find_by_id(&pool, input.store_id)
        .await?
        .filter(|store| store.status == StoreStatus::Active)
        .ok_or_else(|| ApiError::bad_request("storeId does not reference an active store"))?;

    let requested: Vec<RequestedLine> = input
        .items
        .iter()
        .map(|line| RequestedLine { product_id: line.product_id, quantity: line.quantity })
        .collect();
    let product_ids: Vec<Uuid> = requested.iter().map(|line| line.product_id).collect();

    let mut tx = pool.begin().await?;
    let catalog = orders::load_catalog(&mut tx, &product_ids).await?;
    let lines = pricing::price_lines(store.id, &requested, &catalog)
        .map_err(|error| ApiError::bad_request(error.to_string()))?;

    let new_order = NewOrder {
        customer_id: auth.id,
        store_id: store.id,
        lines: &lines,
        subtotal_cents: pricing::subtotal_cents(&lines),
        delivery_fee_cents: config.delivery_fee_cents,
        delivery_address,
        delivery_latitude: input.delivery_latitude,
        delivery_longitude: input.delivery_longitude,
        phone,
        payment_reference,
        notes,
        estimated_delivery_time: &store.delivery_time,
    };
    let order_id = orders::insert(&mut tx, &new_order).await?;
    tx.commit().await?;

    let (_, order) = orders::find_with_parties(&pool, order_id)
        .await?
        .ok_or_else(|| ApiError::Internal("created order vanished".to_string()))?;
    Ok(HttpResponse::Created().json(order))
}

#[get("/orders")]
async fn list(auth: AuthUser, query: web::Query<OrderQuery>, pool: web::Data<PgPool>) -> ApiResult<HttpResponse> {
    let mut filter = OrderFilter { status: query.status, ..OrderFilter::default() };
    match auth.role {
        UserRole::Admin => {}
        UserRole::Customer => filter.customer_id = Some(auth.id),
        UserRole::Partner => filter.store_owner_id = Some(auth.id),
        UserRole::Driver => filter.driver_id = Some(auth.id),
    }
    Ok(HttpResponse::Ok().json(orders::list(&pool, filter).await?))
}

#[get("/orders/available")]
async fn available(auth: AuthUser, pool: web::Data<PgPool>) -> ApiResult<HttpResponse> {
    auth.require(&[UserRole::Driver])?;
    let filter = OrderFilter { only_available: true, ..OrderFilter::default() };
    Ok(HttpResponse::Ok().json(orders::list(&pool, filter).await?))
}

#[get("/orders/{id}")]
async fn detail(auth: AuthUser, path: web::Path<Uuid>, pool: web::Data<PgPool>) -> ApiResult<HttpResponse> {
    let (_, order) = load_visible(&pool, &auth, path.into_inner()).await?;
    Ok(HttpResponse::Ok().json(order))
}

#[patch("/orders/{id}/status")]
async fn update_status(
    auth: AuthUser,
    path: web::Path<Uuid>,
    input: web::Json<StatusInput>,
    pool: web::Data<PgPool>,
) -> ApiResult<HttpResponse> {
    let id = path.into_inner();
    let (parties, _) = load_visible(&pool, &auth, id).await?;
    let next = input.status;

    if !parties.status.is_valid_transition(next) {
        return Err(ApiError::conflict(format!(
            "cannot move an order from {:?} to {:?}",
            parties.status, next
        )));
    }
    if !parties.status.can_transition(next, &actors(&auth, &parties)) {
        return Err(ApiError::forbidden("you cannot apply this status change"));
    }
    if !orders::transition_status(&pool, id, parties.status, next).await? {
        return Err(ApiError::conflict("the order changed meanwhile; reload it and try again"));
    }

    let (_, order) = load_visible(&pool, &auth, id).await?;
    Ok(HttpResponse::Ok().json(order))
}

#[post("/orders/{id}/assign")]
async fn assign(auth: AuthUser, path: web::Path<Uuid>, pool: web::Data<PgPool>) -> ApiResult<HttpResponse> {
    auth.require(&[UserRole::Driver])?;
    if !drivers::exists(&pool, auth.id).await? {
        return Err(ApiError::forbidden("driver profile not found"));
    }
    let id = path.into_inner();
    if !orders::assign_driver(&pool, id, auth.id).await? {
        return match orders::find_with_parties(&pool, id).await? {
            None => Err(ApiError::not_found("order not found")),
            Some(_) => Err(ApiError::conflict("order is not available for assignment")),
        };
    }
    let (_, order) = load_visible(&pool, &auth, id).await?;
    Ok(HttpResponse::Ok().json(order))
}

pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.service(create)
        .service(list)
        .service(available)
        .service(detail)
        .service(update_status)
        .service(assign);
}
