use std::env;

#[derive(Debug, Clone)]
pub struct AppConfig {
    pub server_addr: String,
    pub database_url: String,
    pub database_max_connections: u32,
    pub jwt_secret: String,
    pub jwt_ttl_hours: i64,
    pub cors_allowed_origins: Vec<String>,
    pub delivery_fee_cents: i64,
}

#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    #[error("missing required environment variable {0}")]
    Missing(&'static str),
    #[error("invalid value for environment variable {0}")]
    Invalid(&'static str),
}

const MIN_JWT_SECRET_LEN: usize = 32;

impl AppConfig {
    pub fn from_env() -> Result<Self, ConfigError> {
        let jwt_secret = required("JWT_SECRET")?;
        if jwt_secret.len() < MIN_JWT_SECRET_LEN {
            return Err(ConfigError::Invalid("JWT_SECRET"));
        }

        Ok(Self {
            server_addr: optional("SERVER_ADDR").unwrap_or_else(|| "127.0.0.1:8080".to_string()),
            database_url: required("DATABASE_URL")?,
            database_max_connections: parsed("DATABASE_MAX_CONNECTIONS", 10)?,
            jwt_secret,
            jwt_ttl_hours: parse_duration_hours(optional("JWT_EXPIRATION").as_deref())?,
            cors_allowed_origins: optional("CORS_ALLOWED_ORIGINS")
                .map(|raw| {
                    raw.split(',')
                        .map(|origin| origin.trim().to_string())
                        .filter(|origin| !origin.is_empty())
                        .collect()
                })
                .unwrap_or_default(),
            delivery_fee_cents: parsed("DELIVERY_FEE_CENTS", 0)?,
        })
    }
}

fn optional(key: &'static str) -> Option<String> {
    env::var(key).ok().map(|value| value.trim().to_string()).filter(|value| !value.is_empty())
}

fn required(key: &'static str) -> Result<String, ConfigError> {
    optional(key).ok_or(ConfigError::Missing(key))
}

fn parsed<T: std::str::FromStr>(key: &'static str, default: T) -> Result<T, ConfigError> {
    match optional(key) {
        Some(raw) => raw.parse().map_err(|_| ConfigError::Invalid(key)),
        None => Ok(default),
    }
}

fn parse_duration_hours(raw: Option<&str>) -> Result<i64, ConfigError> {
    let Some(raw) = raw else {
        return Ok(24);
    };
    let invalid = ConfigError::Invalid("JWT_EXPIRATION");
    let (amount, unit) = raw.split_at(raw.len() - 1);
    let hours = match unit {
        "h" => amount.parse::<i64>().map_err(|_| invalid)?,
        "d" => amount.parse::<i64>().map_err(|_| invalid)? * 24,
        _ => raw.parse::<i64>().map_err(|_| invalid)?,
    };
    if hours <= 0 {
        return Err(ConfigError::Invalid("JWT_EXPIRATION"));
    }
    Ok(hours)
}

#[cfg(test)]
mod tests {
    use super::parse_duration_hours;

    #[test]
    fn parses_expiration_units() {
        assert_eq!(parse_duration_hours(None).ok(), Some(24));
        assert_eq!(parse_duration_hours(Some("12h")).ok(), Some(12));
        assert_eq!(parse_duration_hours(Some("2d")).ok(), Some(48));
        assert_eq!(parse_duration_hours(Some("6")).ok(), Some(6));
        assert!(parse_duration_hours(Some("0h")).is_err());
        assert!(parse_duration_hours(Some("abc")).is_err());
    }
}
