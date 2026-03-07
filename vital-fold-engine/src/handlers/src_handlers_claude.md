# `src/handlers/` — Claude Context

> **Purpose:** Self-contained reference for the `src/handlers/` subdirectory. Covers all HTTP endpoint logic across four handler files. No need to load `claude.md` to work on any handler.

---

## Overview

Handlers are Actix Web async functions that implement the REST API. They receive typed `web::Data<T>` arguments injected by Actix and return `Result<HttpResponse, AppError>`.

**Files:**
- `mod.rs` — re-exports all submodules
- `health.rs` — `GET /health`
- `auth.rs` — `POST /api/v1/auth/register`, `/login`, `/admin-login`
- `user.rs` — `GET /api/v1/me`
- `simulation.rs` — `POST /populate`, `POST /simulate`, `POST /simulate/stop`, `GET /simulate/status`, `POST /simulate/reset`, `POST /simulate/reset-dynamo`

**Auth protection:** Protected routes use `HttpAuthentication::bearer(jwt_validator)` applied at the scope level in `routes.rs`. The middleware injects `Claims` into request extensions before the handler runs.

---

## `mod.rs`

```rust
pub mod health;
pub mod auth;
pub mod user;
pub mod simulation;

pub use health::*;
pub use auth::*;
pub use user::*;
pub use simulation::*;
```

---

## `health.rs` — Health Check

### `HealthResponse` (local struct)

```rust
#[derive(Debug, Serialize, ToSchema)]
struct HealthResponse { status: String }
```

### `health_check`

```rust
pub async fn health_check() -> HttpResponse
```

| Route | Method | Auth | Response |
|---|---|---|---|
| `/health` | GET | None | `200 OK` `{"status":"ok"}` |

Always succeeds. Used for liveness probes on Render.com (`healthCheckPath: /health`).

---

## `auth.rs` — Authentication Endpoints

### Local Struct: `AdminLoginRequest`

Defined here (not in `src/models/`):

```rust
#[derive(Debug, Deserialize, utoipa::ToSchema)]
pub struct AdminLoginRequest {
    pub username: String,
    pub password: String,
}
```

### `register`

```rust
pub async fn register(
    pool: web::Data<DbPool>,
    cfg: web::Data<Config>,
    req: web::Json<RegisterRequest>,
) -> Result<HttpResponse, AppError>
```

| Route | Method | Auth | Success | Failure |
|---|---|---|---|---|
| `/api/v1/auth/register` | POST | None | `201 Created` + `AuthResponse` | See below |

**Flow:**
1. `req.validate()` — checks email format and password ≥ 8 chars (→ 400 on failure)
2. Normalize email: `req.email.trim().to_lowercase()`
3. `bcrypt::hash(&req.password, DEFAULT_COST)` (→ 500 on error)
4. Generate `user_id = Uuid::new_v4()`, `now = Utc::now()`
5. `INSERT INTO public.users (id, email, password_hash, created_at) VALUES ($1, $2, $3, $4)`
   - Duplicate key error → `AppError::BadRequest("Email already registered")`
   - Other DB error → `AppError::Database`
6. `generate_token(user_id, email, cfg)` (→ 500 on error)
7. Build `UserProfile` manually (no DB round-trip): `{ id: user_id, email, created_at: now }`
8. Return `HttpResponse::Created().json(AuthResponse { token, user: user_profile })`

---

### `login`

```rust
pub async fn login(
    pool: web::Data<DbPool>,
    cfg: web::Data<Config>,
    req: web::Json<LoginRequest>,
) -> Result<HttpResponse, AppError>
```

| Route | Method | Auth | Success | Failure |
|---|---|---|---|---|
| `/api/v1/auth/login` | POST | None | `200 OK` + `AuthResponse` | See below |

**Flow:**
1. `req.validate()` — checks email and password not empty (→ 400 on failure)
2. Normalize email: `req.email.trim().to_lowercase()`
3. `SELECT id, email, password_hash, created_at FROM public.users WHERE email = $1` via `fetch_optional`
   - Row missing → `AppError::Unauthorized("Invalid credentials")` (same message — prevents enumeration)
4. `bcrypt::verify(&req.password, &user.password_hash)` (→ 500 on bcrypt error)
   - Verification false → `AppError::Unauthorized("Invalid credentials")`
5. `generate_token(user.id, user.email.clone(), cfg)`
6. `UserProfile::from(user)` — uses the `From<User>` impl
7. Return `HttpResponse::Ok().json(AuthResponse { token, user: user_profile })`

**Security:** Unknown email and wrong password return **identical** error messages to prevent user enumeration.

---

### `admin_login`

```rust
pub async fn admin_login(
    cfg: web::Data<Config>,
    req: web::Json<AdminLoginRequest>,
) -> Result<HttpResponse, AppError>
```

| Route | Method | Auth | Success | Failure |
|---|---|---|---|---|
| `/api/v1/auth/admin-login` | POST | None | `200 OK` + `AuthResponse` | `401 Unauthorized` |

**Flow:**
1. Read `cfg.admin_username` and `cfg.admin_password` (`Option<String>`)
   - Either missing → `AppError::Unauthorized("Invalid credentials")`
2. Compare `req.username == expected_username` and `req.password == expected_password`
   - Either mismatch → `AppError::Unauthorized("Invalid credentials")`
3. `admin_id = Uuid::parse_str("00000000-0000-0000-0000-000000000001")` — hardcoded, stable across restarts
4. `admin_email = format!("{}@admin.internal", expected_username)`
5. `generate_token(admin_id, admin_email, cfg)`
6. Build `UserProfile { id: admin_id, email: admin_email, created_at: Utc::now() }` (no DB row)
7. Return `HttpResponse::Ok().json(AuthResponse { token, user })`

**Note:** Admin identity exists only in the JWT — no `public.users` row is required.

---

## `user.rs` — Authenticated User Profile

### `me`

```rust
pub async fn me(
    req: HttpRequest,
    pool: web::Data<DbPool>,
) -> Result<HttpResponse, AppError>
```

| Route | Method | Auth | Success | Failure |
|---|---|---|---|---|
| `/api/v1/me` | GET | Bearer JWT | `200 OK` + `UserProfile` | See below |

**Flow:**
1. `req.extensions().get::<Claims>()` — extracts Claims inserted by `jwt_validator`
   - Missing → `AppError::Unauthorized("Authentication required")` (should never happen on protected routes)
2. `Uuid::parse_str(&claims.sub)` — parses user ID from JWT subject
   - Parse error → `AppError::Unauthorized("Invalid token")`
3. `SELECT id, email, password_hash, created_at FROM public.users WHERE id = $1` via `fetch_optional`
   - Row missing → `AppError::NotFound("User not found")`
4. `UserProfile::from(user)` — drops `password_hash`
5. Return `HttpResponse::Ok().json(user_profile)`

---

## `simulation.rs` — Data Population and Simulation

### Local Struct: `PopulateRequest`

```rust
#[derive(Debug, Deserialize, utoipa::ToSchema)]
pub struct PopulateRequest {
    pub plans_per_company:        Option<usize>,  // default: 3
    pub providers:                Option<usize>,  // default: 50
    pub patients:                 Option<usize>,  // default: 50_000
    pub appointments_per_patient: Option<usize>,  // default: 2
    pub records_per_appointment:  Option<usize>,  // default: 1
}
```

All fields are `Option`. `None` **and** `0` both fall back to the default (zero-filter logic in handler).

---

### `start_populate`

```rust
pub async fn start_populate(
    pool: web::Data<DbPool>,
    dynamo: web::Data<DynamoClient>,
    state: web::Data<SimulatorState>,
    body: Option<web::Json<PopulateRequest>>,
) -> Result<HttpResponse, AppError>
```

| Route | Method | Auth | Success | Conflict |
|---|---|---|---|---|
| `/populate` | POST | Bearer JWT | `202 Accepted` | `400 Bad Request` if already running |

**Flow:**
1. `state.try_start()` — atomic CAS from idle→running
   - Returns false if already running → `AppError::BadRequest("A run is already in progress")`
2. Build `SimulationConfig` from body, using `nonzero(v, default)` — treats `None` and `0` as default
3. Clone pool, dynamo, state; `tokio::spawn` background task calling `run_populate()`
4. Background task calls `state_clone.stop()` on completion (success or error)
5. Return `202 Accepted` + `MessageResponse { message: "Population started" }` immediately

**Seeds:** 7 insurance companies, 10 clinics, N providers, N patients, clinic schedules, N appointments (0–89 days future), N medical records. **No DynamoDB writes.**

---

### `start_simulate`

```rust
pub async fn start_simulate(
    pool: web::Data<DbPool>,
    dynamo: web::Data<DynamoClient>,
    state: web::Data<SimulatorState>,
) -> Result<HttpResponse, AppError>
```

| Route | Method | Auth | Success | Conflict |
|---|---|---|---|---|
| `/simulate` | POST | Bearer JWT | `202 Accepted` | `400 Bad Request` if already running |

**Flow:**
1. `state.try_start()` — same atomic guard as populate
2. Clone pool, dynamo, state; `tokio::spawn` calling `run_simulate()`
3. `run_simulate` queries `vital_fold.appointment WHERE appointment_date::date = CURRENT_DATE`
4. Writes `patient_visit` + `patient_vitals` DynamoDB records for each appointment found
5. Background task calls `state_clone.stop()` on completion
6. Returns `202 Accepted` immediately

**Note:** If no appointments exist for today, `run_simulate` logs a warning and exits cleanly.

---

### `stop_simulation`

```rust
pub async fn stop_simulation(state: web::Data<SimulatorState>) -> Result<HttpResponse, AppError>
```

| Route | Method | Auth | Response |
|---|---|---|---|
| `/simulate/stop` | POST | Bearer JWT | `200 OK` + `{"message":"Run stopped"}` |

Calls `state.stop()` (sets `AtomicBool` to false). The running background task checks this flag and exits gracefully after its current batch.

---

### `get_status`

```rust
pub async fn get_status(state: web::Data<SimulatorState>) -> Result<HttpResponse, AppError>
```

| Route | Method | Auth | Response |
|---|---|---|---|
| `/simulate/status` | GET | Bearer JWT | `200 OK` + `SimulationStatusResponse` |

Returns:
```json
{
  "running": true,
  "last_run": "2025-01-01T12:00:00Z",
  "insurance_companies": 7,
  "insurance_plans": 21,
  "clinics": 10,
  "providers": 50,
  "patients": 50000,
  "emergency_contacts": 50000,
  "patient_demographics": 50000,
  "patient_insurance": 50000,
  "clinic_schedules": 350,
  "appointments": 100000,
  "medical_records": 100000,
  "dynamo_patient_visits": 1200,
  "dynamo_patient_vitals": 1200
}
```

(`counts` is `#[serde(flatten)]` — all count fields appear at the top level.)

---

### `reset_data`

```rust
pub async fn reset_data(
    pool: web::Data<DbPool>,
    state: web::Data<SimulatorState>,
) -> Result<HttpResponse, AppError>
```

| Route | Method | Auth | Success | Blocked |
|---|---|---|---|---|
| `/simulate/reset` | POST | Bearer JWT | `200 OK` | `400` if running |

**Deletes all rows from all `vital_fold` schema tables in FK-safe (children-first) order:**

```
1. vital_fold.medical_record      (medical_record_id)
2. vital_fold.appointment         (appointment_id)
3. vital_fold.clinic_schedule     (schedule_id)
4. vital_fold.patient_insurance   (patient_insurance_id)
5. vital_fold.patient_demographics(demographics_id)
6. vital_fold.emergency_contact   (emergency_contact_id)
7. vital_fold.patient             (patient_id)
8. vital_fold.provider            (provider_id)
9. vital_fold.clinic              (clinic_id)
10. vital_fold.insurance_plan     (insurance_plan_id)
11. vital_fold.insurance_company  (company_id)
```

**Per-table loop (Aurora DSQL has no TRUNCATE and no `ctid`):**
```sql
DELETE FROM {table} WHERE {pk} IN (SELECT {pk} FROM {table} LIMIT 2500)
-- repeated until rows_affected() == 0
```

**Does NOT affect DynamoDB** — use `/simulate/reset-dynamo` separately.

---

### `reset_dynamo`

```rust
pub async fn reset_dynamo(
    dynamo: web::Data<DynamoClient>,
    state: web::Data<SimulatorState>,
) -> Result<HttpResponse, AppError>
```

| Route | Method | Auth | Success | Blocked |
|---|---|---|---|---|
| `/simulate/reset-dynamo` | POST | Bearer JWT | `200 OK` | `400` if running |

Calls `delete_dynamo_table()` for both `patient_visit` and `patient_vitals`. Runs synchronously (not spawned). Both tables share key schema: `PK=patient_id (S)`, `SK=clinic_id (S)`.

---

### `delete_dynamo_table` (private helper)

```rust
async fn delete_dynamo_table(
    dynamo: &DynamoClient,
    table: &str,
    pk_name: &str,
    sk_name: &str,
) -> Result<u64, AppError>
```

**Algorithm:**
1. `scan()` with `projection_expression("#pk, #sk")` — minimal RCU cost
2. Chunk scan results into groups of 25 (DynamoDB `BatchWriteItem` max)
3. For each chunk: `batch_write_item()` with `DeleteRequest` entries
4. Retry `UnprocessedItems` until empty (backoff resets to 0 on success)
5. On `ThrottlingException`: exponential backoff up to `MAX_RETRIES=5`
6. Sleep `CHUNK_DELAY_MS=50` between chunks (≈500 WCU/s sustained)
7. Repeat scan pages until `LastEvaluatedKey` is absent

**Backoff schedule on throttle:**

| Attempt | Delay |
|---|---|
| 1 | 1,000 ms |
| 2 | 2,000 ms |
| 3 | 4,000 ms |
| 4 | 8,000 ms |
| 5 | 16,000 ms |

After `MAX_RETRIES` exceeded → `AppError::Internal`.

**Returns:** total items deleted (`u64`)

---

## HTTP Status Code Reference

| Endpoint | 200 | 201 | 202 | 400 | 401 | 404 | 500 |
|---|---|---|---|---|---|---|---|
| `GET /health` | ✓ | | | | | | |
| `POST /api/v1/auth/register` | | ✓ | | dup email | | | bcrypt/DB |
| `POST /api/v1/auth/login` | ✓ | | | empty fields | bad creds | | bcrypt/DB |
| `POST /api/v1/auth/admin-login` | ✓ | | | | not configured / wrong creds | | |
| `GET /api/v1/me` | ✓ | | | | no/bad token | user gone | DB |
| `POST /populate` | | | ✓ | already running | no token | | |
| `POST /simulate` | | | ✓ | already running | no token | | |
| `POST /simulate/stop` | ✓ | | | | no token | | |
| `GET /simulate/status` | ✓ | | | | no token | | |
| `POST /simulate/reset` | ✓ | | | running | no token | | DB |
| `POST /simulate/reset-dynamo` | ✓ | | | running | no token | | DynamoDB |

---

## Cross-Module Relationships

**Imports from:**
- `crate::db::DbPool`
- `crate::config::Config`
- `crate::errors::AppError`
- `crate::middleware::auth::{generate_token, Claims}`
- `crate::models::{RegisterRequest, LoginRequest, AuthResponse, User, UserProfile, MessageResponse, SimulationStatusResponse}`
- `crate::engine_state::SimulatorState`
- `crate::generators::{run_populate, run_simulate, SimulationConfig}`
- `actix_web`, `bcrypt`, `chrono`, `uuid`, `aws_sdk_dynamodb`

**Exported to:** `routes.rs` routes all handlers to URL paths

---

## Common Imports for This Module

```rust
// Core
use crate::db::DbPool;
use crate::config::Config;
use crate::errors::AppError;
use crate::middleware::auth::{generate_token, Claims};
use crate::models::{RegisterRequest, LoginRequest, AuthResponse, User, UserProfile,
                    MessageResponse, SimulationStatusResponse};
use crate::engine_state::SimulatorState;
use crate::generators::{run_populate, run_simulate, SimulationConfig};

// Actix
use actix_web::{web, HttpRequest, HttpMessage, HttpResponse};

// Auth/hashing
use bcrypt::{hash, verify, DEFAULT_COST};
use uuid::Uuid;
use chrono::Utc;
use serde::Deserialize;

// DynamoDB (simulation.rs only)
use aws_sdk_dynamodb::Client as DynamoClient;
use aws_sdk_dynamodb::types::{DeleteRequest, WriteRequest};
use std::collections::HashMap;
```
