# `src/generators/` — Claude Context

> **Purpose:** Self-contained reference for the `src/generators/` subdirectory. Claude or Haiku can work on any generator file using only this document — no need to load the root `claude.md`.

---

## Overview

The generators module is the core synthetic data pipeline for VitalFold Engine. It produces realistic healthcare data and loads it into two storage systems:

- **Aurora DSQL** (Postgres-compatible) — all relational tables in the `vital_fold` schema
- **DynamoDB** — `patient_visit` table (vitals embedded as attributes)

### Two-Phase Lifecycle

**Phase 1 — `POST /populate` → `run_populate()`**
Seeds all Aurora DSQL tables. Appointments are dated 0–89 days in the future. **No DynamoDB writes.**

**Phase 2 — `POST /simulate` → `run_simulate()`**
Called on any given calendar day. Queries Aurora for today's patient_visits (with embedded vitals), then writes one DynamoDB record per visit (patient_visit with vital attributes). Run once per day to simulate real-time visit data capture.

### FK-Safe Execution Order (run_populate)

```
1. generate_insurance_companies   → ctx.company_ids
2. generate_insurance_plans       → ctx.plan_ids       (needs company_ids)
3. generate_clinics               → ctx.clinic_ids
4. generate_providers             → ctx.provider_ids
5. generate_patients              → ctx.patient_ids, ctx.patient_data
                                    (emergency contacts inserted inline)
6. generate_emergency_contacts    → no-op (done inside generate_patients)
7. generate_patient_demographics  → (needs patient_data)
8. generate_patient_insurance     → (needs patient_ids, plan_ids)
9. generate_clinic_schedules      → (needs provider_ids, clinic_ids)
10. generate_appointments         → (needs patient_ids, provider_ids, clinic_ids)
11. generate_medical_records      → (queries appointments from DB)
12. generate_patient_visits       → (queries appointments from DB, generates vitals inline)
```

---

## SimulationContext and SimulationConfig

**File:** `src/generators/mod.rs`

```rust
pub struct SimulationConfig {
    pub plans_per_company:        usize,  // default: 3
    pub providers:                usize,  // default: 50
    pub patients:                 usize,  // default: 50_000
    pub appointments_per_patient: usize,  // default: 2
    pub records_per_appointment:  usize,  // default: 1
}
```

```rust
pub struct SimulationContext {
    pub pool:           DbPool,        // sqlx::PgPool
    pub dynamo_client:  DynamoClient,  // aws_sdk_dynamodb::Client
    pub config:         SimulationConfig,
    pub counts:         SimulationCounts,  // incremented by each generator

    // Populated by generators in order — used as FK input by later generators:
    pub company_ids:    Vec<Uuid>,  // set by generate_insurance_companies
    pub plan_ids:       Vec<Uuid>,  // set by generate_insurance_plans
    pub clinic_ids:     Vec<Uuid>,  // set by generate_clinics
    pub provider_ids:   Vec<Uuid>,  // set by generate_providers
    pub patient_ids:    Vec<Uuid>,  // set by generate_patients
    pub patient_data:   Vec<(Uuid, String, String, NaiveDate)>,
    //                    patient_id, first_name, last_name, dob
    //                    set by generate_patients, consumed by generate_patient_demographics
}
```

### Entry-Point Functions

```rust
// Phase 1: populate all Aurora DSQL tables
pub async fn run_populate(
    pool: DbPool,
    dynamo_client: DynamoClient,
    config: SimulationConfig,
    state: &SimulatorState,
) -> Result<(), AppError>

// Phase 2: write today's appointments to DynamoDB
pub async fn run_simulate(
    pool: DbPool,
    dynamo_client: DynamoClient,
    state: &SimulatorState,
) -> Result<(), AppError>
```

### DynamoDB Concurrency

`run_simulate` uses a semaphore-bounded task pool:

```rust
const DYNAMO_CONCURRENCY: usize = 128;
// At ~5ms/request: 128 / 0.005 ≈ 25,600 writes/sec — safely under DynamoDB's 30,000/sec limit
```

---

## Per-Generator Reference

### `insurance.rs`

**Public functions:**
```rust
pub async fn generate_insurance_companies(ctx: &mut SimulationContext) -> Result<(), AppError>
pub async fn generate_insurance_plans(ctx: &mut SimulationContext) -> Result<(), AppError>
```

**Reads from ctx:** nothing (seed data)

**Writes to ctx:** `ctx.company_ids`, `ctx.plan_ids`, `ctx.counts.insurance_companies`, `ctx.counts.insurance_plans`

**Tables (Aurora DSQL):**
- `vital_fold.insurance_company` — INSERT one row per company, `RETURNING company_id`
- `vital_fold.insurance_plan` — INSERT one row per company per plan, `RETURNING insurance_plan_id`

**Insurance — SQL columns:**
```
insurance_company: company_name, email, phone_number, tax_id_number (INT, 9-digit)
insurance_plan:    plan_name, company_id, deductible_amount (DECIMAL), copay_amount (DECIMAL),
                   prior_auth_required (BOOL), active_plan (BOOL), active_start_date (DATE)
```

**Fixed data — 7 company names (verbatim):**
```
"Orange Spear", "Care Medical", "Cade Medical", "Multiplied Health",
"Octi Care", "Tatnay", "Caymana"
```

**Generated ranges:**
- `deductible_amount`: $250–$2000
- `copay_amount`: $20–$150
- `prior_auth_required`: 50% true
- `active_plan`: 80% true
- `active_start_date`: hardcoded `2024-01-01`
- `tax_id_number`: 100,000,000–999,999,999

**Insert strategy:** one row at a time (7 companies × N plans; small enough to skip chunking)

---

### `clinic.rs`

**Public functions:**
```rust
pub async fn generate_clinics(ctx: &mut SimulationContext) -> Result<(), AppError>
pub async fn generate_clinic_schedules(ctx: &mut SimulationContext) -> Result<(), AppError>
```

**Reads from ctx:** `ctx.provider_ids`, `ctx.clinic_ids` (generate_clinic_schedules only)

**Writes to ctx:** `ctx.clinic_ids`, `ctx.counts.clinics`, `ctx.counts.clinic_schedules`

**Tables (Aurora DSQL):**
- `vital_fold.clinic` — INSERT 10 rows, `RETURNING clinic_id`
- `vital_fold.clinic_schedule` — INSERT one row per provider-clinic-day combination, `RETURNING schedule_id`

**Clinic — SQL columns:**
```
clinic: clinic_name, region, street_address, city, state, zip_code, phone_number, email
clinic_schedule: clinic_id, provider_id, day_of_week, start_time (09:00), end_time (17:00)
```

**Fixed data — 10 clinic locations:**
```
Charlotte NC (×1), Asheville NC (×1), Atlanta GA (×2), Tallahassee FL (×1),
Miami FL (×2), Orlando FL (×1), Jacksonville FL (×2)
```

**Clinic naming pattern:** `"VitalFold Heart Center - {City}"`

**Schedule generation:**
- Each provider assigned to 1–2 clinics at random
- Each clinic assignment: 3–5 weekdays at random from `["Monday","Tuesday","Wednesday","Thursday","Friday"]`
- Hours: 09:00–17:00 fixed

**Insert strategy:** one row at a time for clinics (10 rows); one row at a time for schedules

---

### `provider.rs`

**Public function:**
```rust
pub async fn generate_providers(ctx: &mut SimulationContext) -> Result<(), AppError>
```

**Reads from ctx:** `ctx.config.providers`

**Writes to ctx:** `ctx.provider_ids`, `ctx.counts.providers`

**Tables (Aurora DSQL):**
- `vital_fold.provider` — INSERT N rows, `RETURNING provider_id`

**Provider — SQL columns:**
```
provider: first_name, last_name, specialty, license_type, phone_number, email
```

**Fixed data:**
```rust
SPECIALTIES:    ["Cardiologist", "Cardiac Surgeon", "Electrophysiologist", "Interventional Cardiologist"]
LICENSE_TYPES:  ["MD", "DO"]
```

**Names:** generated via `fake` crate (`FirstName`, `LastName` from `fake::faker::name::en`)

**Insert strategy:** one row at a time (N providers; typically 50)

---

### `patient.rs`

**Public functions:**
```rust
pub async fn generate_patients(ctx: &mut SimulationContext) -> Result<(), AppError>
pub async fn generate_emergency_contacts(ctx: &mut SimulationContext) -> Result<(), AppError>  // no-op
pub async fn generate_patient_demographics(ctx: &mut SimulationContext) -> Result<(), AppError>
pub async fn generate_patient_insurance(ctx: &mut SimulationContext) -> Result<(), AppError>
```

**Reads from ctx:** `ctx.config.patients`, `ctx.patient_data` (demographics), `ctx.patient_ids` + `ctx.plan_ids` (insurance)

**Writes to ctx:** `ctx.patient_ids`, `ctx.patient_data`, `ctx.counts.patients`, `ctx.counts.emergency_contacts`, `ctx.counts.patient_demographics`, `ctx.counts.patient_insurance`

**Tables (Aurora DSQL):**
- `vital_fold.emergency_contact` — bulk INSERT + UPDATE via UNNEST
- `vital_fold.patient` — bulk INSERT via UNNEST, `RETURNING patient_id`
- `vital_fold.patient_demographics` — bulk INSERT via UNNEST
- `vital_fold.patient_insurance` — bulk INSERT via UNNEST

**Patient — SQL columns:**
```
emergency_contact: emergency_contact_id, patient_id, first_name, last_name,
                   relationship, phone_number, email
patient:           first_name, last_name, date_of_birth, street_address, city, state,
                   zip_code, phone_number, email, registration_date, emergency_contact_id (TEXT)
patient_demographics: patient_id, first_name, last_name, date_of_birth, age (BIGINT),
                      ssn (TEXT "XXX-XX-XXXX"), ethnicity, birth_gender
patient_insurance: patient_id, insurance_plan_id, policy_number (TEXT "POL-XXXXXXXX"),
                   coverage_start_date, coverage_end_date (nullable)
```

**Three-step emergency contact strategy (avoids per-row FK update):**
1. Pre-generate EC UUIDs client-side (`Uuid::new_v4()`)
2. Bulk INSERT ECs with temp `patient_id` (same as `emergency_contact_id` as placeholder)
3. Bulk INSERT patients → receive real `patient_id`s via `RETURNING`
4. Bulk UPDATE ECs: `SET patient_id = u.patient_id FROM UNNEST(...) WHERE ec.emergency_contact_id = u.ec_id`

**Generated ranges:**
- `date_of_birth`: 18–80 years old (`18*365` to `(18+62)*365` days back from today)
- `age`: derived as `(today - dob).num_days() / 365`
- `ssn`: format `"NNN-NN-NNNN"` (random digits)
- `coverage_end_date`: 20% have a non-null past date (30–365 days ago); 80% active (NULL)
- `relationship`: one of `["Spouse", "Parent", "Sibling", "Child", "Friend"]`
- `ethnicity`: one of `["Caucasian", "African American", "Hispanic", "Asian", "Other"]`
- `birth_gender`: one of `["Male", "Female", "Other"]`

**Insert strategy:** chunked UNNEST with `DSQL_BATCH_SIZE = 2500`. RNG built in synchronous `build_patient_batch()` helper, dropped before first `.await`.

---

### `appointment.rs`

**Public function (populate phase):**
```rust
pub async fn generate_appointments(ctx: &mut SimulationContext) -> Result<(), AppError>
```

**Internal function (simulate phase, called by `mod.rs::run_simulate`):**
```rust
pub(super) async fn write_patient_visit(
    dynamo: &aws_sdk_dynamodb::Client,
    visit: &crate::models::PatientVisit,
)
```

Reads all field values from the Aurora `PatientVisit` row (including vitals) — no random generation at write time.

**Reads from ctx:** `ctx.patient_ids`, `ctx.provider_ids`, `ctx.clinic_ids`, `ctx.config.appointments_per_patient`

**Writes to ctx:** `ctx.counts.appointments`

**Tables (Aurora DSQL):**
- `vital_fold.appointment` — bulk INSERT via UNNEST (no RETURNING needed; IDs fetched by run_simulate later)

**Appointment — SQL columns:**
```
appointment: patient_id, provider_id, clinic_id, appointment_date (TIMESTAMP), reason_for_visit
```

**Appointment date generation:**
- `days_ahead`: 0–89 days from today
- `hour`: 9–16 (9 AM–4:59 PM)
- `minute`: 0–59

**Fixed data — appointment reasons:**
```
"Annual checkup", "Chest pain evaluation", "Blood pressure check",
"Follow-up visit", "New patient visit"
```

**Insert strategy:** chunked UNNEST with `DSQL_BATCH_SIZE = 2500`. All data built synchronously before first `.await`.

#### DynamoDB Writes (simulate phase only)

**`patient_visit` table** (single table, vitals embedded):

| Attribute | Type | Notes |
|---|---|---|
| `patient_id` | S (UUID string) | Partition key |
| `clinic_id` | S | Sort key: `"clinic_id#visit_id"` |
| `provider_id` | S (UUID string) | |
| `checkin_time` | S | ISO 8601 |
| `checkout_time` | S | ISO 8601 = checkin + 30–120 min |
| `provider_seen_time` | S | ISO 8601 = checkin + 5–30 min |
| `ekg_usage` | Bool | 20% true |
| `estimated_copay` | N (string) | $20–$149 |
| `creation_time` | N (epoch) | Unix timestamp of write |
| `record_expiration_epoch` | N (epoch) | creation_time + 90 days (DynamoDB TTL) |
| `height` | N (string) | 60.0–77.9 inches |
| `weight` | N (string) | 120.0–219.9 lbs |
| `blood_pressure` | S | `"SYS/DIA"` format (100–159 / 60–99) |
| `heart_rate` | N (string) | 50–119 bpm |
| `temperature` | N (string) | 97.0–99.5 °F |
| `oxygen_saturation` | N (string) | 95.0–99.9 % SpO2 |
| `pulse_rate` | N (string) | 50–119 bpm |

**DynamoDB error handling:** fire-and-forget — errors logged with `tracing::error!`, simulation continues.

---

### `visit.rs`

**Public functions:**
```rust
pub async fn generate_patient_visits(ctx: &mut SimulationContext) -> Result<(), AppError>
pub async fn generate_visits_for_appointments(
    pool: &DbPool,
    appointments: &[(Uuid, Uuid, Uuid, Uuid, NaiveDateTime)],
) -> Result<usize, AppError>
```

**Reads from ctx:** `ctx.pool` (queries appointments from Aurora)

**Writes to ctx:** `ctx.counts.patient_visits`

**Tables (Aurora DSQL):**
- `vital_fold.patient_visits` — bulk INSERT via UNNEST (17 columns)

**patient_visits — SQL columns:**
```
patient_id, clinic_id, provider_id, checkin_time, checkout_time,
provider_seen_time, ekg_usage, estimated_copay, creation_time, record_expiration_epoch,
height, weight, blood_pressure, heart_rate, temperature, oxygen_saturation, pulse_rate
```

**Vitals generation (inline on each visit):**
- `height`: 60.0–77.9 inches
- `weight`: 120.0–219.9 lbs
- `blood_pressure`: "SYS/DIA" (100–159 / 60–99)
- `heart_rate`: 50–119 bpm
- `temperature`: 97.0–99.5 °F
- `oxygen_saturation`: 95.0–99.9 %SpO2
- `pulse_rate`: 50–119 bpm

**Insert strategy:** chunked UNNEST with `DSQL_BATCH_SIZE = 2500`. RNG in synchronous block, dropped before `.await`.

---

### `medical_record.rs`

**Public function:**
```rust
pub async fn generate_medical_records(ctx: &mut SimulationContext) -> Result<(), AppError>
```

**Reads from ctx:** `ctx.config.records_per_appointment`, `ctx.pool` (queries appointments directly)

**Writes to ctx:** `ctx.counts.medical_records`

**Tables (Aurora DSQL):**
- `vital_fold.medical_record` — bulk INSERT via UNNEST

**Medical record — SQL columns:**
```
medical_record: patient_id, provider_id, clinic_id, record_date (TIMESTAMP),
                diagnosis, treatment
```

**record_date generation:** appointment_date + 15–120 minutes offset

**Fixed data — 8 diagnosis codes:**
```
"Atrial Fibrillation (AFib)", "Coronary Artery Disease (CAD)", "Chest Pain",
"Hypertension", "Hyperlipidemia", "Shortness of Breath (SOB)",
"Tachycardia", "Bradycardia"
```

**Treatment mapping (deterministic — one treatment per diagnosis):**
```
"Atrial Fibrillation (AFib)"    → "Anticoagulation therapy"
"Coronary Artery Disease (CAD)" → "Statin therapy"
"Chest Pain"                    → "Stress test ordered"
"Hypertension"                  → "ACE inhibitor"
"Hyperlipidemia"                → "Statin initiated"
"Shortness of Breath (SOB)"     → "Pulmonary function test"
"Tachycardia"                   → "Beta blocker"
"Bradycardia"                   → "Pacemaker evaluation"
```

**Data source:** fetches all appointments from Aurora (full table scan) to get `patient_id`, `provider_id`, `clinic_id`, `appointment_date` for FK references.

**Insert strategy:** chunked UNNEST with `DSQL_BATCH_SIZE = 2500`. RNG in synchronous block, dropped before `.await`.

---

## Shared Patterns and Conventions

### Batch Insert Pattern

Every generator for large tables follows this pattern to comply with Aurora DSQL's 3000-row per-transaction limit:

```rust
const DSQL_BATCH_SIZE: usize = 2500;  // defined locally in each file

// Step 1: Build all column vecs synchronously (RNG must be dropped before .await)
let (col_a, col_b, ...) = {
    let mut rng = thread_rng();
    // ... fill vecs ...
}; // rng dropped here

// Step 2: Chunk and INSERT
for chunk_start in (0..total).step_by(DSQL_BATCH_SIZE) {
    let chunk_end = (chunk_start + DSQL_BATCH_SIZE).min(total);
    let r = chunk_start..chunk_end;

    sqlx::query(
        "INSERT INTO vital_fold.table_name (col_a, col_b) \
         SELECT * FROM UNNEST($1::type_a[], $2::type_b[])"
    )
    .bind(&col_a[r.clone()])
    .bind(&col_b[r.clone()])
    .execute(&ctx.pool)
    .await?;
}
```

### UUID Generation Strategy

UUIDs are generated by **PostgreSQL** using `gen_random_uuid()` — Rust omits the `*_id` columns in INSERT and captures them via `RETURNING`:

```rust
let result: (Uuid,) = sqlx::query_as(
    "INSERT INTO vital_fold.table_name (col_a) VALUES ($1) RETURNING table_id"
)
.bind(value)
.fetch_one(&ctx.pool)
.await?;
ctx.some_ids.push(result.0);
```

Exception: emergency contact IDs are pre-generated client-side (`Uuid::new_v4()`) to avoid a FK lookup round-trip.

### Phone Number Helper

Defined locally in each generator file (not shared via a module):

```rust
fn gen_phone(rng: &mut impl Rng) -> String {
    // Format: +1-NXX-NXX-XXXX (18 chars, fits VARCHAR(20))
    // First digit of each 3-digit group is 2–9; remaining digits are 0–9
}
```

### ThreadRng Must Be Dropped Before `.await`

`ThreadRng` is `!Send`. Always scope it in a synchronous block and drop it before the first `.await`:

```rust
let data = {
    let mut rng = thread_rng();
    // build data...
}; // rng dropped here
some_async_call().await?;
```

### Error Handling

All generators return `Result<(), AppError>`. Use `?` for propagation. DynamoDB writes are fire-and-forget:

```rust
if let Err(e) = dynamo.put_item()...send().await {
    tracing::error!("DynamoDB write failed: {:?}", e);
}
```

### Counting

Every generator increments `ctx.counts.*` after each successful insert. Counts are stored in `SimulatorState` at the end of `run_populate` / `run_simulate`.

---

## Common Imports for Generator Files

```rust
use crate::errors::AppError;
use super::SimulationContext;

// Types
use uuid::Uuid;
use chrono::{NaiveDate, NaiveDateTime, NaiveTime, TimeDelta, Utc, Local};
use sqlx::types::BigDecimal;

// Fake data
use fake::{Fake, Faker};
use fake::faker::name::en::{FirstName, LastName};
use fake::faker::internet::en::SafeEmail;
use fake::faker::address::en::{StreetName, CityName, StateAbbr, ZipCode};

// Random
use rand::{thread_rng, Rng};

// DynamoDB (appointment.rs only)
use aws_sdk_dynamodb::types::AttributeValue;

// Tracing
use tracing;  // info!, warn!, error!
```

---

## Aurora DSQL Constraints

| Constraint | Value | Why |
|---|---|---|
| Max rows per transaction statement | 3,000 | DSQL hard limit |
| Batch size used | 2,500 | Headroom below limit |
| `TRUNCATE` | Not supported | Use batched `DELETE ... WHERE pk IN (SELECT pk FROM t LIMIT 2500)` |
| `ctid` | Not supported | DSQL is distributed — no physical row pointers |
| IAM token TTL | ~15 min | Token refresh handled in `src/db/mod.rs` (not generators) |
| UUID generation | Server-side `gen_random_uuid()` | Generators use `RETURNING` to capture |
