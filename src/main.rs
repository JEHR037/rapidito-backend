mod auth;
mod config;
mod db;
mod domain;
mod error;
mod http;
mod payments;
mod pricing;
mod rates;
mod validation;

use std::time::Duration;

use actix_cors::Cors;
use actix_web::middleware::{Logger, NormalizePath, TrailingSlash};
use actix_web::{http::header, web, App, HttpServer};
use sqlx::postgres::PgPoolOptions;
use sqlx::PgPool;

use crate::config::AppConfig;

const DB_RETRY_DELAY: Duration = Duration::from_secs(10);

async fn connect(config: &AppConfig) -> PgPool {
    loop {
        match PgPoolOptions::new()
            .max_connections(config.database_max_connections)
            .acquire_timeout(Duration::from_secs(5))
            .connect(&config.database_url)
            .await
        {
            Ok(pool) => return pool,
            Err(error) => {
                log::error!("database connection failed, retrying in {}s: {error}", DB_RETRY_DELAY.as_secs());
                tokio::time::sleep(DB_RETRY_DELAY).await;
            }
        }
    }
}

fn cors(origins: &[String]) -> Cors {
    let cors = Cors::default()
        .allowed_methods(["GET", "POST", "PUT", "PATCH", "DELETE", "OPTIONS"])
        .allowed_headers([header::AUTHORIZATION, header::CONTENT_TYPE, header::ACCEPT])
        .max_age(3600);
    origins.iter().fold(cors, |cors, origin| cors.allowed_origin(origin))
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    dotenvy::dotenv().ok();
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    let config = AppConfig::from_env().map_err(std::io::Error::other)?;
    let pool = connect(&config).await;
    sqlx::migrate!("./migrations").run(&pool).await.map_err(std::io::Error::other)?;
    log::info!("database ready, listening on {}", config.server_addr);

    match &config.exchange_rate_url {
        Some(url) => rates::spawn_refresher(
            pool.clone(),
            url.clone(),
            Duration::from_secs(config.exchange_rate_refresh_minutes * 60),
        ),
        None => log::warn!("EXCHANGE_RATE_URL is empty; the exchange rate must be set manually"),
    }

    let address = config.server_addr.clone();
    let config = web::Data::new(config);
    let pool = web::Data::new(pool);

    HttpServer::new(move || {
        App::new()
            .app_data(config.clone())
            .app_data(pool.clone())
            .app_data(web::JsonConfig::default().limit(64 * 1024).error_handler(error::json_error_handler))
            .app_data(web::QueryConfig::default().error_handler(error::query_error_handler))
            .app_data(web::PathConfig::default().error_handler(error::path_error_handler))
            .wrap(cors(&config.cors_allowed_origins))
            .wrap(NormalizePath::new(TrailingSlash::Trim))
            .wrap(Logger::new("%r %s %Dms"))
            .configure(http::configure)
            .default_service(web::to(http::route_not_found))
    })
    .bind(address)?
    .run()
    .await
}
