use dotenv::dotenv;
use std::env;

// Inicializa la configuración cargando el archivo .env
pub fn init() {
    dotenv().ok();
    
    // Verificar variables de entorno críticas
    if env::var("DATABASE_URL").is_err() {
        eprintln!("Error: DATABASE_URL no está configurada. Por favor, configure su archivo .env");
    }
}

pub fn get_server_addr() -> String {
    env::var("SERVER_ADDR").unwrap_or_else(|_| "127.0.0.1:8080".to_string())
}

pub fn get_server_env() -> String {
    env::var("SERVER_ENV").unwrap_or_else(|_| "development".to_string())
}

// Configuración PostgreSQL
pub fn get_database_url() -> String {
    env::var("DATABASE_URL").expect("DATABASE_URL debe estar configurada")
}

pub fn get_database_max_connections() -> u32 {
    env::var("DATABASE_MAX_CONNECTIONS")
        .unwrap_or_else(|_| "10".to_string())
        .parse()
        .unwrap_or(10)
}

pub fn get_database_timeout_seconds() -> u64 {
    env::var("DATABASE_TIMEOUT_SECONDS")
        .unwrap_or_else(|_| "5".to_string())
        .parse()
        .unwrap_or(5)
}

// Configuración Redis
pub fn get_redis_url() -> String {
    env::var("REDIS_URL").unwrap_or_else(|_| "redis://127.0.0.1:6379".to_string())
}

// Autenticación y seguridad
pub fn get_jwt_secret() -> String {
    env::var("JWT_SECRET").expect("JWT_SECRET debe estar configurada")
}

pub fn get_jwt_expiration() -> String {
    env::var("JWT_EXPIRATION").unwrap_or_else(|_| "24h".to_string())
}

// Configuración de logging
pub fn get_log_level() -> String {
    env::var("LOG_LEVEL").unwrap_or_else(|_| "info".to_string())
}

// Configuración de CORS
pub fn get_cors_allowed_origins() -> Vec<String> {
    env::var("CORS_ALLOWED_ORIGINS")
        .unwrap_or_else(|_| "http://localhost:3000".to_string())
        .split(',')
        .map(|s| s.to_string())
        .collect()
}

// Timeouts de WebSocket
pub fn get_websocket_heartbeat_interval() -> u64 {
    env::var("WEBSOCKET_HEARTBEAT_INTERVAL")
        .unwrap_or_else(|_| "5".to_string())
        .parse()
        .unwrap_or(5)
}

pub fn get_websocket_client_timeout() -> u64 {
    env::var("WEBSOCKET_CLIENT_TIMEOUT")
        .unwrap_or_else(|_| "10".to_string())
        .parse()
        .unwrap_or(10)
}

// Configuración del Pool Manager
pub fn get_pool_rebalance_interval() -> u64 {
    env::var("POOL_REBALANCE_INTERVAL")
        .unwrap_or_else(|_| "5".to_string())
        .parse()
        .unwrap_or(5)
}

pub fn get_target_clients_per_pool() -> usize {
    env::var("TARGET_CLIENTS_PER_POOL")
        .unwrap_or_else(|_| "33".to_string())
        .parse()
        .unwrap_or(33)
}

// Configuración de almacenamiento
pub fn get_storage_type() -> String {
    env::var("STORAGE_TYPE").unwrap_or_else(|_| "local".to_string())
}

pub fn get_storage_local_path() -> String {
    env::var("STORAGE_LOCAL_PATH").unwrap_or_else(|_| "./uploads".to_string())
}