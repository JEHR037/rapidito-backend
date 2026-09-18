# Rapidito Backend

Order management API for vendors and delivery, written in Rust on Actix-web with PostgreSQL
through sqlx. It covers accounts, stores, products, orders, drivers and vehicles, with JWT
authentication and bcrypt password hashing.

## Endpoints

Route groups are registered in `src/main.rs`:

| Group     | Module              |
| --------- | ------------------- |
| Auth      | `src/http/auth.rs`  |
| Users     | `src/http/users.rs` |
| Stores    | `src/http/stores.rs` |
| Products  | `src/http/products.rs` |
| Orders    | `src/http/orders.rs` |
| Drivers   | `src/http/drivers.rs` |
| Vehicles  | `src/http/vehicles.rs` |

The database layer (`src/db/`) adds addresses and payments on top of those. `Swagger.json`
at the root documents the HTTP surface.

A WebSocket layer (`src/websocket/`, with rooms, client pools and per-topic channels for
home, notifications, orders, products and stores) is in the tree and is not yet mounted in
`main.rs`.

## Stack

| Concern        | Crate                      |
| -------------- | -------------------------- |
| HTTP           | actix-web 4                |
| Async runtime  | tokio                      |
| Database       | sqlx (PostgreSQL, uuid, time) |
| Auth           | jsonwebtoken, bcrypt       |
| Serialization  | serde, serde_json          |
| Errors         | thiserror                  |
| Logging        | log, env_logger            |

## Configuration

Copy `.env.example` to `.env` and fill it in. The keys cover the server address and
environment, PostgreSQL and Redis connection strings, JWT and cookie secrets, logging,
CORS origins, WebSocket timeouts, optional SMTP for notifications, optional S3-compatible
storage for images, and the pool manager intervals.

Do not commit `.env`. It holds secrets.

## Run

```sh
cargo run
```

The server retries the database connection every ten seconds until it succeeds, then
listens on `127.0.0.1:8080`.

## Layout

| Path              | Responsibility                                   |
| ----------------- | ------------------------------------------------ |
| `src/main.rs`     | Bootstrap, database pool, route registration     |
| `src/http/`       | Handlers, one module per resource                |
| `src/db/`         | Queries and models                               |
| `src/validators/` | Input validation                                 |
| `src/websocket/`  | Realtime layer, not yet wired in                 |
| `src/config.rs`   | Environment loading                              |
| `Swagger.json`    | OpenAPI description of the HTTP API              |
