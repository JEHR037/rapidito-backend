use serde::{Deserialize, Serialize};

pub const BASIS_POINTS: i64 = 10_000;
pub const RATE_SCALE: i64 = 1_000_000;
pub const MAX_SURCHARGE_BPS: i32 = 5_000;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "paymentmethod")]
#[serde(rename_all = "snake_case")]
pub enum PaymentMethod {
    PagoMovil,
    BankTransfer,
    Binance,
    Paypal,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum Currency {
    Usd,
    Ves,
}

impl PaymentMethod {
    pub const ALL: [PaymentMethod; 4] = [Self::PagoMovil, Self::BankTransfer, Self::Binance, Self::Paypal];

    pub fn currency(self) -> Currency {
        match self {
            Self::PagoMovil | Self::BankTransfer => Currency::Ves,
            Self::Binance | Self::Paypal => Currency::Usd,
        }
    }

    pub fn needs_exchange_rate(self) -> bool {
        self.currency() == Currency::Ves
    }
}

pub fn surcharge_cents(base_cents: i64, surcharge_bps: i32) -> i64 {
    let numerator = i128::from(base_cents) * i128::from(surcharge_bps);
    let rounded = (numerator + i128::from(BASIS_POINTS) / 2) / i128::from(BASIS_POINTS);
    i64::try_from(rounded).unwrap_or(i64::MAX)
}

pub fn usd_to_ves_cents(usd_cents: i64, ves_per_usd_micros: i64) -> i64 {
    let numerator = i128::from(usd_cents) * i128::from(ves_per_usd_micros);
    let rounded = (numerator + i128::from(RATE_SCALE) / 2) / i128::from(RATE_SCALE);
    i64::try_from(rounded).unwrap_or(i64::MAX)
}

#[derive(Debug, PartialEq, Eq)]
pub struct InvalidRate;

pub fn parse_rate_micros(raw: &str) -> Result<i64, InvalidRate> {
    let trimmed = raw.trim().replace(',', ".");
    let (whole, fraction) = match trimmed.split_once('.') {
        Some((whole, fraction)) => (whole, fraction),
        None => (trimmed.as_str(), ""),
    };
    if whole.is_empty()
        || whole.len() > 9
        || fraction.len() > 6
        || !whole.chars().all(|c| c.is_ascii_digit())
        || !fraction.chars().all(|c| c.is_ascii_digit())
    {
        return Err(InvalidRate);
    }
    let whole: i64 = whole.parse().map_err(|_| InvalidRate)?;
    let padded = format!("{fraction:0<6}");
    let fraction: i64 = padded.parse().map_err(|_| InvalidRate)?;
    let micros = whole * RATE_SCALE + fraction;
    if micros <= 0 {
        return Err(InvalidRate);
    }
    Ok(micros)
}

pub fn rate_micros_from_f64(value: f64) -> Option<i64> {
    if !value.is_finite() || value <= 0.0 || value >= 1e9 {
        return None;
    }
    let micros = (value * RATE_SCALE as f64).round();
    if micros < 1.0 {
        return None;
    }
    Some(micros as i64)
}

#[derive(Debug, Default, Clone, Copy)]
pub struct PayerDetails<'a> {
    pub name: Option<&'a str>,
    pub phone: Option<&'a str>,
    pub bank: Option<&'a str>,
    pub document: Option<&'a str>,
    pub account: Option<&'a str>,
}

pub fn required_payer_fields(method: PaymentMethod) -> &'static [&'static str] {
    match method {
        PaymentMethod::PagoMovil => &["payerPhone", "payerBank", "payerDocument"],
        PaymentMethod::BankTransfer => &["payerBank", "payerDocument"],
        PaymentMethod::Binance | PaymentMethod::Paypal => &["payerAccount"],
    }
}

pub fn missing_payer_field(method: PaymentMethod, payer: &PayerDetails<'_>) -> Option<&'static str> {
    required_payer_fields(method).iter().copied().find(|field| {
        let value = match *field {
            "payerPhone" => payer.phone,
            "payerBank" => payer.bank,
            "payerDocument" => payer.document,
            "payerAccount" => payer.account,
            _ => payer.name,
        };
        value.map(str::trim).is_none_or(str::is_empty)
    })
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Unavailability {
    Disabled,
    RateUnavailable,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MethodQuote {
    pub method: PaymentMethod,
    pub available: bool,
    pub unavailable_reason: Option<Unavailability>,
    pub currency: Currency,
    pub surcharge_cents: i64,
    pub total_cents: i64,
    pub total_ves_cents: Option<i64>,
}

pub fn quote_method(
    method: PaymentMethod,
    enabled: bool,
    base_cents: i64,
    paypal_surcharge_bps: i32,
    fresh_rate_micros: Option<i64>,
) -> MethodQuote {
    let surcharge = if method == PaymentMethod::Paypal { surcharge_cents(base_cents, paypal_surcharge_bps) } else { 0 };
    let total = base_cents + surcharge;
    let total_ves = if method.needs_exchange_rate() { fresh_rate_micros.map(|rate| usd_to_ves_cents(total, rate)) } else { None };
    let unavailable_reason = if !enabled {
        Some(Unavailability::Disabled)
    } else if method.needs_exchange_rate() && total_ves.is_none() {
        Some(Unavailability::RateUnavailable)
    } else {
        None
    };
    MethodQuote {
        method,
        available: unavailable_reason.is_none(),
        unavailable_reason,
        currency: method.currency(),
        surcharge_cents: surcharge,
        total_cents: total,
        total_ves_cents: total_ves,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn surcharge_rounds_half_up_in_basis_points() {
        assert_eq!(surcharge_cents(1000, 800), 80);
        assert_eq!(surcharge_cents(1348, 800), 108);
        assert_eq!(surcharge_cents(1, 800), 0);
        assert_eq!(surcharge_cents(7, 800), 1);
        assert_eq!(surcharge_cents(1000, 0), 0);
    }

    #[test]
    fn converts_usd_cents_to_ves_cents() {
        assert_eq!(usd_to_ves_cents(1348, 36_500_000), 49_202);
        assert_eq!(usd_to_ves_cents(100, 301_370_000), 30_137);
        assert_eq!(usd_to_ves_cents(0, 36_500_000), 0);
    }

    #[test]
    fn parses_decimal_rates_without_floats() {
        assert_eq!(parse_rate_micros("36.5"), Ok(36_500_000));
        assert_eq!(parse_rate_micros("301,37"), Ok(301_370_000));
        assert_eq!(parse_rate_micros(" 40 "), Ok(40_000_000));
        assert_eq!(parse_rate_micros("0.000001"), Ok(1));
        assert!(parse_rate_micros("0").is_err());
        assert!(parse_rate_micros("-3").is_err());
        assert!(parse_rate_micros("1.1234567").is_err());
        assert!(parse_rate_micros("abc").is_err());
        assert!(parse_rate_micros("").is_err());
    }

    #[test]
    fn converts_provider_floats() {
        assert_eq!(rate_micros_from_f64(301.37), Some(301_370_000));
        assert_eq!(rate_micros_from_f64(0.0), None);
        assert_eq!(rate_micros_from_f64(f64::NAN), None);
    }

    #[test]
    fn requires_payer_fields_per_method() {
        let empty = PayerDetails::default();
        assert_eq!(missing_payer_field(PaymentMethod::PagoMovil, &empty), Some("payerPhone"));
        let pago_movil = PayerDetails { phone: Some("04121234567"), bank: Some("0102"), document: Some("V12345678"), ..empty };
        assert_eq!(missing_payer_field(PaymentMethod::PagoMovil, &pago_movil), None);
        assert_eq!(missing_payer_field(PaymentMethod::BankTransfer, &PayerDetails { bank: Some("0105"), ..empty }), Some("payerDocument"));
        assert_eq!(missing_payer_field(PaymentMethod::Paypal, &PayerDetails { account: Some(" "), ..empty }), Some("payerAccount"));
        assert_eq!(missing_payer_field(PaymentMethod::Binance, &PayerDetails { account: Some("user@x.com"), ..empty }), None);
    }

    #[test]
    fn quotes_each_method() {
        let paypal = quote_method(PaymentMethod::Paypal, true, 1348, 800, None);
        assert_eq!((paypal.surcharge_cents, paypal.total_cents, paypal.available), (108, 1456, true));
        assert_eq!(paypal.total_ves_cents, None);

        let pago_movil = quote_method(PaymentMethod::PagoMovil, true, 1348, 800, Some(36_500_000));
        assert_eq!((pago_movil.surcharge_cents, pago_movil.total_cents), (0, 1348));
        assert_eq!(pago_movil.total_ves_cents, Some(49_202));

        let no_rate = quote_method(PaymentMethod::BankTransfer, true, 1348, 800, None);
        assert_eq!(no_rate.unavailable_reason, Some(Unavailability::RateUnavailable));

        let disabled = quote_method(PaymentMethod::Binance, false, 1348, 800, None);
        assert_eq!(disabled.unavailable_reason, Some(Unavailability::Disabled));
    }

    #[test]
    fn currency_per_method() {
        assert!(PaymentMethod::PagoMovil.needs_exchange_rate());
        assert!(PaymentMethod::BankTransfer.needs_exchange_rate());
        assert!(!PaymentMethod::Binance.needs_exchange_rate());
        assert!(!PaymentMethod::Paypal.needs_exchange_rate());
    }
}
