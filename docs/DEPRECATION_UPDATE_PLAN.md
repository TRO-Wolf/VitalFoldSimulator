# Deprecation & Outdated Code Update Plan

> **Target executor:** Claude Haiku
> **Scope:** `vital-fold-engine/` source tree and root `claude.md`
> **Goal:** Remove unused dependencies, fix deprecated API usage, and sync `claude.md` to the actual implementation.

---

## Summary of Issues Found

| # | Severity | Location | Issue |
|---|---|---|---|
| 1 | High | `Cargo.toml` | 5 unused dependencies bloat the build (including `duckdb` and `polars`, which are very large) |
| 2 | Medium | `src/middleware/auth.rs` | `Validation::default()` does not pin the JWT algorithm — algorithm confusion attack surface |
| 3 | Low | `src/middleware/auth.rs` | `chrono::Duration::hours()` — `Duration` is deprecated alias for `TimeDelta` since chrono 0.4.32 |
| 4 | Low | `src/generators/appointment.rs` | `chrono::Duration` used in 5 places |
| 5 | Low | `src/generators/patient.rs` | `chrono::Duration::days()` used in 2 places |
| 6 | Low | `src/generators/medical_record.rs` | `chrono::Duration::minutes()` used in 1 place |
| 7 | Medium | `claude.md` | Tech stack, error spec, db spec, and Cargo.toml section all reference `deadpool-postgres` / `tokio-postgres` but actual code uses `sqlx::PgPool` throughout |

---

## Step-by-Step Instructions for Haiku

Work through each step in order. After each step, verify the change compiles before moving on.

---

### Step 1 — Remove Unused Dependencies from `Cargo.toml`

**File:** `vital-fold-engine/Cargo.toml`

Remove the following lines entirely. None of these crates are imported anywhere in `src/`:

```toml
# DELETE these lines:
tokio-postgres = { version = "0.7", features = ["with-uuid-1", "with-chrono-0_4"] }
duckdb = { version = "1.4.4", features = ["bundled"] }
polars = { version = "0.53", features = ["parquet", "csv", "json"] }
config = "0.15"
aws-sdk-rds = "1"
```

After removing, run `cargo check` to confirm no missing imports.

**Expected result:** Cargo.toml has 5 fewer entries. Build time improves significantly (duckdb and polars are multi-minute builds).

---

### Step 2 — Fix JWT Algorithm in `src/middleware/auth.rs`

**File:** `vital-fold-engine/src/middleware/auth.rs`

**Change 1 — Update the import line (line 6):**

Find:
```rust
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
```

Replace with:
```rust
use jsonwebtoken::{decode, encode, Algorithm, DecodingKey, EncodingKey, Header, Validation};
```

**Change 2 — Fix `validate_token` (line 69):**

Find:
```rust
    decode::<Claims>(token, &decoding_key, &Validation::default())
```

Replace with:
```rust
    decode::<Claims>(token, &decoding_key, &Validation::new(Algorithm::HS256))
```

**Why:** `Validation::default()` accepts any signing algorithm. An attacker could craft a token signed with a different algorithm (e.g., RS256 using the public key as the secret) and it would pass. `Validation::new(Algorithm::HS256)` pins validation to exactly the algorithm used during encoding (`Header::default()` = HS256).

---

### Step 3 — Update `chrono::Duration` → `chrono::TimeDelta` in `src/middleware/auth.rs`

**File:** `vital-fold-engine/src/middleware/auth.rs`

Find (line 38):
```rust
    let expiration = now + chrono::Duration::hours(cfg.jwt_expiry_hours);
```

Replace with:
```rust
    let expiration = now + chrono::TimeDelta::hours(cfg.jwt_expiry_hours);
```

---

### Step 4 — Update `chrono::Duration` → `chrono::TimeDelta` in `src/generators/appointment.rs`

**File:** `vital-fold-engine/src/generators/appointment.rs`

**Change 1 — Update the import (line 12):**

Find:
```rust
use chrono::{Duration, NaiveDateTime, Utc};
```

Replace with:
```rust
use chrono::{TimeDelta, NaiveDateTime, Utc};
```

**Change 2 — Update usages (there are 4 occurrences of `Duration::` in this file):**

Find:
```rust
                    today + Duration::days(days_ahead),
```

Replace with:
```rust
                    today + TimeDelta::days(days_ahead),
```

Find:
```rust
    let checkout_time = (appointment_dt + Duration::minutes(checkout_offset)).format("%Y-%m-%dT%H:%M:%SZ").to_string();
    let provider_seen = (appointment_dt + Duration::minutes(provider_seen_offset)).format("%Y-%m-%dT%H:%M:%SZ").to_string();
```

Replace with:
```rust
    let checkout_time = (appointment_dt + TimeDelta::minutes(checkout_offset)).format("%Y-%m-%dT%H:%M:%SZ").to_string();
    let provider_seen = (appointment_dt + TimeDelta::minutes(provider_seen_offset)).format("%Y-%m-%dT%H:%M:%SZ").to_string();
```

Find (two occurrences of `Duration::days(90)` — one in `write_patient_visit`, one in `write_patient_vitals`):
```rust
    let expiry_epoch = (now + Duration::days(90)).timestamp();
```

Replace both with:
```rust
    let expiry_epoch = (now + TimeDelta::days(90)).timestamp();
```

---

### Step 5 — Update `chrono::Duration` → `chrono::TimeDelta` in `src/generators/patient.rs`

**File:** `vital-fold-engine/src/generators/patient.rs`

Find (line 99, inside `build_patient_batch`):
```rust
        batch.pt_dobs.push(today - chrono::Duration::days(days_back));
```

Replace with:
```rust
        batch.pt_dobs.push(today - chrono::TimeDelta::days(days_back));
```

Find (line 305, inside `generate_patient_insurance`):
```rust
                Some(today - chrono::Duration::days(rng.gen_range(30..365)))
```

Replace with:
```rust
                Some(today - chrono::TimeDelta::days(rng.gen_range(30..365)))
```

---

### Step 6 — Update `chrono::Duration` → `chrono::TimeDelta` in `src/generators/medical_record.rs`

**File:** `vital-fold-engine/src/generators/medical_record.rs`

Find (line 74):
```rust
                record_dates.push(appointment_date + chrono::Duration::minutes(offset));
```

Replace with:
```rust
                record_dates.push(appointment_date + chrono::TimeDelta::minutes(offset));
```

---

### Step 7 — Sync `claude.md` to Actual Implementation

**File:** `claude.md` (at repo root)

The `claude.md` document was written before implementation and describes `deadpool-postgres` as the DB pool. The actual implementation uses `sqlx::PgPool` throughout. Update the following sections:

#### 7a — Tech Stack table

Find:
```
| Database Pool (DSQL) | deadpool-postgres 0.12 + tokio-postgres 0.7 |
| Database ORM (simulation) | SQLx 0.8 (async, compile-time checked queries) |
```

Replace with:
```
| Database Pool (DSQL) | sqlx 0.8.6 PgPool (runtime-tokio-rustls, PgConnectOptions, SSL) |
```

(Remove the separate "Database ORM" row — it is now one entry since sqlx handles both pooling and queries.)

#### 7b — `src/errors.rs` module spec

Find:
```
- `From<tokio_postgres::Error>` and `From<deadpool_postgres::PoolError>` both map to `AppError::Database`
```

Replace with:
```
- `From<sqlx::Error>` maps to `AppError::Database`
```

#### 7c — `src/db/mod.rs` module spec

Find the entire block:
```
`create_pool(cfg: &Config) -> Result<Pool, AppError>`:
1. Load AWS config with `aws_config::defaults(BehaviorVersion::latest())` and the configured region
2. Generate an IAM auth token via `AuthTokenGenerator` → `db_connect_admin_auth_token`
3. Build `deadpool_postgres::Config` (host, dbname, user, password=token, port=5432, `RecyclingMethod::Fast`)
4. Return `pool.create_pool(Some(Runtime::Tokio1), NoTls)`

> **⚠ IAM tokens expire in ~15 min.** The initial build generates one token at startup. For long-running services, add a background Tokio task to rebuild the pool on a schedule.

Type alias: `pub type DbPool = deadpool_postgres::Pool;`
```

Replace with:
```
`create_pool(cfg: &Config) -> Result<DbPool, AppError>`:
1. Load AWS config with `aws_config::defaults(BehaviorVersion::latest())` and the configured region
2. Generate an IAM auth token via `AuthTokenGenerator` → `db_connect_admin_auth_token`
3. Build `PgConnectOptions` (host, port=5432, database, username, password=token, `PgSslMode::Require`)
4. Return `PgPoolOptions::new().max_connections(cfg.db_pool_size).connect_with(opts).await`

> **⚠ IAM tokens expire in ~15 min.** Call `db::spawn_token_refresh_task(pool.clone(), config.clone())` in `main.rs` after pool creation. It refreshes the token every 12 minutes via `pool.set_connect_options(new_opts)` without restarting the pool.

Type alias: `pub type DbPool = sqlx::PgPool;`
```

#### 7d — JWT validation note in `src/middleware/auth.rs` spec

Find:
```
- **`validate_token(token, secret)`** — decodes with `Validation::default()`, maps errors to `AppError::Unauthorized`
```

Replace with:
```
- **`validate_token(token, secret)`** — decodes with `Validation::new(Algorithm::HS256)`, maps errors to `AppError::Unauthorized`
```

#### 7e — Key Dependencies `Cargo.toml` section

In the `Key Dependencies (Cargo.toml)` section, remove these lines from the listed `[dependencies]`:

```toml
# REMOVE these — they are not in the actual Cargo.toml:
tokio-postgres = { version = "0.7", features = ["with-uuid-1", "with-chrono-0_4"] }
deadpool-postgres = "0.12"
duckdb = { version = "1.4.4", features = ["bundled"] }
polars = { version = "0.53", features = ["parquet", "csv", "json"] }
config = "0.15"
aws-sdk-rds = "1"
```

Also update the `Common Imports Reference` section — remove the `deadpool_postgres` import alias:

Find:
```rust
// Database
use sqlx::PgPool;
use deadpool_postgres::Pool as DeadPool;
```

Replace with:
```rust
// Database
use sqlx::PgPool;
```

---

## Verification Checklist

After completing all steps, run:

```bash
# 1. Confirm no compile errors
cargo check

# 2. Full build (verify deps removed properly)
cargo build

# 3. Confirm no remaining deprecated Duration usages
grep -rn "chrono::Duration" vital-fold-engine/src/

# 4. Confirm JWT validation is pinned
grep -rn "Validation::default" vital-fold-engine/src/

# 5. Confirm unused deps are gone
grep -n "tokio-postgres\|deadpool-postgres\|duckdb\|polars\|aws-sdk-rds" vital-fold-engine/Cargo.toml
```

All checks should return clean (no matches for checks 3–5, no errors for 1–2).

---

## Files Modified

| File | Steps |
|---|---|
| `vital-fold-engine/Cargo.toml` | Step 1 |
| `vital-fold-engine/src/middleware/auth.rs` | Steps 2, 3 |
| `vital-fold-engine/src/generators/appointment.rs` | Step 4 |
| `vital-fold-engine/src/generators/patient.rs` | Step 5 |
| `vital-fold-engine/src/generators/medical_record.rs` | Step 6 |
| `claude.md` | Step 7 |
