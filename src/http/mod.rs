mod admin;
mod auth;
mod drivers;
mod health;
mod orders;
mod payments;
mod products;
mod stores;
mod users;

use actix_web::{web, HttpResponse};

use crate::error::ApiError;

pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/api")
            .configure(health::configure)
            .configure(auth::configure)
            .configure(users::configure)
            .configure(stores::configure)
            .configure(products::configure)
            .configure(orders::configure)
            .configure(payments::configure)
            .configure(drivers::configure)
            .configure(admin::configure),
    );
}

pub async fn route_not_found() -> Result<HttpResponse, ApiError> {
    Err(ApiError::not_found("route not found"))
}
