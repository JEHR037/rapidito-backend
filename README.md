# Rapidito Backend

Order management API for the Rapidito delivery app, written in Rust on Actix-web with
PostgreSQL through sqlx. It covers accounts, addresses, stores, products, orders and
drivers, with JWT authentication, bcrypt password hashing and role-based access for
customers, partners, drivers and admins.

## API contract

[`docs/API.md`](docs/API.md) is the single source of truth for routes, payloads, enums,
status transitions and error codes. It is shared with `rapidito-app`; change it first and
then both sides. Every route lives under `/api`, JSON uses `camelCase`, and money travels as
integer cents.

## Stack

| Concern        | Crate                                        |
| -------------- | -------------------------------------------- |
| HTTP           | actix-web 4, actix-cors                      |
| Async runtime  | tokio                                        |
| Database       | sqlx (PostgreSQL, migrations, uuid, chrono)  |
| Auth           | jsonwebtoken (HS256), bcrypt                 |
| Serialization  | serde, serde_json                            |
| Errors         | thiserror                                    |
| Logging        | log, env_logger                              |

## Configuration

Copy `.env.example` to `.env` and fill it in. `.env` is ignored by git; never commit it.

| Key                        | Required | Default          | Purpose                                         |
| -------------------------- | -------- | ---------------- | ----------------------------------------------- |
| `DATABASE_URL`             | yes      |                  | PostgreSQL connection string                    |
| `JWT_SECRET`               | yes      |                  | HS256 signing key, at least 32 characters       |
| `SERVER_ADDR`              | no       | `127.0.0.1:8080` | Bind address                                    |
| `DATABASE_MAX_CONNECTIONS` | no       | `10`             | Pool size                                       |
| `JWT_EXPIRATION`           | no       | `24h`            | Token lifetime, in hours (`12h`) or days (`2d`) |
| `CORS_ALLOWED_ORIGINS`     | no       | none             | Comma separated browser origins                 |
| `DELIVERY_FEE_CENTS`       | no       | `0`              | Flat delivery fee added to every order          |
| `RUST_LOG`                 | no       | `info`           | Log filter                                      |

Native mobile clients send no `Origin` header, so CORS only matters for the web build.

## Run

```sh
cargo run
```

The server retries the database connection every ten seconds until it succeeds, applies
pending migrations from `migrations/`, and then listens on `SERVER_ADDR`.

### Migrations

Migrations run automatically at startup and are embedded in the binary.

| Migration                       | Purpose                                                                                     |
| ------------------------------- | ------------------------------------------------------------------------------------------- |
| `20261029000001_baseline`       | Original schema. It is skipped on databases that already have it, so existing data is kept. |
| `20261029000002_app_integration` | Order statuses of the app, money as integer cents, store ownership and branding, address labels and defaults, user phone and avatar, removal of stored cards |

The second migration is destructive in two places: it drops `payment_methods` (full card
numbers and CVVs must never be stored) and the `name`, `email` and `license_plate` columns
duplicated in `drivers` (they now come from `users` and `vehicles`). Back up production
before deploying it.

### First admin

Registration always creates customers. Register the first admin through the API and promote
it once from the database:

```sql
UPDATE users SET role = 'Admin' WHERE email = 'admin@example.com';
```

Every other role change goes through `/api/admin/users/{id}` and `/api/drivers`.

## Test

```sh
cargo test
```

Unit tests cover the order status machine, server-side pricing, input validation and
configuration parsing.

`scripts/e2e_smoke.py` walks the whole contract against a running server and a disposable
database (it promotes users with `psql`, so never point it at real data):

```sh
API_URL=http://127.0.0.1:8080/api DATABASE_URL=postgres://... python3 scripts/e2e_smoke.py
```

## Layout

| Path                 | Responsibility                                                   |
| -------------------- | ---------------------------------------------------------------- |
| `src/main.rs`        | Bootstrap: configuration, pool, migrations, middleware, routes   |
| `src/config.rs`      | Typed environment configuration                                  |
| `src/error.rs`       | `ApiError` and the JSON error body                               |
| `src/auth.rs`        | Token issuing and the `AuthUser` extractor                       |
| `src/domain.rs`      | Enums and the order status transition table                      |
| `src/pricing.rs`     | Server-side order pricing                                        |
| `src/validation.rs`  | Input validation helpers                                         |
| `src/http/`          | Handlers, one module per resource                                |
| `src/db/`            | Queries and row models, one module per aggregate                 |
| `src/websocket/`     | Realtime layer, planned for API version 2 and not compiled yet   |
| `migrations/`        | sqlx migrations                                                  |
| `docs/API.md`        | API contract                                                     |
