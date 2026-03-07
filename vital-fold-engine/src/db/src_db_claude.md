# `src/db/` — Claude Context

> **Purpose:** Self-contained reference for the `src/db/` subdirectory. One file: `mod.rs`. Covers Aurora DSQL connection pool creation and IAM token management.

---

## Overview

The `db` module manages the PostgreSQL connection pool for Aurora DSQL. Aurora DSQL uses short-lived IAM-signed tokens instead of static passwords, which requires:

1. **Generating a fresh token at startup** before the pool is created
2. **Refreshing the token every 12 minutes** via a background task (tokens expire in ~15 min)

The pool itself (`DbPool`) is a type alias for `sqlx::PgPool` and is registered as `web::Data<DbPool>` in `main.rs`, shared across all handlers.

**Why `PgConnectOptions` instead of a URL string:** IAM tokens contain `=` and `+` characters that break URL parsing. Options struct avoids this.

---

## Type Alias

```rust
pub type DbPool = sqlx::PgPool;
```

Used everywhere else in the codebase. Import as `use crate::db::DbPool`.

---

## Constants

```rust
const TOKEN_REFRESH_INTERVAL: Duration = Duration::from_secs(12 * 60);
// 12 minutes — stays well under DSQL's ~15-minute token expiry
```

---

## Public Functions

### `create_pool`

```rust
pub async fn create_pool(cfg: &Config) -> Result<DbPool, AppError>
```

**What it does:**
1. Loads AWS config (`aws_config::defaults(BehaviorVersion::latest())`) with the configured region
2. Calls `generate_auth_token()` to get a SigV4-signed IAM token
3. Calls `build_connect_opts()` to construct `PgConnectOptions`
4. Creates `PgPoolOptions::new().max_connections(cfg.db_pool_size as u32).connect_with(opts)`
5. Logs pool creation with endpoint and max size

**Called from:** `main.rs` at startup, before `HttpServer::new`.

**Errors:** Returns `AppError::Database` if token generation fails or pool creation fails. Logs with `tracing::error!` before returning.

---

### `spawn_token_refresh_task`

```rust
pub fn spawn_token_refresh_task(pool: DbPool, cfg: Config)
```

**What it does:**
- Spawns a `tokio::spawn` background loop that runs forever
- Every `TOKEN_REFRESH_INTERVAL` (12 min), generates a fresh IAM token and calls `pool.set_connect_options(new_opts)`
- `set_connect_options` updates options for **new** connections only — existing checked-out connections are not affected
- The pool and all `web::Data<DbPool>` clones remain valid throughout

**On error:** Logs `tracing::error!` and continues the loop — the previous token may still be valid for the remaining window.

**Called from:** `main.rs` immediately after `create_pool()`.

```rust
// main.rs usage:
let pool = create_pool(&config).await.expect("...");
db::spawn_token_refresh_task(pool.clone(), config.clone());
```

---

## Private Functions

### `build_connect_opts` (private)

```rust
fn build_connect_opts(cfg: &Config, token: &str) -> PgConnectOptions
```

Constructs the SQLx connection options:

| Option | Value |
|---|---|
| host | `cfg.dsql_endpoint` |
| port | `5432` (hardcoded) |
| database | `cfg.dsql_db_name` |
| username | `cfg.dsql_user` |
| password | `token` (the IAM auth token) |
| ssl_mode | `PgSslMode::Require` |

---

### `generate_auth_token` (private)

```rust
async fn generate_auth_token(
    aws_config: &aws_config::SdkConfig,
    endpoint: &str,
) -> Result<String, Box<dyn std::error::Error + Send + Sync>>
```

Generates a SigV4-signed IAM auth token using the AWS SDK:

```rust
let token_config = DsqlTokenConfig::builder().hostname(endpoint).build()?;
let generator = AuthTokenGenerator::new(token_config);
let token = generator.db_connect_admin_auth_token(aws_config).await?;
Ok(token.to_string())
```

Uses `aws_sdk_dsql::auth_token::{AuthTokenGenerator, Config as DsqlTokenConfig}`.

---

## AWS Credential Lookup Order

The AWS config loaded by `aws_config::defaults(...)` resolves credentials in this order:

1. Environment variables: `AWS_ACCESS_KEY_ID`, `AWS_SECRET_ACCESS_KEY`, `AWS_SESSION_TOKEN`
2. Shared credentials file: `~/.aws/credentials`
3. AWS SSO / IAM Identity Center
4. EC2/ECS/Lambda instance metadata (IAM role)

**Required IAM permission:**
```json
{
  "Effect": "Allow",
  "Action": "dsql:DbConnectAdmin",
  "Resource": "arn:aws:dsql:<region>:<account>:cluster/<cluster-id>"
}
```

---

## Config Fields Used

From `crate::config::Config`:

| Field | Purpose |
|---|---|
| `dsql_endpoint` | Aurora DSQL cluster hostname |
| `dsql_region` | AWS region for token signing (default: `"us-east-1"`) |
| `dsql_db_name` | PostgreSQL database name (default: `"postgres"`) |
| `dsql_user` | PostgreSQL username (default: `"admin"`) |
| `db_pool_size` | sqlx max pool connections (default: `10`) |

---

## Cross-Module Relationships

**Imports from:**
- `crate::config::Config`
- `crate::errors::AppError`
- `aws_config`, `aws_sdk_dsql`, `sqlx`

**Exported to:**
- `crate::db::DbPool` — used in handlers, generators, routes
- `crate::db::create_pool` — called in `main.rs`
- `crate::db::spawn_token_refresh_task` — called in `main.rs`

---

## Common Imports for This Module

```rust
use crate::config::Config;
use crate::errors::AppError;
use aws_config::BehaviorVersion;
use aws_sdk_dsql::auth_token::{AuthTokenGenerator, Config as DsqlTokenConfig};
use sqlx::postgres::{PgConnectOptions, PgPoolOptions, PgSslMode};
use std::time::Duration;
```
