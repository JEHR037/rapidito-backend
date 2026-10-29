use actix_web::{get, post, put, web, HttpResponse};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;

use crate::auth::AuthUser;
use crate::config::AppConfig;
use crate::db::payments::{self, ExchangeRate, PaymentSettings};
use crate::domain::UserRole;
use crate::error::{ApiError, ApiResult};
use crate::payments::{parse_rate_micros, Currency, PaymentMethod, MAX_SURCHARGE_BPS};
use crate::validation;

#[derive(Debug, Serialize)]
#[serde(tag = "method", rename_all = "snake_case")]
enum Instructions {
    PagoMovil { bank: String, phone: String, document: String },
    BankTransfer { bank: String, account_number: String, account_holder: String, document: String, account_type: Option<String> },
    Binance { pay_id: Option<String>, email: Option<String> },
    Paypal { email: String },
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct MethodOption {
    currency: Currency,
    surcharge_bps: i32,
    #[serde(flatten)]
    instructions: Instructions,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct RateResponse {
    rate: Option<ExchangeRate>,
    fresh: bool,
    max_age_hours: i64,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct ManualRateInput {
    ves_per_usd: String,
}

fn instructions(settings: &PaymentSettings, method: PaymentMethod) -> Option<Instructions> {
    if !settings.is_enabled(method) {
        return None;
    }
    let text = |value: &Option<String>| value.clone().unwrap_or_default();
    Some(match method {
        PaymentMethod::PagoMovil => Instructions::PagoMovil {
            bank: text(&settings.pago_movil_bank),
            phone: text(&settings.pago_movil_phone),
            document: text(&settings.pago_movil_document),
        },
        PaymentMethod::BankTransfer => Instructions::BankTransfer {
            bank: text(&settings.bank_name),
            account_number: text(&settings.bank_account_number),
            account_holder: text(&settings.bank_account_holder),
            document: text(&settings.bank_account_document),
            account_type: settings.bank_account_type.clone(),
        },
        PaymentMethod::Binance => Instructions::Binance {
            pay_id: settings.binance_pay_id.clone().filter(|value| !value.trim().is_empty()),
            email: settings.binance_email.clone().filter(|value| !value.trim().is_empty()),
        },
        PaymentMethod::Paypal => Instructions::Paypal { email: text(&settings.paypal_email) },
    })
}

async fn rate_response(pool: &PgPool, config: &AppConfig) -> ApiResult<RateResponse> {
    let rate = payments::current_rate(pool).await?;
    let fresh = rate.as_ref().is_some_and(|rate| rate.is_fresh(chrono::Utc::now(), config.exchange_rate_max_age_hours));
    Ok(RateResponse { rate, fresh, max_age_hours: config.exchange_rate_max_age_hours })
}

fn clean(field: &str, value: &Option<String>, max: usize) -> ApiResult<Option<String>> {
    Ok(validation::optional_text(field, value.as_deref(), max)?.map(str::to_string))
}

fn validated(input: &PaymentSettings) -> ApiResult<PaymentSettings> {
    if !(0..=MAX_SURCHARGE_BPS).contains(&input.paypal_surcharge_bps) {
        return Err(ApiError::bad_request(format!("paypalSurchargeBps must be between 0 and {MAX_SURCHARGE_BPS}")));
    }
    let settings = PaymentSettings {
        pago_movil_enabled: input.pago_movil_enabled,
        pago_movil_bank: clean("pagoMovilBank", &input.pago_movil_bank, 100)?,
        pago_movil_phone: input.pago_movil_phone.as_deref().filter(|value| !value.trim().is_empty()).map(validation::phone).transpose()?.map(str::to_string),
        pago_movil_document: clean("pagoMovilDocument", &input.pago_movil_document, 20)?,
        bank_transfer_enabled: input.bank_transfer_enabled,
        bank_name: clean("bankName", &input.bank_name, 100)?,
        bank_account_number: clean("bankAccountNumber", &input.bank_account_number, 30)?,
        bank_account_holder: clean("bankAccountHolder", &input.bank_account_holder, 120)?,
        bank_account_document: clean("bankAccountDocument", &input.bank_account_document, 20)?,
        bank_account_type: clean("bankAccountType", &input.bank_account_type, 30)?,
        binance_enabled: input.binance_enabled,
        binance_pay_id: clean("binancePayId", &input.binance_pay_id, 64)?,
        binance_email: input.binance_email.as_deref().filter(|value| !value.trim().is_empty()).map(validation::email).transpose()?,
        paypal_enabled: input.paypal_enabled,
        paypal_email: input.paypal_email.as_deref().filter(|value| !value.trim().is_empty()).map(validation::email).transpose()?,
        paypal_surcharge_bps: input.paypal_surcharge_bps,
    };
    let incomplete = PaymentMethod::ALL.iter().find(|method| {
        let requested = match method {
            PaymentMethod::PagoMovil => settings.pago_movil_enabled,
            PaymentMethod::BankTransfer => settings.bank_transfer_enabled,
            PaymentMethod::Binance => settings.binance_enabled,
            PaymentMethod::Paypal => settings.paypal_enabled,
        };
        requested && !settings.is_enabled(**method)
    });
    if let Some(method) = incomplete {
        return Err(ApiError::bad_request(format!("{method:?} is enabled but its receiving account is incomplete")));
    }
    Ok(settings)
}

#[get("/payments/methods")]
async fn methods(_auth: AuthUser, pool: web::Data<PgPool>) -> ApiResult<HttpResponse> {
    let settings = payments::settings(&pool).await?;
    let options: Vec<MethodOption> = PaymentMethod::ALL
        .iter()
        .filter_map(|method| {
            instructions(&settings, *method).map(|instructions| MethodOption {
                currency: method.currency(),
                surcharge_bps: if *method == PaymentMethod::Paypal { settings.paypal_surcharge_bps } else { 0 },
                instructions,
            })
        })
        .collect();
    Ok(HttpResponse::Ok().json(options))
}

#[get("/exchange-rate")]
async fn exchange_rate(pool: web::Data<PgPool>, config: web::Data<AppConfig>) -> ApiResult<HttpResponse> {
    Ok(HttpResponse::Ok().json(rate_response(&pool, &config).await?))
}

#[get("/admin/payment-settings")]
async fn get_settings(auth: AuthUser, pool: web::Data<PgPool>) -> ApiResult<HttpResponse> {
    auth.require(&[UserRole::Admin])?;
    Ok(HttpResponse::Ok().json(payments::settings(&pool).await?))
}

#[put("/admin/payment-settings")]
async fn update_settings(auth: AuthUser, input: web::Json<PaymentSettings>, pool: web::Data<PgPool>) -> ApiResult<HttpResponse> {
    auth.require(&[UserRole::Admin])?;
    let settings = validated(&input)?;
    Ok(HttpResponse::Ok().json(payments::update_settings(&pool, &settings, auth.id).await?))
}

#[post("/admin/exchange-rate")]
async fn set_manual_rate(
    auth: AuthUser,
    input: web::Json<ManualRateInput>,
    pool: web::Data<PgPool>,
    config: web::Data<AppConfig>,
) -> ApiResult<HttpResponse> {
    auth.require(&[UserRole::Admin])?;
    let micros = parse_rate_micros(&input.ves_per_usd)
        .map_err(|_| ApiError::bad_request("vesPerUsd must be a positive decimal with up to 6 decimals"))?;
    payments::insert_manual_rate(&pool, micros, auth.id).await?;
    Ok(HttpResponse::Created().json(rate_response(&pool, &config).await?))
}

pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.service(methods)
        .service(exchange_rate)
        .service(get_settings)
        .service(update_settings)
        .service(set_manual_rate);
}
