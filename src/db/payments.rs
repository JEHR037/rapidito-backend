use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, PgPool};
use uuid::Uuid;

use crate::payments::PaymentMethod;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "ratesource")]
#[serde(rename_all = "snake_case")]
pub enum RateSource {
    Provider,
    Manual,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow, Default)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PaymentSettings {
    pub pago_movil_enabled: bool,
    pub pago_movil_bank: Option<String>,
    pub pago_movil_phone: Option<String>,
    pub pago_movil_document: Option<String>,
    pub bank_transfer_enabled: bool,
    pub bank_name: Option<String>,
    pub bank_account_number: Option<String>,
    pub bank_account_holder: Option<String>,
    pub bank_account_document: Option<String>,
    pub bank_account_type: Option<String>,
    pub binance_enabled: bool,
    pub binance_pay_id: Option<String>,
    pub binance_email: Option<String>,
    pub paypal_enabled: bool,
    pub paypal_email: Option<String>,
    pub paypal_surcharge_bps: i32,
}

fn filled(value: &Option<String>) -> bool {
    value.as_deref().is_some_and(|text| !text.trim().is_empty())
}

impl PaymentSettings {
    pub fn is_enabled(&self, method: PaymentMethod) -> bool {
        match method {
            PaymentMethod::PagoMovil => {
                self.pago_movil_enabled
                    && filled(&self.pago_movil_bank)
                    && filled(&self.pago_movil_phone)
                    && filled(&self.pago_movil_document)
            }
            PaymentMethod::BankTransfer => {
                self.bank_transfer_enabled
                    && filled(&self.bank_name)
                    && filled(&self.bank_account_number)
                    && filled(&self.bank_account_holder)
                    && filled(&self.bank_account_document)
            }
            PaymentMethod::Binance => self.binance_enabled && (filled(&self.binance_pay_id) || filled(&self.binance_email)),
            PaymentMethod::Paypal => self.paypal_enabled && filled(&self.paypal_email),
        }
    }
}

#[derive(Debug, Clone, Serialize, FromRow)]
#[serde(rename_all = "camelCase")]
pub struct ExchangeRate {
    pub ves_per_usd_micros: i64,
    pub source: RateSource,
    pub published_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub checked_at: DateTime<Utc>,
}

impl ExchangeRate {
    pub fn last_confirmed_at(&self) -> DateTime<Utc> {
        match self.source {
            RateSource::Provider => self.checked_at,
            RateSource::Manual => self.created_at,
        }
    }

    pub fn is_fresh(&self, now: DateTime<Utc>, max_age_hours: i64) -> bool {
        now - self.last_confirmed_at() <= chrono::Duration::hours(max_age_hours)
    }
}

const SETTINGS_COLUMNS: &str = "pago_movil_enabled, pago_movil_bank, pago_movil_phone, pago_movil_document, \
     bank_transfer_enabled, bank_name, bank_account_number, bank_account_holder, bank_account_document, \
     bank_account_type, binance_enabled, binance_pay_id, binance_email, paypal_enabled, paypal_email, \
     paypal_surcharge_bps";

const RATE_COLUMNS: &str = "ves_per_usd_micros, source, published_at, created_at, checked_at";

pub async fn settings(pool: &PgPool) -> sqlx::Result<PaymentSettings> {
    sqlx::query_as::<_, PaymentSettings>(&format!("SELECT {SETTINGS_COLUMNS} FROM payment_settings WHERE id = 1"))
        .fetch_one(pool)
        .await
}

pub async fn update_settings(pool: &PgPool, settings: &PaymentSettings, updated_by: Uuid) -> sqlx::Result<PaymentSettings> {
    sqlx::query_as::<_, PaymentSettings>(&format!(
        "UPDATE payment_settings SET \
            pago_movil_enabled = $1, pago_movil_bank = $2, pago_movil_phone = $3, pago_movil_document = $4, \
            bank_transfer_enabled = $5, bank_name = $6, bank_account_number = $7, bank_account_holder = $8, \
            bank_account_document = $9, bank_account_type = $10, binance_enabled = $11, binance_pay_id = $12, \
            binance_email = $13, paypal_enabled = $14, paypal_email = $15, paypal_surcharge_bps = $16, \
            updated_at = CURRENT_TIMESTAMP, updated_by = $17 \
         WHERE id = 1 RETURNING {SETTINGS_COLUMNS}"
    ))
    .bind(settings.pago_movil_enabled)
    .bind(&settings.pago_movil_bank)
    .bind(&settings.pago_movil_phone)
    .bind(&settings.pago_movil_document)
    .bind(settings.bank_transfer_enabled)
    .bind(&settings.bank_name)
    .bind(&settings.bank_account_number)
    .bind(&settings.bank_account_holder)
    .bind(&settings.bank_account_document)
    .bind(&settings.bank_account_type)
    .bind(settings.binance_enabled)
    .bind(&settings.binance_pay_id)
    .bind(&settings.binance_email)
    .bind(settings.paypal_enabled)
    .bind(&settings.paypal_email)
    .bind(settings.paypal_surcharge_bps)
    .bind(updated_by)
    .fetch_one(pool)
    .await
}

pub async fn current_rate(pool: &PgPool) -> sqlx::Result<Option<ExchangeRate>> {
    sqlx::query_as::<_, ExchangeRate>(&format!(
        "SELECT {RATE_COLUMNS} FROM exchange_rates ORDER BY created_at DESC, id DESC LIMIT 1"
    ))
    .fetch_optional(pool)
    .await
}

pub async fn insert_manual_rate(pool: &PgPool, micros: i64, created_by: Uuid) -> sqlx::Result<ExchangeRate> {
    sqlx::query_as::<_, ExchangeRate>(&format!(
        "INSERT INTO exchange_rates (ves_per_usd_micros, source, created_by) VALUES ($1, 'Manual', $2) RETURNING {RATE_COLUMNS}"
    ))
    .bind(micros)
    .bind(created_by)
    .fetch_one(pool)
    .await
}

pub async fn record_provider_rate(
    pool: &PgPool,
    micros: i64,
    published_at: Option<DateTime<Utc>>,
) -> sqlx::Result<bool> {
    let mut tx = pool.begin().await?;
    let latest: Option<(i64, i64, Option<DateTime<Utc>>)> = sqlx::query_as(
        "SELECT id, ves_per_usd_micros, published_at FROM exchange_rates \
         WHERE source = 'Provider' ORDER BY created_at DESC, id DESC LIMIT 1 FOR UPDATE",
    )
    .fetch_optional(&mut *tx)
    .await?;

    let inserted = match latest {
        Some((id, latest_micros, latest_published)) if latest_micros == micros && latest_published == published_at => {
            sqlx::query("UPDATE exchange_rates SET checked_at = CURRENT_TIMESTAMP WHERE id = $1")
                .bind(id)
                .execute(&mut *tx)
                .await?;
            false
        }
        _ => {
            sqlx::query("INSERT INTO exchange_rates (ves_per_usd_micros, source, published_at) VALUES ($1, 'Provider', $2)")
                .bind(micros)
                .bind(published_at)
                .execute(&mut *tx)
                .await?;
            true
        }
    };

    tx.commit().await?;
    Ok(inserted)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rate(source: RateSource, created_hours_ago: i64, checked_hours_ago: i64) -> ExchangeRate {
        let now = Utc::now();
        ExchangeRate {
            ves_per_usd_micros: 36_500_000,
            source,
            published_at: None,
            created_at: now - chrono::Duration::hours(created_hours_ago),
            checked_at: now - chrono::Duration::hours(checked_hours_ago),
        }
    }

    #[test]
    fn methods_need_their_receiving_account() {
        let mut settings = PaymentSettings { paypal_enabled: true, ..PaymentSettings::default() };
        assert!(!settings.is_enabled(PaymentMethod::Paypal));
        settings.paypal_email = Some("pagos@rapidito.app".to_string());
        assert!(settings.is_enabled(PaymentMethod::Paypal));
        settings.binance_enabled = true;
        settings.binance_pay_id = Some("  ".to_string());
        assert!(!settings.is_enabled(PaymentMethod::Binance));
        settings.binance_email = Some("binance@rapidito.app".to_string());
        assert!(settings.is_enabled(PaymentMethod::Binance));
        assert!(!settings.is_enabled(PaymentMethod::PagoMovil));
    }

    #[test]
    fn provider_freshness_follows_last_check() {
        assert!(rate(RateSource::Provider, 100, 1).is_fresh(Utc::now(), 24));
        assert!(!rate(RateSource::Provider, 100, 30).is_fresh(Utc::now(), 24));
    }

    #[test]
    fn manual_freshness_follows_creation() {
        assert!(rate(RateSource::Manual, 2, 2).is_fresh(Utc::now(), 24));
        assert!(!rate(RateSource::Manual, 30, 30).is_fresh(Utc::now(), 24));
    }
}
