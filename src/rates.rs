use std::time::Duration;

use chrono::{DateTime, Utc};
use serde::Deserialize;
use sqlx::PgPool;

use crate::db::payments;
use crate::payments::rate_micros_from_f64;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ProviderRate {
    promedio: Option<f64>,
    venta: Option<f64>,
    fecha_actualizacion: Option<String>,
}

#[derive(Debug, PartialEq, Eq)]
struct ParsedRate {
    micros: i64,
    published_at: Option<DateTime<Utc>>,
}

fn parse_provider(body: &str) -> Option<ParsedRate> {
    let rate: ProviderRate = serde_json::from_str(body).ok()?;
    let micros = rate.promedio.and_then(rate_micros_from_f64).or_else(|| rate.venta.and_then(rate_micros_from_f64))?;
    let published_at = rate
        .fecha_actualizacion
        .as_deref()
        .and_then(|raw| DateTime::parse_from_rfc3339(raw).ok())
        .map(|value| value.with_timezone(&Utc));
    Some(ParsedRate { micros, published_at })
}

async fn refresh_once(client: &reqwest::Client, pool: &PgPool, url: &str) -> Result<bool, String> {
    let body = client
        .get(url)
        .send()
        .await
        .and_then(reqwest::Response::error_for_status)
        .map_err(|error| format!("request failed: {error}"))?
        .text()
        .await
        .map_err(|error| format!("unreadable body: {error}"))?;
    let parsed = parse_provider(&body).ok_or_else(|| "unexpected payload".to_string())?;
    payments::record_provider_rate(pool, parsed.micros, parsed.published_at)
        .await
        .map_err(|error| format!("database error: {error}"))
}

pub fn spawn_refresher(pool: PgPool, url: String, every: Duration) {
    tokio::spawn(async move {
        let client = match reqwest::Client::builder().timeout(Duration::from_secs(15)).build() {
            Ok(client) => client,
            Err(error) => {
                log::error!("exchange rate refresher disabled: {error}");
                return;
            }
        };
        let mut interval = tokio::time::interval(every);
        loop {
            interval.tick().await;
            match refresh_once(&client, &pool, &url).await {
                Ok(true) => log::info!("exchange rate updated from provider"),
                Ok(false) => log::debug!("exchange rate unchanged"),
                Err(error) => log::warn!("exchange rate refresh failed: {error}"),
            }
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_promedio_then_venta() {
        let parsed = parse_provider(
            r#"{"fuente":"oficial","nombre":"Oficial","compra":null,"venta":null,"promedio":301.37,"fechaActualizacion":"2026-01-01T00:00:00.000Z"}"#,
        );
        assert_eq!(parsed.as_ref().map(|rate| rate.micros), Some(301_370_000));
        assert!(parsed.and_then(|rate| rate.published_at).is_some());

        let fallback = parse_provider(r#"{"promedio":null,"venta":36.5}"#);
        assert_eq!(fallback.map(|rate| rate.micros), Some(36_500_000));
    }

    #[test]
    fn rejects_unusable_payloads() {
        assert_eq!(parse_provider(r#"{"promedio":null,"venta":null}"#), None);
        assert_eq!(parse_provider(r#"{"promedio":0}"#), None);
        assert_eq!(parse_provider("not json"), None);
    }
}
