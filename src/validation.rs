use crate::error::{ApiError, ApiResult};

pub fn text<'a>(field: &str, value: &'a str, min: usize, max: usize) -> ApiResult<&'a str> {
    let trimmed = value.trim();
    let length = trimmed.chars().count();
    if length < min || length > max {
        return Err(ApiError::bad_request(format!("{field} must be between {min} and {max} characters")));
    }
    Ok(trimmed)
}

pub fn optional_text<'a>(field: &str, value: Option<&'a str>, max: usize) -> ApiResult<Option<&'a str>> {
    match value.map(str::trim) {
        None | Some("") => Ok(None),
        Some(trimmed) => text(field, trimmed, 1, max).map(Some),
    }
}

pub fn email(value: &str) -> ApiResult<String> {
    let normalized = value.trim().to_lowercase();
    let valid = normalized.len() <= 254
        && !normalized.contains(char::is_whitespace)
        && normalized
            .split_once('@')
            .is_some_and(|(local, domain)| !local.is_empty() && domain.contains('.') && !domain.starts_with('.') && !domain.ends_with('.'));
    if !valid {
        return Err(ApiError::bad_request("email is not valid"));
    }
    Ok(normalized)
}

pub fn password(value: &str) -> ApiResult<&str> {
    let length = value.chars().count();
    let strong = (8..=128).contains(&length)
        && value.chars().any(char::is_uppercase)
        && value.chars().any(char::is_lowercase)
        && value.chars().any(|c| c.is_ascii_digit());
    if !strong {
        return Err(ApiError::bad_request(
            "password must have 8 to 128 characters with at least one uppercase letter, one lowercase letter and one digit",
        ));
    }
    Ok(value)
}

pub fn phone(value: &str) -> ApiResult<&str> {
    let trimmed = value.trim();
    let digits = trimmed.chars().filter(char::is_ascii_digit).count();
    let allowed = trimmed.chars().all(|c| c.is_ascii_digit() || matches!(c, '+' | ' ' | '-' | '(' | ')'));
    if !allowed || !(7..=15).contains(&digits) {
        return Err(ApiError::bad_request("phone is not valid"));
    }
    Ok(trimmed)
}

pub fn color(field: &str, value: Option<&str>) -> ApiResult<Option<String>> {
    let Some(raw) = value.map(str::trim).filter(|raw| !raw.is_empty()) else {
        return Ok(None);
    };
    let hex = raw.strip_prefix('#').unwrap_or_default();
    if !matches!(hex.len(), 3 | 6 | 8) || !hex.chars().all(|c| c.is_ascii_hexdigit()) {
        return Err(ApiError::bad_request(format!("{field} must be a hex color such as #1E88E5")));
    }
    Ok(Some(raw.to_uppercase()))
}

pub fn coordinates(latitude: Option<f64>, longitude: Option<f64>) -> ApiResult<()> {
    match (latitude, longitude) {
        (None, None) => Ok(()),
        (Some(lat), Some(lng)) if (-90.0..=90.0).contains(&lat) && (-180.0..=180.0).contains(&lng) => Ok(()),
        _ => Err(ApiError::bad_request("latitude and longitude must be provided together and be in range")),
    }
}

pub fn cents(field: &str, value: i64) -> ApiResult<i64> {
    if !(0..=100_000_000).contains(&value) {
        return Err(ApiError::bad_request(format!("{field} must be between 0 and 100000000")));
    }
    Ok(value)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalizes_and_validates_email() {
        assert_eq!(email("  Ana@Example.COM ").ok().as_deref(), Some("ana@example.com"));
        assert!(email("ana@example").is_err());
        assert!(email("ana example@x.com").is_err());
        assert!(email("@x.com").is_err());
    }

    #[test]
    fn enforces_password_strength() {
        assert!(password("Secret123").is_ok());
        assert!(password("secret123").is_err());
        assert!(password("SECRET123").is_err());
        assert!(password("SecretPass").is_err());
        assert!(password("Se1").is_err());
    }

    #[test]
    fn validates_colors_and_coordinates() {
        assert_eq!(color("primaryColor", Some("#ffc72c")).ok().flatten().as_deref(), Some("#FFC72C"));
        assert!(color("primaryColor", Some("red")).is_err());
        assert!(coordinates(Some(10.5), Some(-66.9)).is_ok());
        assert!(coordinates(Some(10.5), None).is_err());
        assert!(coordinates(Some(100.0), Some(0.0)).is_err());
    }

    #[test]
    fn validates_phone() {
        assert!(phone("+58 412-1234567").is_ok());
        assert!(phone("12").is_err());
        assert!(phone("call me").is_err());
    }
}
