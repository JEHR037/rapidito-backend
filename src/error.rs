use actix_web::http::StatusCode;
use actix_web::{HttpResponse, ResponseError};
use serde_json::json;

#[derive(Debug, thiserror::Error)]
pub enum ApiError {
    #[error("{0}")]
    BadRequest(String),
    #[error("{0}")]
    Unauthorized(String),
    #[error("{0}")]
    Forbidden(String),
    #[error("{0}")]
    NotFound(String),
    #[error("{0}")]
    Conflict(String),
    #[error("internal error: {0}")]
    Internal(String),
}

pub type ApiResult<T> = Result<T, ApiError>;

impl ApiError {
    pub fn bad_request(message: impl Into<String>) -> Self {
        Self::BadRequest(message.into())
    }

    pub fn unauthorized(message: impl Into<String>) -> Self {
        Self::Unauthorized(message.into())
    }

    pub fn forbidden(message: impl Into<String>) -> Self {
        Self::Forbidden(message.into())
    }

    pub fn not_found(message: impl Into<String>) -> Self {
        Self::NotFound(message.into())
    }

    pub fn conflict(message: impl Into<String>) -> Self {
        Self::Conflict(message.into())
    }

    fn code(&self) -> &'static str {
        match self {
            Self::BadRequest(_) => "bad_request",
            Self::Unauthorized(_) => "unauthorized",
            Self::Forbidden(_) => "forbidden",
            Self::NotFound(_) => "not_found",
            Self::Conflict(_) => "conflict",
            Self::Internal(_) => "internal",
        }
    }

    fn public_message(&self) -> String {
        match self {
            Self::Internal(_) => "Unexpected server error".to_string(),
            other => other.to_string(),
        }
    }
}

impl ResponseError for ApiError {
    fn status_code(&self) -> StatusCode {
        match self {
            Self::BadRequest(_) => StatusCode::BAD_REQUEST,
            Self::Unauthorized(_) => StatusCode::UNAUTHORIZED,
            Self::Forbidden(_) => StatusCode::FORBIDDEN,
            Self::NotFound(_) => StatusCode::NOT_FOUND,
            Self::Conflict(_) => StatusCode::CONFLICT,
            Self::Internal(_) => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }

    fn error_response(&self) -> HttpResponse {
        if let Self::Internal(detail) = self {
            log::error!("{detail}");
        }
        HttpResponse::build(self.status_code()).json(json!({
            "error": { "code": self.code(), "message": self.public_message() }
        }))
    }
}

impl From<sqlx::Error> for ApiError {
    fn from(error: sqlx::Error) -> Self {
        if let sqlx::Error::Database(db_error) = &error {
            if db_error.is_unique_violation() {
                return Self::conflict("Resource already exists");
            }
            if db_error.is_foreign_key_violation() {
                return Self::bad_request("Referenced resource does not exist");
            }
            if db_error.is_check_violation() {
                return Self::bad_request("Value out of range");
            }
        }
        Self::Internal(error.to_string())
    }
}

impl From<bcrypt::BcryptError> for ApiError {
    fn from(error: bcrypt::BcryptError) -> Self {
        Self::Internal(error.to_string())
    }
}

impl From<jsonwebtoken::errors::Error> for ApiError {
    fn from(error: jsonwebtoken::errors::Error) -> Self {
        Self::Internal(error.to_string())
    }
}

pub fn json_error_handler(
    error: actix_web::error::JsonPayloadError,
    _request: &actix_web::HttpRequest,
) -> actix_web::Error {
    ApiError::bad_request(format!("Invalid JSON body: {error}")).into()
}

pub fn query_error_handler(
    error: actix_web::error::QueryPayloadError,
    _request: &actix_web::HttpRequest,
) -> actix_web::Error {
    ApiError::bad_request(format!("Invalid query string: {error}")).into()
}

pub fn path_error_handler(
    error: actix_web::error::PathError,
    _request: &actix_web::HttpRequest,
) -> actix_web::Error {
    ApiError::bad_request(format!("Invalid path parameter: {error}")).into()
}
