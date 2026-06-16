# AgroVest Backend — Agent Guide

Single-crate Rust/Axum REST API for a Web3 agricultural investment platform.
No workspace, no monorepo — one `Cargo.toml`, one binary target.

## Commands

```bash
# Start database dependencies (required before cargo run/test)
docker-compose up -d postgres redis

# Copy env and configure (DATABASE_URL + JWT_SECRET are required)
cp .env.example .env

# Run migrations
cargo install sqlx-cli
sqlx migrate run

# Build & run
cargo run

# CI gate (run all four before committing)
cargo fmt -- --check
cargo clippy -- -D warnings
cargo check
cargo test
```

Tests require live Postgres and Redis (no mocks). Integration test dirs exist but are empty.

## Environment

`dotenvy` loads `.env` automatically. Two vars are hard-required: `DATABASE_URL`, `JWT_SECRET`.
All others have defaults or are optional (see `.env.example`).

## Architecture

```
main.rs → AppConfig::from_env() → AppState::new() → build_router() → axum::serve
```

- **routes/** — HTTP handlers, each module has a `pub fn routes() -> Router<AppState>`
- **services/** — business logic, takes `&PgPool` or `&AppState`, returns `Result<T, ApiError>`
- **models/** — DB row structs (`FromRow` + `Serialize` + `Deserialize`)
- **middleware/auth.rs** — `AuthUser` extractor (JWT Bearer token, HS256)
- **error.rs** — `ApiError` enum, converts to JSON `{ "error": { "code", "message" } }`
- **blockchain/** — Soroban RPC client, event polling (indexer spawns background tokio tasks)
- **db/redis.rs** — Redis caching helpers
- **utils/crypto.rs** — Ed25519 signature verification
- **utils/pagination.rs** — shared pagination query parsing

All routes are nested under `/api/v1`.

## SQLx usage

Queries use runtime `sqlx::query_as` (not compile-time checked macros).
No `.sqlx/` offline cache directory exists. CI sets `SQLX_OFFLINE=true` to skip checking.

## Adding a new endpoint

1. Create migration in `migrations/NNN_description.sql` (sequential numbering, `IF NOT EXISTS`)
2. Add model in `src/models/` with `#[derive(Debug, Serialize, Deserialize, FromRow)]`
3. Add service in `src/services/` — functions return `Result<T, ApiError>`
4. Add route in `src/routes/` — create `pub fn routes() -> Router<AppState>` and merge it in `routes/mod.rs`
5. Use `ApiError` variants for error responses, not raw `StatusCode`

## Gotcha: CORS config

`routes/mod.rs` currently hardcodes `allow_origin(Any)` — the `CORS_ORIGINS` env var
in `config.rs` is parsed but unused in the router. Don't assume it takes effect.
