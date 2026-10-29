use actix_web::{get, patch, post, web, HttpResponse};
use serde::Deserialize;
use sqlx::PgPool;
use uuid::Uuid;

use crate::auth::AuthUser;
use crate::config::AppConfig;
use crate::db::orders::{self, NewOrder, Order, OrderFilter, OrderParties};
use crate::db::payments::{self as payment_store, PaymentSettings};
use crate::db::stores::Store;
use crate::db::{drivers, stores};
use crate::payments::{self, MethodQuote, PayerDetails, PaymentMethod};
use crate::domain::{OrderActor, OrderStatus, StoreStatus, UserRole};
use crate::error::{ApiError, ApiResult};
use crate::pricing::{self, PricedLine, RequestedLine};
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
    payment_method: PaymentMethod,
    payment_reference: String,
    payer_name: Option<String>,
    payer_phone: Option<String>,
    payer_bank: Option<String>,
    payer_document: Option<String>,
    payer_account: Option<String>,
    notes: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct QuoteInput {
    store_id: Uuid,
    items: Vec<OrderLineInput>,
}

#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct QuoteResponse {
    subtotal_cents: i64,
    delivery_fee_cents: i64,
    exchange_rate: Option<payment_store::ExchangeRate>,
    exchange_rate_fresh: bool,
    methods: Vec<MethodQuote>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct StatusInput {
    status: OrderStatus,
    reason: Option<String>,
}

struct PaymentContext {
    settings: PaymentSettings,
    rate: Option<payment_store::ExchangeRate>,
    fresh_rate_micros: Option<i64>,
}

async fn payment_context(pool: &PgPool, config: &AppConfig) -> ApiResult<PaymentContext> {
    let settings = payment_store::settings(pool).await?;
    let rate = payment_store::current_rate(pool).await?;
    let fresh_rate_micros = rate
        .as_ref()
        .filter(|rate| rate.is_fresh(chrono::Utc::now(), config.exchange_rate_max_age_hours))
        .map(|rate| rate.ves_per_usd_micros);
    Ok(PaymentContext { settings, rate, fresh_rate_micros })
}

impl PaymentContext {
    fn quote(&self, method: PaymentMethod, base_cents: i64) -> MethodQuote {
        payments::quote_method(
            method,
            self.settings.is_enabled(method),
            base_cents,
            self.settings.paypal_surcharge_bps,
            self.fresh_rate_micros,
        )
    }
}

async fn priced_cart(
    pool: &PgPool,
    store_id: Uuid,
    items: &[OrderLineInput],
) -> ApiResult<(Store, Vec<PricedLine>, sqlx::Transaction<'static, sqlx::Postgres>)> {
    let store = stores::find_by_id(pool, store_id)
        .await?
        .filter(|store| store.status == StoreStatus::Active)
        .ok_or_else(|| ApiError::bad_request("storeId does not reference an active store"))?;
    let requested: Vec<RequestedLine> = items
        .iter()
        .map(|line| RequestedLine { product_id: line.product_id, quantity: line.quantity })
        .collect();
    let product_ids: Vec<Uuid> = requested.iter().map(|line| line.product_id).collect();
    let mut tx = pool.begin().await?;
    let catalog = orders::load_catalog(&mut tx, &product_ids).await?;
    let lines = pricing::price_lines(store.id, &requested, &catalog)
        .map_err(|error| ApiError::bad_request(error.to_string()))?;
    Ok((store, lines, tx))
}

fn optional_field<'a>(field: &str, value: &'a Option<String>, max: usize) -> ApiResult<Option<&'a str>> {
    validation::optional_text(field, value.as_deref(), max)
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

#[post("/orders/quote")]
async fn quote(
    auth: AuthUser,
    input: web::Json<QuoteInput>,
    pool: web::Data<PgPool>,
    config: web::Data<AppConfig>,
) -> ApiResult<HttpResponse> {
    auth.require(&[UserRole::Customer])?;
    let (_, lines, tx) = priced_cart(&pool, input.store_id, &input.items).await?;
    tx.rollback().await?;
    let context = payment_context(&pool, &config).await?;
    let subtotal_cents = pricing::subtotal_cents(&lines);
    let base_cents = subtotal_cents + config.delivery_fee_cents;
    Ok(HttpResponse::Ok().json(QuoteResponse {
        subtotal_cents,
        delivery_fee_cents: config.delivery_fee_cents,
        exchange_rate_fresh: context.fresh_rate_micros.is_some(),
        methods: PaymentMethod::ALL.iter().map(|method| context.quote(*method, base_cents)).collect(),
        exchange_rate: context.rate,
    }))
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
    let payment_reference = validation::text("paymentReference", &input.payment_reference, 4, 64)?;
    let notes = validation::optional_text("notes", input.notes.as_deref(), 500)?;
    validation::coordinates(input.delivery_latitude, input.delivery_longitude)?;

    let payer = PayerDetails {
        name: optional_field("payerName", &input.payer_name, 100)?,
        phone: input.payer_phone.as_deref().map(validation::phone).transpose()?,
        bank: optional_field("payerBank", &input.payer_bank, 100)?,
        document: optional_field("payerDocument", &input.payer_document, 20)?,
        account: optional_field("payerAccount", &input.payer_account, 254)?,
    };
    if let Some(field) = payments::missing_payer_field(input.payment_method, &payer) {
        return Err(ApiError::bad_request(format!("{field} is required for this payment method")));
    }

    let context = payment_context(&pool, &config).await?;
    let (store, lines, mut tx) = priced_cart(&pool, input.store_id, &input.items).await?;
    let subtotal_cents = pricing::subtotal_cents(&lines);
    let method_quote = context.quote(input.payment_method, subtotal_cents + config.delivery_fee_cents);
    if let Some(reason) = method_quote.unavailable_reason {
        return Err(ApiError::conflict(match reason {
            payments::Unavailability::Disabled => "this payment method is not available",
            payments::Unavailability::RateUnavailable => "the exchange rate is not available; try another payment method",
        }));
    }

    let new_order = NewOrder {
        customer_id: auth.id,
        store_id: store.id,
        lines: &lines,
        subtotal_cents,
        delivery_fee_cents: config.delivery_fee_cents,
        delivery_address,
        delivery_latitude: input.delivery_latitude,
        delivery_longitude: input.delivery_longitude,
        phone,
        payment_method: input.payment_method,
        payment_reference,
        surcharge_cents: method_quote.surcharge_cents,
        exchange_rate_micros: if input.payment_method.needs_exchange_rate() { context.fresh_rate_micros } else { None },
        total_cents: method_quote.total_cents,
        total_ves_cents: method_quote.total_ves_cents,
        payer,
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
    let reason = validation::optional_text("reason", input.reason.as_deref(), 300)?;
    if reason.is_some() && next != OrderStatus::Cancelled {
        return Err(ApiError::bad_request("reason is only accepted when cancelling"));
    }
    if !orders::transition_status(&pool, id, parties.status, next, auth.id, reason).await? {
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
    cfg.service(quote)
        .service(create)
        .service(list)
        .service(available)
        .service(detail)
        .service(update_status)
        .service(assign);
}
