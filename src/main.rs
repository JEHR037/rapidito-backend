mod http;
mod db;
mod validators;

use actix_web::{App, HttpServer, web};
use dotenv::dotenv;
use db::init_db;

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    dotenv().ok();
    env_logger::init();

    let pool = loop {
        match init_db().await {
            Ok(pool) => {
                println!("Conexión a la base de datos establecida correctamente.");
                break pool;
            },
            Err(e) => {
                println!("Error al conectar a la base de datos: {}", e);
                std::thread::sleep(std::time::Duration::from_secs(10));
            },
        };
    };

    println!("Iniciando servidor en http://127.0.0.1:8080");
    HttpServer::new(move || {
        App::new()
            .app_data(web::Data::new(pool.clone()))
            .configure(http::configure_auth)
            .configure(http::configure_products)
            .configure(http::configure_stores)
            .configure(http::configure_orders)
            .configure(http::configure_vehicles)
            .configure(http::configure_drivers)
            .configure(http::configure_users)
    })
    .bind("127.0.0.1:8080")?
    .run()
    .await
}