pub mod auth;
pub mod stores;
pub mod products;
pub mod orders;
 
pub use auth::configure as configure_auth; 
pub use stores::configure as configure_stores;
pub use products::configure as configure_products;
pub use orders::configure as configure_orders;