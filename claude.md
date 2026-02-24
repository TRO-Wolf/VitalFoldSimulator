# VitalFold Engine — Claude Context

> **Primary builder:** Claude Haiku. This document is the single authoritative source of truth for all code generation. Every section is written so that Haiku can implement the full project with no ambiguity. Do not deviate from the specifications below unless a compilation error requires it. When in doubt, implement the simplest correct solution and leave a `// TODO:` comment.

---

## Project Overview

**Vital Fold Health LLC** is a multi-region healthcare company headquartered in Florida. VitalFold Engine is a **data simulator and REST API** written in **Rust** using the **Actix Web** framework. It is a portfolio-grade data pipeline seed project with two primary purposes:

1. **Data simulation** — generate and populate realistic synthetic health clinic data into Aurora DSQL and DynamoDB, simulating patient activity across clinic locations in the southeastern United States.
2. **Authenticated API** — expose protected REST endpoints secured with JWT bearer tokens, allowing the simulator to be started, stopped, and monitored via API calls.

The project is deployed to **Render.com** and feeds a downstream AWS data pipeline.

### Business Context
- The health company operates clinics across multiple regions (see "Clinic Locations" below).
- Data simulates real patient flow: registrations, appointments, diagnoses, and medical records.
- The simulator can be toggled on/off via API so data generation can be controlled without redeployment.
- Insurance companies are drawn from a fixed list of fictional carriers (see "Synthetic Data").
- Diagnosis codes are drawn from a fixed cardiac-focused list (see "Synthetic Data").

---

## Synthetic Data Domain Values

Haiku must use these exact values when seeding fixed-domain data. Do not invent alternatives.

### Insurance Companies (use these names verbatim)
1. Orange Spear
2. Care Medical
3. Cade Medical
4. Multiplied Health
5. Octi Care
6. Tatnay
7. Caymana

### Diagnosis / ICD Codes (cardiac-focused clinic)

Use these exact strings when populating `medical_record.diagnosis`:

1. `"Atrial Fibrillation (AFib)"`
2. `"Coronary Artery Disease (CAD)"`
3. `"Chest Pain"`
4. `"Hypertension"`
5. `"Hyperlipidemia"`
6. `"Shortness of Breath (SOB)"`
7. `"Tachycardia"`
8. `"Bradycardia"`

**Note:** These are the canonical spellings. The synthetic_data.md file contains typos — use this list instead.

### Clinic Locations (seed exactly this distribution)

| City | State | Count |
|---|---|---|
| Charlotte | NC | 1 |
| Asheville | NC | 1 |
| Atlanta | GA | 2 |
| Tallahassee | FL | 1 |
| Miami | FL | 2 |
| Orlando | FL | 1 |
| Jacksonville | FL | 2 |

**Total: 10 clinics.** The company HQ is in Florida. Florida clinics (5) represent the largest cluster.

### Provider Names
Generate using the `fake` crate — names must be clearly random/fictional (e.g., "Dr. Karev Plinton"). Do not use real physician names.

### Patient Names
Generate using the `fake` crate — names must be clearly random/fictional. Distribute patient home addresses across the clinic metro areas proportionally.

---

## Tech Stack

| Layer | Technology |
|---|---|
| Language | Rust (stable, 2021 edition) |
| Web Framework | Actix Web 4 |
| Auth Middleware | actix-web-httpauth 0.8 (bearer tokens) |
| Database Pool (DSQL) | deadpool-postgres 0.12 + tokio-postgres 0.7 |
| Database ORM (simulation) | SQLx 0.8 (async, compile-time checked queries) |
| Database | Amazon Aurora DSQL (Postgres-compatible) + PostgreSQL 15+ |
| Async Runtime | Tokio |
| Data Faking | `fake` crate v4 |
| Serialization | Serde + serde_json |
| Passwords | bcrypt (DEFAULT_COST) |
| Tokens | jsonwebtoken 9 (HS256, configurable TTL) |
| Config | `config` crate + dotenvy (environment-based) |
| Analytics | DuckDB 1.4.4 + Polars 0.53 |
| OpenAPI / Docs | utoipa 5 + utoipa-swagger-ui 9 |
| Deployment | Render.com (Docker or native Rust buildpack) |

---

## Project Structure

```
vitalFoldEngine/
├── Cargo.toml
├── Dockerfile
├── render.yaml
├── .env.example
├── .gitignore
├── health_clinic_schema.sql
├── claude.md
├── README.md
├── migrations/
│   └── 001_init.sql              # Users table DDL for Aurora DSQL
└── src/
    ├── main.rs                   # Actix server bootstrap; registers SimulatorState as app_data
    ├── config.rs                 # Typed config from env vars
    ├── engine_state.rs           # AtomicBool SimulatorState (running flag + last_run + counts)
    ├── db/
    │   └── mod.rs                # deadpool-postgres pool + DSQL IAM auth
    ├── errors.rs                 # Unified AppError type
    ├── routes.rs                 # Route registration
    ├── middleware/
    │   ├── mod.rs
    │   └── auth.rs               # generate_token(), validate_token(), jwt_validator()
    ├── models/
    │   ├── mod.rs
    │   ├── user.rs               # User, RegisterRequest, LoginRequest, AuthResponse, UserProfile
    │   ├── insurance.rs          # InsuranceCompany, InsurancePlan, PatientInsurance
    │   ├── patient.rs            # Patient, EmergencyContact, PatientDemographics
    │   ├── provider.rs           # Provider
    │   ├── clinic.rs             # Clinic, ClinicSchedule
    │   ├── appointment.rs        # Appointment
    │   └── medical_record.rs     # MedicalRecord
    ├── generators/
    │   ├── mod.rs                # SimulationContext (holds shared pools/counts)
    │   ├── insurance.rs
    │   ├── patient.rs
    │   ├── provider.rs
    │   ├── clinic.rs
    │   ├── appointment.rs
    │   └── medical_record.rs
    └── handlers/
        ├── mod.rs
        ├── auth.rs               # POST /api/v1/auth/register, /login
        ├── user.rs               # GET /api/v1/me (protected)
        ├── simulation.rs         # POST /simulate, POST /simulate/stop, GET /simulate/status
        └── health.rs             # GET /health
```

---

## API Endpoints

### Health Check
```
GET /health
→ 200 OK { "status": "ok" }
```

### Authentication (public)
```
POST /api/v1/auth/register
Body: { "email": "user@example.com", "password": "..." }
→ 201 Created { "token": "<jwt>", "user": { "id", "email", "created_at" } }

POST /api/v1/auth/login
Body: { "email": "user@example.com", "password": "..." }
→ 200 OK { "token": "<jwt>", "user": { "id", "email", "created_at" } }
```

### User (protected — Bearer token required)
```
GET /api/v1/me
→ 200 OK { "id": "<uuid>", "email": "...", "created_at": "..." }
```

### Simulation (protected)
```
POST /simulate
Body (JSON, all fields optional — omit any to use the default):
{
  "plans_per_company": 3,
  "providers": 50,
  "patients": 50000,
  "appointments_per_patient": 2,
  "records_per_appointment": 1
}
Note: insurance_companies and clinics are NOT configurable — they are seeded
from the fixed domain lists. The request body controls volume only.
→ 202 Accepted { "message": "Simulation started" }

POST /simulate/stop
→ 200 OK { "message": "stop signal sent" }

GET /simulate/status
→ 200 OK { "running": true|false, "last_run": "<timestamp>", "counts": { ... } }

DELETE /simulate/reset
→ 200 OK { "message": "all tables truncated" }
```

**Simulation Request/Response Models** (in `src/handlers/simulation.rs`):

```rust
/// All fields are Option — omit any field to fall back to SimulationConfig::default().
#[derive(Debug, Deserialize, utoipa::ToSchema)]
pub struct SimulateRequest {
    pub plans_per_company: Option<usize>,
    pub providers: Option<usize>,
    pub patients: Option<usize>,
    pub appointments_per_patient: Option<usize>,
    pub records_per_appointment: Option<usize>,
}

#[derive(Debug, Serialize)]
pub struct SimulateResponse {
    pub job_id: Uuid,
    pub status: String,
}

#[derive(Debug, Serialize)]
pub struct SimulationStatusResponse {
    pub running: bool,
    pub last_run: Option<DateTime<Utc>>,
    pub counts: SimulationCounts,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SimulationCounts {
    pub insurance_companies: usize,
    pub insurance_plans: usize,
    pub clinics: usize,
    pub providers: usize,
    pub patients: usize,
    pub emergency_contacts: usize,
    pub patient_demographics: usize,
    pub patient_insurance: usize,
    pub clinic_schedules: usize,
    pub appointments: usize,
    pub medical_records: usize,
}

#[derive(Debug, Serialize)]
pub struct MessageResponse {
    pub message: String,
}
```

### HTTP Status Code Reference

| Code | Meaning |
|---|---|
| `201 Created` | Successful registration |
| `202 Accepted` | Simulation job accepted |
| `200 OK` | Successful login, /me, status, stop |
| `409 Conflict` | Simulator already running |
| `400 Bad Request` | Email already registered |
| `401 Unauthorized` | Wrong credentials or invalid/expired JWT |
| `404 Not Found` | User row missing |
| `500 Internal Server Error` | Database or bcrypt failure (message sanitised) |

---

## Database Schemas

### `vital_fold` schema — Simulation tables

Full DDL in `health_clinic_schema.sql`. Table insertion order (FK-safe):

1. `insurance_company` — no dependencies
2. `insurance_plan` — depends on `insurance_company`
3. `provider` — no dependencies
4. `clinic` — no dependencies
5. `patient` — no dependencies (emergency_contact_id is VARCHAR, not FK)
6. `emergency_contact` — depends on `patient`
7. `patient_demographics` — depends on `patient`
8. `patient_insurance` — depends on `patient` and `insurance_plan`
9. `clinic_schedule` — depends on `clinic` and `provider`
10. `appointment` — depends on `patient`, `provider`, `clinic`
11. `medical_record` — depends on `patient`, `provider`, `clinic`

**IMPORTANT:** The schema file `health_clinic_schema.sql` must define tables in this exact order to avoid FK constraint errors during DDL execution.

Relationships:
```
insurance_company
    └── insurance_plan (company_id → insurance_company)
            └── patient_insurance (insurance_plan_id → insurance_plan)
                    └── patient (patient_id → patient_insurance)

patient
    ├── emergency_contact (patient_id → patient)
    ├── patient_demographics (patient_id → patient)
    ├── patient_insurance (patient_id → patient)
    ├── appointment (patient_id → patient)
    └── medical_record (patient_id → patient)

provider
    ├── appointment (provider_id → provider)
    ├── clinic_schedule (provider_id → provider)
    └── medical_record (provider_id → provider)

clinic
    ├── appointment (clinic_id → clinic)
    ├── clinic_schedule (clinic_id → clinic)
    └── medical_record (clinic_id → clinic)
```

### `public` schema — Auth tables (`migrations/001_init.sql`)

```sql
CREATE EXTENSION IF NOT EXISTS "uuid-ossp";

CREATE TABLE IF NOT EXISTS users (
    id            UUID         PRIMARY KEY DEFAULT uuid_generate_v4(),
    email         TEXT         NOT NULL UNIQUE,
    password_hash TEXT         NOT NULL,
    created_at    TIMESTAMPTZ  NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_users_email ON users (email);
```


---

## Module Specifications

### `src/engine_state.rs`

```rust
pub struct SimulatorState {
    pub running: AtomicBool,
    pub last_run: Mutex<Option<DateTime<Utc>>>,
    pub counts: Mutex<SimulationCounts>,
}
```

`SimulationCounts` holds the last-run row counts for each table. Registered as `web::Data<Arc<SimulatorState>>` in `main.rs`. The `POST /simulate` handler calls `running.compare_exchange(false, true, ...)` — if it returns `Err`, respond `409 Conflict`. The `POST /simulate/stop` handler sets `running.store(false, ...)`.

### `src/config.rs`

`Config` struct (derive `Debug`, `Clone`) loaded via `Config::from_env() -> anyhow::Result<Self>`. Panics fast if required vars are absent.

Fields: `host`, `port`, `dsql_endpoint`, `dsql_region`, `dsql_db_name`, `dsql_user`, `db_pool_size`, `jwt_secret`, `jwt_expiry_hours`.

### `src/errors.rs`

`AppError` enum via `thiserror`:
- Variants: `Database(String)`, `NotFound(String)`, `Unauthorized(String)`, `BadRequest(String)`, `Internal(String)`
- `From<tokio_postgres::Error>` and `From<deadpool_postgres::PoolError>` both map to `AppError::Database`
- Implements `actix_web::ResponseError`; 500 variants call `tracing::error!` before returning sanitised message
- All responses serialise as `{ "error": "<message>" }`

### `src/db/mod.rs`

`create_pool(cfg: &Config) -> Result<Pool, AppError>`:
1. Load AWS config with `aws_config::defaults(BehaviorVersion::latest())` and the configured region
2. Generate an IAM auth token via `AuthTokenGenerator` → `db_connect_admin_auth_token`
3. Build `deadpool_postgres::Config` (host, dbname, user, password=token, port=5432, `RecyclingMethod::Fast`)
4. Return `pool.create_pool(Some(Runtime::Tokio1), NoTls)`

> **⚠ IAM tokens expire in ~15 min.** The initial build generates one token at startup. For long-running services, add a background Tokio task to rebuild the pool on a schedule.

Type alias: `pub type DbPool = deadpool_postgres::Pool;`

### `src/middleware/auth.rs`

- **`Claims`** (`Serialize`, `Deserialize`, `Clone`, `Debug`): `sub: String`, `email: String`, `exp: i64`, `iat: i64`
- **`generate_token(user_id, email, cfg)`** — builds `Claims`, encodes with HS256 via `EncodingKey::from_secret`
- **`validate_token(token, secret)`** — decodes with `Validation::default()`, maps errors to `AppError::Unauthorized`
- **`jwt_validator`** — `actix-web-httpauth` extractor; inserts `Claims` into `req.extensions_mut()` on success

### `src/models/user.rs`

| Struct | Derives | Key Notes |
|---|---|---|
| `User` | `Serialize, Deserialize, Clone, Debug, sqlx::FromRow` | `password_hash` carries `#[serde(skip_serializing)]` |
| `RegisterRequest` | `Deserialize, Debug` | `email: String`, `password: String` |
| `LoginRequest` | `Deserialize, Debug` | `email: String`, `password: String` |
| `AuthResponse` | `Serialize, Debug` | `token: String`, `user: UserProfile` |
| `UserProfile` | `Serialize, Debug, Clone` | `id: Uuid`, `email: String`, `created_at: DateTime<Utc>` |

Implement `From<User> for UserProfile`.

### `src/models/insurance.rs`

```rust
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct InsuranceCompany {
    pub company_id: Uuid,
    pub company_name: String,
    pub email: String,
    pub phone_number: String,
    pub tax_id_number: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct InsurancePlan {
    pub insurance_plan_id: Uuid,
    pub plan_name: String,
    pub company_id: Uuid,
    pub deductible_amount: BigDecimal,
    pub copay_amount: BigDecimal,
    pub prior_auth_required: bool,
    pub active_plan: bool,
    pub active_start_date: NaiveDate,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct PatientInsurance {
    pub patient_insurance_id: Uuid,
    pub patient_id: Uuid,
    pub insurance_plan_id: Uuid,
    pub policy_number: String,
    pub coverage_start_date: NaiveDate,
    pub coverage_end_date: Option<NaiveDate>,
}
```

### `src/models/patient.rs`

```rust
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Patient {
    pub patient_id: Uuid,
    pub first_name: String,
    pub last_name: String,
    pub middle_name: Option<String>,
    pub date_of_birth: NaiveDate,
    pub street_address: String,
    pub city: String,
    pub state: String,
    pub zip_code: String,
    pub phone_number: String,
    pub email: String,
    pub registration_date: NaiveDate,
    pub emergency_contact_id: String,  // VARCHAR, not UUID FK
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct EmergencyContact {
    pub emergency_contact_id: Uuid,
    pub patient_id: Uuid,
    pub first_name: String,
    pub last_name: String,
    pub relationship: String,
    pub phone_number: String,
    pub email: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct PatientDemographics {
    pub demographics_id: Uuid,
    pub patient_id: Uuid,
    pub first_name: String,
    pub last_name: String,
    pub date_of_birth: NaiveDate,
    pub age: i32,
    pub ssn: String,
    pub ethnicity: String,
    pub birth_gender: String,
}
```

### `src/models/provider.rs`

```rust
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Provider {
    pub provider_id: Uuid,
    pub first_name: String,
    pub last_name: String,
    pub specialty: String,
    pub license_type: String,
    pub phone_number: String,
    pub email: String,
}
```

### `src/models/clinic.rs`

```rust
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Clinic {
    pub clinic_id: Uuid,
    pub clinic_name: String,
    pub region: String,
    pub street_address: String,
    pub city: String,
    pub state: String,
    pub zip_code: String,
    pub phone_number: String,
    pub email: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct ClinicSchedule {
    pub schedule_id: Uuid,
    pub clinic_id: Uuid,
    pub provider_id: Uuid,
    pub day_of_week: String,
    pub start_time: NaiveTime,
    pub end_time: NaiveTime,
}
```

### `src/models/appointment.rs`

```rust
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Appointment {
    pub appointment_id: Uuid,
    pub patient_id: Uuid,
    pub provider_id: Uuid,
    pub clinic_id: Uuid,
    pub appointment_date: NaiveDateTime,
    pub reason_for_visit: String,
}
```

### `src/models/medical_record.rs`

```rust
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct MedicalRecord {
    pub medical_record_id: Uuid,
    pub patient_id: Uuid,
    pub provider_id: Uuid,
    pub clinic_id: Uuid,
    pub record_date: NaiveDateTime,
    pub diagnosis: String,
    pub treatment: String,
}
```

### `src/models/mod.rs`

```rust
pub mod user;
pub mod insurance;
pub mod patient;
pub mod provider;
pub mod clinic;
pub mod appointment;
pub mod medical_record;

pub use user::*;
pub use insurance::*;
pub use patient::*;
pub use provider::*;
pub use clinic::*;
pub use appointment::*;
pub use medical_record::*;
```

### `src/handlers/auth.rs`

**`register`**: check email uniqueness → hash password (bcrypt `DEFAULT_COST`) → insert user → return `201 + AuthResponse`

**`login`**: lookup by email (missing → `Unauthorized`) → verify bcrypt (fail → `Unauthorized`) → return `200 + AuthResponse`

Both endpoints return the **same** error message for unknown email and wrong password to prevent user enumeration.

### `src/handlers/user.rs`

**`me`**: extract `Claims` from extensions → parse `claims.sub` as `Uuid` → SELECT user → return `200 + UserProfile`

---

## Simulation Logic

### Generator Module Specifications

Each generator file in `src/generators/` must implement a single public async function that takes a reference to `SimulationContext` and returns `Result<(), AppError>`.

**`src/generators/mod.rs`** — SimulationContext and orchestration:
```rust
pub struct SimulationContext {
    pub pool: sqlx::PgPool,
    pub dynamo_client: aws_sdk_dynamodb::Client,
    pub config: SimulationConfig,
    // Cached IDs for FK references
    pub company_ids: Vec<Uuid>,
    pub plan_ids: Vec<Uuid>,
    pub clinic_ids: Vec<Uuid>,
    pub provider_ids: Vec<Uuid>,
    pub patient_ids: Vec<Uuid>,
    // Cached patient data (id, first_name, last_name, dob) for demographics generation
    pub patient_data: Vec<(Uuid, String, String, NaiveDate)>,
}

/// Defaults: plans_per_company=3, providers=50, patients=50000,
///           appointments_per_patient=2, records_per_appointment=1
pub struct SimulationConfig {
    pub plans_per_company: usize,  // default: 3
    pub providers: usize,           // default: 50
    pub patients: usize,            // default: 50000
    pub appointments_per_patient: usize, // default: 2
    pub records_per_appointment: usize,  // default: 1
}

pub async fn run_simulation(ctx: &mut SimulationContext, state: &SimulatorState) -> Result<SimulationCounts, AppError>;
```

**`src/generators/insurance.rs`**:
```rust
pub async fn generate_insurance(ctx: &mut SimulationContext) -> Result<(), AppError>;
```
- Insert all 7 insurance companies from the fixed list
- Generate `plans_per_company` plans for each company
- Store returned UUIDs in `ctx.company_ids` and `ctx.plan_ids`

**`src/generators/clinic.rs`**:
```rust
pub async fn generate_clinics(ctx: &mut SimulationContext) -> Result<(), AppError>;
```
- Insert exactly 10 clinics per the fixed location distribution
- Generate realistic clinic names like "VitalFold Heart Center - {City}"
- Store returned UUIDs in `ctx.clinic_ids`

**`src/generators/provider.rs`**:
```rust
pub async fn generate_providers(ctx: &mut SimulationContext) -> Result<(), AppError>;
```
- Generate `config.providers` providers with fake names
- Specialty should be cardiac-related: "Cardiologist", "Cardiac Surgeon", "Electrophysiologist", "Interventional Cardiologist"
- License types: "MD", "DO"
- Store returned UUIDs in `ctx.provider_ids`

**`src/generators/patient.rs`**:
```rust
pub async fn generate_patients(ctx: &mut SimulationContext) -> Result<(), AppError>;
```
- Generate `config.patients` patients
- For each patient, also insert: emergency_contact, patient_demographics, patient_insurance
- Assign each patient to one insurance plan randomly
- Store returned patient UUIDs in `ctx.patient_ids`

**`src/generators/appointment.rs`**:
```rust
pub async fn generate_appointments(ctx: &mut SimulationContext) -> Result<(), AppError>;
```
- First, generate clinic_schedule rows (provider-clinic-day assignments)
- Then generate `appointments_per_patient` appointments per patient
- Each appointment randomly selects a provider and clinic
- After each appointment INSERT, write to DynamoDB `patient_visit` and `patient_vitals`

**`src/generators/medical_record.rs`**:
```rust
pub async fn generate_medical_records(ctx: &mut SimulationContext) -> Result<(), AppError>;
```
- Generate `records_per_appointment` medical records per appointment
- Diagnosis drawn randomly from the 8 cardiac codes
- Treatment matches the diagnosis per the treatment mapping

### Engine On/Off Control
The simulation engine has a toggleable running state stored in an `AtomicBool` (or `Mutex<bool>`) registered as `web::Data`. The `POST /simulate` endpoint checks this flag before starting a run and returns `409 Conflict { "error": "simulator is already running" }` if a run is in progress. The `POST /simulate/stop` endpoint sets the flag to false, gracefully ending the current run after the current batch completes.

### Data Generation Rules

**Execution Model:**
- Each simulation run executes **sequentially** within a single async task to respect FK constraints.
- All inserts use **bulk UNNEST inserts** via SQLx: `INSERT INTO ... SELECT * FROM UNNEST($1::type[], ...)`.
- **Aurora DSQL has a hard limit of 3000 rows per transaction statement.** All bulk inserts must be chunked using `DSQL_BATCH_SIZE = 2500` (headroom below the limit). Use `(0..n).step_by(DSQL_BATCH_SIZE)` to iterate chunks.
- Generators build all row data in-memory first (in a synchronous block so `ThreadRng` is dropped before any `.await`), then loop over chunks issuing one `UNNEST` insert per chunk.
- UUIDs are generated by PostgreSQL (`gen_random_uuid()`) — Rust omits the `*_id` columns during INSERT and uses `RETURNING` to capture generated UUIDs for FK references.

**Fake Data Generation:**
- The `fake` crate provides locale-aware fake data: names, addresses, phone numbers, emails, dates.
- Use `fake::faker::name::en::*` for English names.
- Use `fake::faker::phone_number::en::PhoneNumber` for phone numbers.
- Use `fake::faker::internet::en::SafeEmail` for emails.

**Table-Specific Rules:**
- `patient_demographics.age` is derived from `date_of_birth` at insert time: `(today - dob).years()`.
- `patient_insurance.coverage_end_date` is nullable — ~20% of records have a non-null end date (expired coverage).
- `clinic_schedule` generates one row per provider per clinic per assigned weekday. Assign each provider to 1-2 clinics randomly, working 3-5 days per week.
- `patient.emergency_contact_id` is populated AFTER inserting the corresponding `emergency_contact` row — use the returned UUID as a string.

**Fixed Domain Values:**
- Insurance companies are seeded from the **fixed list** in "Synthetic Data Domain Values" — do not generate random carrier names.
- Diagnosis codes on `medical_record` must be drawn from the **fixed cardiac diagnosis list** — select randomly from those 8 values.
- Clinics are seeded with the **exact city/state/count distribution** from "Synthetic Data Domain Values".

**Geographic Distribution:**
- Patient addresses should loosely match the metro area of the clinic they are assigned to.
- Assign patients to clinics proportionally: more patients for clinics in larger cities (Miami, Atlanta, Jacksonville).

**Treatment Generation:**
- For `medical_record.treatment`, generate realistic cardiac treatments matching the diagnosis:
  - AFib: "Anticoagulation therapy", "Rate control medication", "Cardioversion"
  - CAD: "Statin therapy", "Angioplasty referral", "Lifestyle modification"
  - Chest Pain: "Stress test ordered", "ECG monitoring", "Nitroglycerin PRN"
  - Hypertension: "ACE inhibitor", "Beta blocker", "Lifestyle counseling"
  - Hyperlipidemia: "Statin initiated", "Dietary counseling", "Lipid panel follow-up"
  - SOB: "Pulmonary function test", "Echocardiogram ordered", "Diuretic therapy"
  - Tachycardia: "Beta blocker", "Electrophysiology referral", "Holter monitor"
  - Bradycardia: "Pacemaker evaluation", "Medication review", "Holter monitor"

### DynamoDB Write Rules
After each appointment is inserted into Aurora DSQL, write two corresponding DynamoDB items:
- `patient_visit` table: write one item per appointment (PK: `patient_id`, SK: `clinic_id`). Populate `checkin_time`, `checkout_time`, `provider_seen_time` as generated ISO 8601 timestamps; randomly set `ekg_usage` (20% true) and generate a realistic `estimated_copay` (range $20–$150).
- `patient_vitals` table: write one item per appointment (PK: `patient_id`, SK: `clinic_id`). Include `visit_id` linking back to the appointment UUID. Generate realistic cardiac-range vitals: heart rate 50–120 bpm, SpO2 92–100%, temperature 97.0–99.5°F, blood pressure formatted as `"SYS/DIA"`.
- DynamoDB writes are **fire-and-forget** — log errors with `tracing::warn!` but do not fail the simulation run if a DynamoDB write fails.

## Configuration (Environment Variables)

| Variable | Required | Description | Default |
|---|---|---|---|
| `HOST` | No | Bind address | `0.0.0.0` |
| `PORT` | No | Bind port | `8787` |
| `DATABASE_URL` | For SQLx | Full PostgreSQL connection string for simulation DB | — |
| `DSQL_CLUSTER_ENDPOINT` | **Yes** | Aurora DSQL hostname | — |
| `DSQL_REGION` | No | AWS region for token signing | `us-east-1` |
| `DSQL_DB_NAME` | No | Postgres database name | `postgres` |
| `DSQL_USER` | No | Postgres user | `admin` |
| `DB_POOL_SIZE` | No | deadpool max pool size | `10` |
| `JWT_SECRET` | **Yes** | HMAC secret for HS256 signing | — |
| `JWT_EXPIRY_HOURS` | No | Token lifetime in hours | `24` |
| `RUST_LOG` | No | Log level | `info` |

---

## Aurora DSQL IAM Authentication

Aurora DSQL replaces static passwords with short-lived IAM-signed tokens. `db::create_pool()` handles this at startup.

### AWS Credential Lookup Order

1. Environment variables: `AWS_ACCESS_KEY_ID`, `AWS_SECRET_ACCESS_KEY`, `AWS_SESSION_TOKEN`
2. Shared credentials file: `~/.aws/credentials`
3. AWS SSO / IAM Identity Center
4. EC2/ECS/Lambda instance metadata (IAM role)

### Required IAM Permission

```json
{
  "Effect": "Allow",
  "Action": "dsql:DbConnectAdmin",
  "Resource": "arn:aws:dsql:<region>:<account>:cluster/<cluster-id>"
}
```

---

## Render.com Deployment

- Service type: **Web Service**
- Runtime: **Docker** (preferred) or Rust native buildpack
- Start command: `./vital-fold-engine`
- Health check path: `/health`

### render.yaml
```yaml
services:
  - type: web
    name: vitalfold-engine
    runtime: docker
    healthCheckPath: /health
    envVars:
      - key: DATABASE_URL
        fromDatabase:
          name: vitalfold-db
          property: connectionString
      - key: RUST_LOG
        value: info

databases:
  - name: vitalfold-db
    databaseName: vitalfold
    user: vitalfold
```

---

## Key Dependencies (Cargo.toml)

**IMPORTANT:** Use Rust edition `2021` (not `2024` — that edition does not exist).

```toml
[package]
name = "vital-fold-engine"
version = "0.1.0"
edition = "2021"

[dependencies]
actix-web = "4"
actix-web-httpauth = "0.8"
tokio = { version = "1", features = ["full"] }
tokio-postgres = { version = "0.7", features = ["with-uuid-1", "with-chrono-0_4"] }
deadpool-postgres = "0.12"
sqlx = { version = "0.8.6", features = [
    "postgres", "runtime-tokio-rustls",
    "uuid", "chrono", "bigdecimal", "macros", "derive"
] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
jsonwebtoken = "9"
bcrypt = "0.15"
anyhow = "1"
duckdb = { version = "1.4.4", features = ["bundled"] }
polars = { version = "0.53", features = ["parquet", "csv", "json"] }
uuid = { version = "1", features = ["v4", "serde"] }
chrono = { version = "0.4", features = ["serde"] }
fake = { version = "4", features = ["derive"] }
rand = "0.9"
config = "0.15"
dotenvy = "0.15"
tracing = "0.1"
tracing-actix-web = "0.7"
tracing-subscriber = { version = "0.3", features = ["env-filter"] }
thiserror = "2"

# AWS SDK
aws-config = { version = "1", features = ["behavior-version-latest"] }
aws-sdk-dsql = "1"
aws-sdk-dynamodb = "1"
aws-sdk-rds = "1"
aws-credential-types = "1"

# OpenAPI / Swagger
utoipa = { version = "5", features = ["actix_extras", "chrono", "uuid"] }
utoipa-swagger-ui = { version = "9", features = ["actix-web"] }
utoipa-actix-web = "0.1"
```

---

## Common Imports Reference

Include these imports as needed in each module:

```rust
// Core types
use uuid::Uuid;
use chrono::{DateTime, Utc, NaiveDate, NaiveDateTime, NaiveTime};
use sqlx::types::BigDecimal;

// Serde
use serde::{Deserialize, Serialize};

// Actix
use actix_web::{web, HttpResponse, HttpRequest};
use actix_web::http::StatusCode;

// Error handling
use anyhow::Result;
use thiserror::Error;

// Async
use tokio::sync::Mutex;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

// Database
use sqlx::PgPool;
use deadpool_postgres::Pool as DeadPool;

// Fake data
use fake::{Fake, Faker};
use fake::faker::name::en::{FirstName, LastName};
use fake::faker::phone_number::en::PhoneNumber;
use fake::faker::internet::en::SafeEmail;
use fake::faker::address::en::{StreetName, CityName, StateAbbr, ZipCode};
use rand::Rng;

// AWS
use aws_sdk_dynamodb::Client as DynamoClient;
use aws_sdk_dsql::auth_token::AuthTokenGenerator;

// Tracing
use tracing::{info, warn, error, instrument};
```

## Code Style and Conventions

- Use `async`/`await` throughout; no blocking calls on the async executor.
- All database errors propagate via a unified `AppError` type implementing `actix_web::ResponseError`.
- Generators return `Result<(), AppError>` and receive a reference to the SQLx pool.
- Keep generator functions pure — no global state; pass `SimulationContext` explicitly.
- Use `tracing` macros (`info!`, `warn!`, `error!`) — never `println!`.
- `tracing::error!` must be called inside `AppError`'s `ResponseError` impl before any 500 response.
- All struct fields use `snake_case` mirroring SQL column names.
- SQL queries use the `sqlx::query!` macro where the schema is known at compile time.
- Both web scopes (`/api/v1/auth` and `/api/v1`) share the same `HttpServer::new` closure so they share `app_data`.
- `password_hash` on `User` must carry `#[serde(skip_serializing)]` — never returned in responses.

---

## DynamoDB Tables

Schema source: `dynamo.json` in the repo root. Both tables use on-demand (PAY_PER_REQUEST) billing mode. Region matches `DSQL_REGION`.

### `patient_visit`

| Attribute | Type | Role |
|---|---|---|
| `patient_id` | String (UUID) | Partition key |
| `clinic_id` | String (UUID) | Sort key |
| `provider_id` | String (UUID) | |
| `checkin_time` | String (ISO 8601) | |
| `checkout_time` | String (ISO 8601) | |
| `provider_seen_time` | String (ISO 8601) | |
| `ekg_usage` | Boolean | Whether an EKG was performed |
| `estimated_copay` | Float | Estimated patient copay amount |

### `patient_vitals`

| Attribute | Type | Role |
|---|---|---|
| `patient_id` | String (UUID) | Partition key |
| `clinic_id` | String (UUID) | Sort key |
| `provider_id` | String (UUID) | |
| `visit_id` | String (UUID) | Links to `patient_visit` |
| `height` | Decimal | In inches |
| `weight` | Decimal | In pounds (note: `wieght` typo preserved from source schema) |
| `blood_pressure` | String | Format: `"120/80"` |
| `heart_rate` | Decimal | Beats per minute |
| `temperature` | Decimal | In Fahrenheit |
| `oxygen` | Decimal | SpO2 percentage |
| `pulses` | Decimal | Pulse rate |

---

## Notes and Constraints

- **Aurora DSQL 3000-row limit** — every `INSERT` statement (including `UNNEST` bulk inserts) is limited to 3000 rows per transaction. Use `const DSQL_BATCH_SIZE: usize = 2500` in each generator and chunk all bulk inserts with `(0..n).step_by(DSQL_BATCH_SIZE)`.
- `vital_fold.patient.emergency_contact_id` is `VARCHAR(255)` — populate with the string UUID of the generated contact after the `emergency_contact` insert.
- `patient_demographics` duplicates `first_name`, `last_name`, `date_of_birth` from `patient` — generate from the same source values.
- `tax_id_number` on `insurance_company` is `INT` — generate realistic 9-digit integers.
- `ssn` on `patient_demographics` is `VARCHAR(11)` — format as `XXX-XX-XXXX`.
- Index creation uses `CREATE INDEX ASYNC` — CockroachDB/YugabyteDB syntax. Confirm Postgres dialect before running migrations in production.
- Aurora DSQL IAM tokens expire in ~15 minutes. `db::spawn_token_refresh_task(pool.clone(), config.clone())` must be called in `main.rs` immediately after pool creation. It runs a Tokio background loop calling `pool.set_connect_options(new_opts)` every 12 minutes, pushing a fresh token into the pool without replacing it — all `web::Data<DbPool>` clones stay valid. Without this, any simulation run after the first token expires gets "access denied".
- The `register` and `login` handlers return the same `Unauthorized("Invalid credentials")` message for both unknown email and wrong password — prevents user enumeration.

### Haiku Implementation Order

Haiku should build modules in this sequence to avoid circular dependencies and allow incremental `cargo check` validation at each step:

1. `Cargo.toml` — all dependencies (use edition = "2021", NOT "2024")
2. `src/errors.rs` — `AppError` (no other local deps)
3. `src/config.rs` — `Config::from_env()` (depends on nothing)
4. `src/db/mod.rs` — pool setup (depends on `config`, `errors`)
5. `src/engine_state.rs` — `SimulatorState` (depends on `chrono`)
6. `src/models/mod.rs` + all model files
7. `src/middleware/mod.rs` + `auth.rs`
8. `src/handlers/mod.rs` + `health.rs` — simplest handler first
9. `src/handlers/auth.rs` + `user.rs`
10. `src/generators/mod.rs` — `SimulationContext` struct
11. `src/generators/insurance.rs` — insurance_company + insurance_plan
12. `src/generators/clinic.rs` — clinic (fixed locations)
13. `src/generators/provider.rs` — provider
14. `src/generators/patient.rs` — patient + emergency_contact + patient_demographics + patient_insurance
15. `src/generators/appointment.rs` — clinic_schedule + appointment
16. `src/generators/medical_record.rs` — medical_record
17. `src/handlers/simulation.rs`
18. `src/routes.rs` — wire everything together
19. `src/main.rs` — bootstrap
20. `migrations/001_init.sql`
21. `.env.example`

**Run `cargo check` after completing each numbered step** to catch errors early.

### `.env.example` Template

```env
# Server
HOST=0.0.0.0
PORT=8787

# Aurora DSQL
DSQL_CLUSTER_ENDPOINT=your-cluster.dsql.us-east-1.on.aws
DSQL_REGION=us-east-1
DSQL_DB_NAME=postgres
DSQL_USER=admin
DB_POOL_SIZE=10

# SQLx (for compile-time query checking)
DATABASE_URL=postgres://admin:token@your-cluster.dsql.us-east-1.on.aws:5432/postgres

# Auth
JWT_SECRET=your-secret-key-minimum-32-characters-long
JWT_EXPIRY_HOURS=24

# Logging
RUST_LOG=info

# AWS (if not using instance profile)
# AWS_ACCESS_KEY_ID=
# AWS_SECRET_ACCESS_KEY=
# AWS_REGION=us-east-1
```

### `src/main.rs` Bootstrap Structure

```rust
use actix_web::{web, App, HttpServer, middleware::Logger};
use actix_web_httpauth::middleware::HttpAuthentication;
use std::sync::Arc;
use tracing_actix_web::TracingLogger;

mod config;
mod db;
mod engine_state;
mod errors;
mod generators;
mod handlers;
mod middleware;
mod models;
mod routes;

use config::Config;
use db::create_pool;
use engine_state::SimulatorState;
use middleware::auth::jwt_validator;

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    // Initialize tracing
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    // Load config
    dotenvy::dotenv().ok();
    let config = Config::from_env().expect("Failed to load configuration");

    // Create database pool
    let pool = create_pool(&config).await.expect("Failed to create database pool");

    // Create DynamoDB client
    let aws_config = aws_config::defaults(aws_config::BehaviorVersion::latest())
        .region(aws_config::Region::new(config.dsql_region.clone()))
        .load()
        .await;
    let dynamo_client = aws_sdk_dynamodb::Client::new(&aws_config);

    // Create simulator state
    let simulator_state = Arc::new(SimulatorState::new());

    let bind_addr = format!("{}:{}", config.host, config.port);
    tracing::info!("Starting server at {}", bind_addr);

    HttpServer::new(move || {
        let auth = HttpAuthentication::bearer(jwt_validator);

        App::new()
            .wrap(TracingLogger::default())
            .app_data(web::Data::new(pool.clone()))
            .app_data(web::Data::new(dynamo_client.clone()))
            .app_data(web::Data::new(config.clone()))
            .app_data(web::Data::new(simulator_state.clone()))
            .configure(routes::configure_routes)
    })
    .bind(&bind_addr)?
    .run()
    .await
}
```

### `src/routes.rs` Structure

```rust
use actix_web::web;
use actix_web_httpauth::middleware::HttpAuthentication;
use crate::middleware::auth::jwt_validator;
use crate::handlers::{auth, health, simulation, user};

pub fn configure_routes(cfg: &mut web::ServiceConfig) {
    cfg
        // Public routes
        .route("/health", web::get().to(health::health_check))
        .service(
            web::scope("/api/v1/auth")
                .route("/register", web::post().to(auth::register))
                .route("/login", web::post().to(auth::login))
        )
        // Protected routes
        .service(
            web::scope("/api/v1")
                .wrap(HttpAuthentication::bearer(jwt_validator))
                .route("/me", web::get().to(user::me))
        )
        .service(
            web::scope("/simulate")
                .wrap(HttpAuthentication::bearer(jwt_validator))
                .route("", web::post().to(simulation::start_simulation))
                .route("/stop", web::post().to(simulation::stop_simulation))
                .route("/status", web::get().to(simulation::get_status))
                .route("/reset", web::delete().to(simulation::reset_data))
        );
}
```

### Out of Scope (not implemented in initial build)

- Refresh token rotation
- Rate limiting
- ~~Automatic IAM token renewal background task~~ (**implemented** — `db::spawn_token_refresh_task`)
- Integration tests
- Docker configuration

---

## Self-Improvement Instructions for Haiku

This section contains meta-instructions for Haiku to follow during and after implementation.

### During Implementation

1. **Compile Early, Compile Often** — After writing each module, run `cargo check` before moving to the next. Fix all errors immediately. Do not accumulate technical debt across modules.

2. **When Stuck on a Compilation Error:**
   - Read the full error message carefully
   - Check if the issue is a missing import, wrong type, or lifetime issue
   - If the error references a type from another module, verify that module exports it via `pub`
   - For trait bounds errors, check if the required derive macros are present
   - Add a `// TODO: <description>` comment if you make a temporary workaround

3. **When Unsure About Implementation Details:**
   - Default to the simplest correct implementation
   - Prefer explicit types over inference when debugging
   - Use `.clone()` liberally during initial implementation; optimize later
   - If a section of this document is ambiguous, document your interpretation with a comment

4. **Database Query Strategy:**
   - Use `sqlx::query!` macro for compile-time checked queries when `DATABASE_URL` is set
   - Fall back to `sqlx::query_as::<_, ModelType>()` with raw SQL strings if compile-time checking is unavailable
   - Always use parameterized queries (`$1`, `$2`, etc.) — never string interpolation

5. **Error Handling Pattern:**
   - Use `?` operator for propagation within handlers
   - Map external errors to `AppError` variants using `From` impls
   - Log errors with `tracing::error!` before returning 500s
   - Never expose internal error details to API responses

### After Implementation

1. **Update This Document** — If you discover that any specification was incorrect or incomplete, add a note in the relevant section describing the actual implementation and why it differs.

2. **Document Deviations** — If you had to deviate from this spec due to compilation errors, library incompatibilities, or other technical constraints, add an entry to a new "Implementation Notes" section at the end of this file.

3. **Flag Improvements** — If you identify opportunities for improvement (better error messages, more efficient queries, cleaner abstractions), add them to a "Future Improvements" section rather than implementing them immediately.

### Common Pitfalls to Avoid

| Pitfall | Solution |
|---|---|
| `sqlx::FromRow` not deriving correctly | Ensure field names match SQL column names exactly (snake_case) |
| `BigDecimal` type mismatch | Import from `sqlx::types::BigDecimal`, not `bigdecimal` crate directly |
| `NaiveTime` serialization issues | Add `#[serde(with = "chrono::serde::ts_seconds_option")]` if needed for Option types |
| `AtomicBool` in async context | Use `Ordering::SeqCst` for simplicity; `Acquire`/`Release` only if performance-critical |
| JWT validation failing | Ensure `exp` claim is future timestamp; check clock skew tolerance in `Validation` |
| DynamoDB attribute types | All numbers must be sent as strings in DynamoDB SDK; use `.n()` not `.s()` |
| Pool exhaustion | Set reasonable `DB_POOL_SIZE` (10-20); use `timeout` on pool checkout |
| `fake` crate locale issues | Always use `::en::` module for English locale fakers |
| DSQL 3000-row limit exceeded | Chunk all UNNEST inserts with `DSQL_BATCH_SIZE = 2500`; use `(0..n).step_by(DSQL_BATCH_SIZE)` |
| `ThreadRng` not `Send` across `.await` | Build all row data in a synchronous block `{ let mut rng = thread_rng(); ... }` so rng is dropped before the first `.await` |
| `ctid` not supported on DSQL | DSQL is distributed — physical row pointers don't exist. For batched deletes use `DELETE FROM t WHERE pk IN (SELECT pk FROM t LIMIT 2500)` in a loop until `rows_affected() == 0`. TRUNCATE is also unsupported. |

### Checklist Before Completion

- [ ] All 21 implementation steps completed
- [ ] `cargo check` passes with no errors
- [ ] `cargo clippy` has no warnings (or warnings are documented)
- [ ] All endpoints respond correctly to manual curl tests
- [ ] Health check returns `{"status": "ok"}`
- [ ] Registration creates user and returns JWT
- [ ] Login validates credentials and returns JWT
- [ ] Protected endpoints reject requests without valid Bearer token
- [ ] Simulation starts, populates data, and respects stop signal
- [ ] DynamoDB writes succeed (or fail gracefully with warning logs)
- [ ] `.env.example` contains all required variables

---

## Implementation Notes

_This section is for Haiku to document any deviations, issues encountered, or clarifications discovered during implementation._

### Completed Steps

✅ **Step 1: Cargo.toml** — Updated edition to "2021", added all required dependencies

✅ **Step 2: src/errors.rs** — Complete AppError enum with:
- 5 error variants: Database, NotFound, Unauthorized, BadRequest, Internal
- ResponseError trait impl for Actix integration
- From impls for automatic error conversion (tokio_postgres, deadpool_postgres, sqlx, bcrypt, jsonwebtoken, anyhow)
- Safe client message method (doesn't expose internals)
- Proper logging of 500 errors before response

✅ **Step 3: src/config.rs** — Complete Config struct with:
- All required fields (host, port, dsql_endpoint, dsql_region, dsql_db_name, dsql_user, db_pool_size, jwt_secret, jwt_expiry_hours)
- from_env() method with proper error handling
- JWT secret minimum length validation (32 chars)
- Sensible defaults for most fields
- Environment variable parsing with type validation

✅ **Step 4: src/db/mod.rs** — Database pool with Aurora DSQL IAM auth:
- DbPool type alias for deadpool_postgres::Pool
- create_pool() async function for connection pool setup
- AWS credentials and IAM token generation
- RecyclingMethod::Fast for connection reuse
- Proper error logging and propagation
- Documented token expiration (~15 min) with note about refresh

✅ **Step 5: src/engine_state.rs** — Simulator state management:
- SimulationCounts struct for tracking last run metrics (11 table counts)
- SimulatorState with AtomicBool running flag, Mutex-protected last_run and counts
- Helper methods: is_running(), try_start(), stop(), get_last_run(), set_last_run(), get_counts(), set_counts()
- Comprehensive unit tests for state transitions and data updates
- Thread-safe with SeqCst ordering for simplicity

✅ **Step 6: src/models/** — All domain models (7 files):
- **mod.rs**: Exports all model submodules
- **user.rs**: User (with password_hash skip_serializing), RegisterRequest, LoginRequest, AuthResponse, UserProfile with From<User> impl
- **insurance.rs**: InsuranceCompany, InsurancePlan, PatientInsurance with BigDecimal for financial fields
- **patient.rs**: Patient, EmergencyContact, PatientDemographics (note: emergency_contact_id is VARCHAR)
- **provider.rs**: Provider with specialty and license_type fields
- **clinic.rs**: Clinic, ClinicSchedule with NaiveTime for hours
- **appointment.rs**: Appointment with reason_for_visit
- **medical_record.rs**: MedicalRecord with diagnosis and treatment fields
- All models derive Debug, Clone, Serialize, Deserialize, sqlx::FromRow

✅ **Step 7: src/middleware/** — JWT authentication:
- **mod.rs**: Exports auth module
- **auth.rs**: Claims struct, generate_token(), validate_token(), jwt_validator() extractor
- Comprehensive unit tests: token generation/validation, invalid token handling, wrong secret
- Proper error logging and Unauthorized responses
- HS256 encoding with configurable expiry from config

✅ **Step 8: src/handlers/health.rs** — Health check endpoint:
- health_check() handler returns {"status": "ok"}
- Simple validation that service is running
- Proper logging

✅ **Step 9: src/handlers/auth.rs & user.rs** — Authentication handlers:
- **auth.rs**: register() and login() handlers
  - register: email uniqueness check, bcrypt password hashing, JWT generation, 201 response
  - login: email lookup, bcrypt verification, same error for both unknown email and wrong password (enumeration prevention)
- **user.rs**: me() handler
  - Extracts Claims from extensions, parses user ID, queries database, returns UserProfile
  - 404 if user not found, 401 if token invalid
- **simulation.rs**: Placeholder handlers (to be implemented later)

---

## Future Improvements

_This section is for Haiku to document potential improvements identified during implementation that are out of scope for the initial build._

<!-- Haiku: Add improvement ideas here -->
