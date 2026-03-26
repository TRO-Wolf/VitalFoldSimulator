# `src/models/` — Claude Context

> **Purpose:** Self-contained reference for all domain models in `src/models/`. Covers every struct, field, trait impl, and validation rule across all 8 model files.

---

## Overview

The models module defines all domain data types used for:
- **Database mapping** — structs with `sqlx::FromRow` for querying Aurora DSQL
- **API request/response** — structs with `Serialize`/`Deserialize` for JSON
- **OpenAPI documentation** — structs with `utoipa::ToSchema`

**Schema split:**
- `public.users` — auth table (user accounts)
- `vital_fold.*` — all simulation tables (patients, clinics, providers, etc.)

---

## `mod.rs`

```rust
pub mod user;
pub mod insurance;
pub mod patient;
pub mod provider;
pub mod clinic;
pub mod appointment;
pub mod medical_record;
pub mod patient_visit;

pub use user::*;
pub use insurance::*;
pub use patient::*;
pub use provider::*;
pub use clinic::*;
pub use appointment::*;
pub use medical_record::*;
pub use patient_visit::*;
```

All types are re-exported — import any model as `use crate::models::SomeStruct`.

---

## Common Traits for All Simulation Models

Every struct in the simulation domain (all files except `user.rs`) derives:

```rust
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
```

- `sqlx::FromRow` — enables `sqlx::query_as::<_, MyStruct>(...)` and `fetch_all`/`fetch_one`
- All field names use `snake_case` matching the SQL column names exactly
- All primary keys are `Uuid`
- All foreign keys are `Uuid` **except** `patient.emergency_contact_id` which is `String`

---

## `user.rs` — Auth Models

**Imports:** `chrono::{DateTime, Utc}`, `serde`, `uuid::Uuid`, `utoipa::ToSchema`, `crate::errors::AppError`

---

### `User` — Database Row

```rust
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct User {
    pub id: Uuid,
    pub email: String,
    #[serde(skip_serializing)]   // NEVER returned in API responses
    pub password_hash: String,
    pub created_at: DateTime<Utc>,
}
```

**Table:** `public.users`

**Critical:** `password_hash` has `#[serde(skip_serializing)]` — it will never appear in JSON output even if `User` is accidentally serialized directly.

---

### `LoginRequest` — POST body for `/api/v1/auth/login`

```rust
#[derive(Debug, Deserialize, ToSchema)]
pub struct LoginRequest {
    pub email: String,
    pub password: String,
}
```

**Validation (`validate() -> Result<(), AppError>`):**
- Email: not empty (trimmed) → `AppError::BadRequest`
- Password: not empty → `AppError::BadRequest`

Login validation checks only that fields are non-empty — it does not format-check the email.

---

### `AuthResponse` — Response for login

```rust
#[derive(Debug, Serialize, ToSchema)]
pub struct AuthResponse {
    pub token: String,       // JWT bearer token
    pub user: UserProfile,   // safe profile (no password)
}
```

Returned as `200 OK` by login and admin-login.

---

### `UserProfile` — Safe user data for API responses

```rust
#[derive(Debug, Serialize, Clone, ToSchema)]
pub struct UserProfile {
    pub id: Uuid,
    pub email: String,
    pub created_at: DateTime<Utc>,
}
```

Never includes `password_hash`. Returned by `/api/v1/me` and inside `AuthResponse`.

**Conversion:**
```rust
impl From<User> for UserProfile {
    fn from(user: User) -> Self {
        UserProfile { id: user.id, email: user.email, created_at: user.created_at }
    }
}
```

---

### `MessageResponse` — Generic status message

```rust
#[derive(Debug, Serialize, ToSchema)]
pub struct MessageResponse {
    pub message: String,
}
```

Used by stop, reset, and other endpoints that return only a text confirmation.

---

### `SimulationStatusResponse` — Response for `GET /simulate/status`

```rust
#[derive(Debug, Serialize, ToSchema)]
pub struct SimulationStatusResponse {
    pub running: bool,
    pub last_run: Option<DateTime<Utc>>,
    #[serde(flatten)]
    pub counts: crate::engine_state::SimulationCounts,
}
```

`#[serde(flatten)]` causes all `SimulationCounts` fields to appear at the top level of the JSON (not nested under `"counts"`). See `engine_state.rs` for the full field list.

---

## `insurance.rs` — Insurance Domain

**Imports:** `chrono::NaiveDate`, `serde`, `sqlx::types::BigDecimal`, `uuid::Uuid`

---

### `InsuranceCompany`

```rust
pub struct InsuranceCompany {
    pub company_id:     Uuid,
    pub company_name:   String,  // One of 7 fixed names (see generators doc)
    pub email:          String,
    pub phone_number:   String,  // Format: +1-NXX-NXX-XXXX
    pub tax_id_number:  i32,     // 9-digit EIN: 100_000_000..999_999_999
}
```

**Table:** `vital_fold.insurance_company` | **PK:** `company_id`

---

### `InsurancePlan`

```rust
pub struct InsurancePlan {
    pub insurance_plan_id:   Uuid,
    pub plan_name:           String,      // e.g., "Plan 1", "Plan 2"
    pub company_id:          Uuid,        // FK → InsuranceCompany
    pub deductible_amount:   BigDecimal,  // $250–$2000
    pub copay_amount:        BigDecimal,  // $20–$150
    pub prior_auth_required: bool,        // 50% true
    pub active_plan:         bool,        // 80% true
    pub active_start_date:   NaiveDate,   // hardcoded 2024-01-01
}
```

**Table:** `vital_fold.insurance_plan` | **PK:** `insurance_plan_id`

**Important:** `BigDecimal` comes from `sqlx::types::BigDecimal`, not the standalone `bigdecimal` crate.

---

### `PatientInsurance`

```rust
pub struct PatientInsurance {
    pub patient_insurance_id: Uuid,
    pub patient_id:           Uuid,             // FK → Patient
    pub insurance_plan_id:    Uuid,             // FK → InsurancePlan
    pub policy_number:        String,           // Format: "POL-XXXXXXXX" (8 hex chars)
    pub coverage_start_date:  NaiveDate,        // today at insert time
    pub coverage_end_date:    Option<NaiveDate>,// NULL = active; 20% have a past end date
}
```

**Table:** `vital_fold.patient_insurance` | **PK:** `patient_insurance_id`

---

## `patient.rs` — Patient Domain

**Imports:** `chrono::NaiveDate`, `serde`, `uuid::Uuid`

---

### `Patient`

```rust
pub struct Patient {
    pub patient_id:           Uuid,
    pub first_name:           String,
    pub last_name:            String,
    pub middle_name:          Option<String>,  // always NULL in generated data
    pub date_of_birth:        NaiveDate,       // age range: 18–80 years
    pub street_address:       String,
    pub city:                 String,
    pub state:                String,
    pub zip_code:             String,
    pub phone_number:         String,          // Format: +1-NXX-NXX-XXXX
    pub email:                String,
    pub registration_date:    NaiveDate,       // today at insert time
    pub emergency_contact_id: String,          // VARCHAR(255) — UUID as string, NOT a UUID FK
}
```

**Table:** `vital_fold.patient` | **PK:** `patient_id`

**Critical:** `emergency_contact_id` is `String`, not `Uuid`. The database column is `VARCHAR(255)`. This is intentional — the value is the UUID of the related `EmergencyContact` row stored as a plain string.

---

### `EmergencyContact`

```rust
pub struct EmergencyContact {
    pub emergency_contact_id: Uuid,
    pub patient_id:           Uuid,    // FK → Patient (updated after patient INSERT)
    pub first_name:           String,
    pub last_name:            String,
    pub relationship:         String,  // "Spouse", "Parent", "Sibling", "Child", "Friend"
    pub phone_number:         String,
    pub email:                String,
}
```

**Table:** `vital_fold.emergency_contact` | **PK:** `emergency_contact_id`

**Insert order:** Emergency contacts are inserted before patients (with a placeholder `patient_id`), then bulk-updated after patients are inserted to set the real `patient_id`.

---

### `PatientDemographics`

```rust
pub struct PatientDemographics {
    pub demographics_id: Uuid,
    pub patient_id:      Uuid,      // FK → Patient
    pub first_name:      String,    // duplicated from Patient
    pub last_name:       String,    // duplicated from Patient
    pub date_of_birth:   NaiveDate, // duplicated from Patient
    pub age:             i32,       // computed: (today - dob).num_days() / 365
    pub ssn:             String,    // format: "NNN-NN-NNNN"
    pub ethnicity:       String,    // "Caucasian","African American","Hispanic","Asian","Other"
    pub birth_gender:    String,    // "Male","Female","Other"
}
```

**Table:** `vital_fold.patient_demographics` | **PK:** `demographics_id`

**Note:** `age` is `i32` in the model but inserted as `BIGINT` in SQL. Duplicated fields (`first_name`, `last_name`, `date_of_birth`) preserve demographic history in case patient data changes.

---

## `provider.rs` — Provider Domain

**Imports:** `serde`, `uuid::Uuid`

### `Provider`

```rust
pub struct Provider {
    pub provider_id:  Uuid,
    pub first_name:   String,
    pub last_name:    String,
    pub specialty:    String,  // "Cardiologist","Cardiac Surgeon","Electrophysiologist","Interventional Cardiologist"
    pub license_type: String,  // "MD" or "DO"
    pub phone_number: String,
    pub email:        String,
}
```

**Table:** `vital_fold.provider` | **PK:** `provider_id`

---

## `clinic.rs` — Clinic Domain

**Imports:** `chrono::NaiveTime`, `serde`, `uuid::Uuid`

### `Clinic`

```rust
pub struct Clinic {
    pub clinic_id:      Uuid,
    pub clinic_name:    String,  // e.g., "VitalFold Heart Center - Miami"
    pub region:         String,  // same as state abbreviation in generated data
    pub street_address: String,
    pub city:           String,
    pub state:          String,
    pub zip_code:       String,
    pub phone_number:   String,
    pub email:          String,
}
```

**Table:** `vital_fold.clinic` | **PK:** `clinic_id`

**Fixed distribution:** 10 clinics across SE US (Charlotte NC ×1, Asheville NC ×1, Atlanta GA ×2, Tallahassee FL ×1, Miami FL ×2, Orlando FL ×1, Jacksonville FL ×2).

---

### `ClinicSchedule`

```rust
pub struct ClinicSchedule {
    pub schedule_id: Uuid,
    pub clinic_id:   Uuid,      // FK → Clinic
    pub provider_id: Uuid,      // FK → Provider
    pub day_of_week: String,    // "Monday","Tuesday","Wednesday","Thursday","Friday"
    pub start_time:  NaiveTime, // 09:00:00 fixed
    pub end_time:    NaiveTime, // 17:00:00 fixed
}
```

**Table:** `vital_fold.clinic_schedule` | **PK:** `schedule_id`

One row per provider-clinic-day combination. Each provider works at 1–2 clinics, 3–5 days/week.

---

## `appointment.rs` — Appointment Domain

**Imports:** `chrono::NaiveDateTime`, `serde`, `uuid::Uuid`

### `Appointment`

```rust
pub struct Appointment {
    pub appointment_id:   Uuid,
    pub patient_id:       Uuid,          // FK → Patient
    pub provider_id:      Uuid,          // FK → Provider
    pub clinic_id:        Uuid,          // FK → Clinic
    pub appointment_date: NaiveDateTime, // no timezone; 0–89 days in future at insert time
    pub reason_for_visit: String,        // "Annual checkup","Chest pain evaluation",etc.
}
```

**Table:** `vital_fold.appointment` | **PK:** `appointment_id`

**Lifecycle:**
- Populated during `POST /populate` (0–89 days in future)
- Queried during `POST /simulate` (`WHERE appointment_date::date = CURRENT_DATE`)
- Each appointment produces 1 `patient_visit` Aurora row (with embedded vitals) and 1 DynamoDB record

---

## `medical_record.rs` — Medical Record Domain

**Imports:** `chrono::NaiveDateTime`, `serde`, `uuid::Uuid`

### `MedicalRecord`

```rust
pub struct MedicalRecord {
    pub medical_record_id: Uuid,
    pub patient_id:        Uuid,          // FK → Patient
    pub provider_id:       Uuid,          // FK → Provider
    pub clinic_id:         Uuid,          // FK → Clinic
    pub record_date:       NaiveDateTime, // appointment_date + 15–120 min offset
    pub diagnosis:         String,        // one of 8 cardiac diagnosis codes
    pub treatment:         String,        // matched to diagnosis (deterministic)
}
```

**Table:** `vital_fold.medical_record` | **PK:** `medical_record_id`

**Fixed diagnosis → treatment mapping:**

| Diagnosis | Treatment |
|---|---|
| `"Atrial Fibrillation (AFib)"` | `"Anticoagulation therapy"` |
| `"Coronary Artery Disease (CAD)"` | `"Statin therapy"` |
| `"Chest Pain"` | `"Stress test ordered"` |
| `"Hypertension"` | `"ACE inhibitor"` |
| `"Hyperlipidemia"` | `"Statin initiated"` |
| `"Shortness of Breath (SOB)"` | `"Pulmonary function test"` |
| `"Tachycardia"` | `"Beta blocker"` |
| `"Bradycardia"` | `"Pacemaker evaluation"` |

---

## `patient_visit.rs` — Patient Visit Domain (with Embedded Vitals)

**Imports:** `chrono::NaiveDateTime`, `serde`, `sqlx::types::BigDecimal`, `uuid::Uuid`

### `PatientVisit`

```rust
pub struct PatientVisit {
    pub patient_visit_id:        Uuid,
    pub patient_id:              Uuid,          // FK → Patient
    pub clinic_id:               Uuid,          // FK → Clinic
    pub provider_id:             Uuid,          // FK → Provider
    pub checkin_time:            NaiveDateTime,
    pub checkout_time:           Option<NaiveDateTime>,
    pub provider_seen_time:      Option<NaiveDateTime>,
    pub ekg_usage:               bool,          // 20% true
    pub estimated_copay:         BigDecimal,    // $20–$150
    pub creation_time:           NaiveDateTime,
    pub record_expiration_epoch: i64,           // Unix epoch + 7 years
    // Embedded vitals (wide-column pivot from former patient_vitals table)
    pub height:                  BigDecimal,    // inches
    pub weight:                  BigDecimal,    // pounds
    pub blood_pressure:          String,        // "SYS/DIA" format
    pub heart_rate:              i32,           // 50–120 bpm
    pub temperature:             BigDecimal,    // °F, 97.0–99.5
    pub oxygen_saturation:       BigDecimal,    // SpO2 92–100%
    pub pulse_rate:              i32,           // 50–120 bpm
}
```

**Table:** `vital_fold.patient_visits` | **PK:** `patient_visit_id`

**17 columns total** — vitals are stored directly on the visit row (no separate `patient_vitals` table). This 1:1 relationship eliminates the 7x EAV row multiplier.

**Generated during:** `POST /populate` (Phase 1, Aurora) → `POST /simulate` reads these rows and writes to DynamoDB `patient_visit` table with embedded vital attributes.

---

## Quick Reference: Field Types by Category

| Type | Used for |
|---|---|
| `Uuid` | All PKs and FKs |
| `String` | `patient.emergency_contact_id` (VARCHAR FK), text fields |
| `NaiveDate` | Date-only fields (DOB, coverage dates, registration) |
| `NaiveDateTime` | Timestamp fields without timezone (appointments, records) |
| `NaiveTime` | Time-of-day (clinic schedule start/end) |
| `DateTime<Utc>` | UTC timestamps in auth models (`User.created_at`) |
| `BigDecimal` | Financial amounts and vitals (`InsurancePlan.deductible_amount`, `copay_amount`; `PatientVisit.height`, `weight`, `temperature`, `oxygen_saturation`, `estimated_copay`) |
| `i32` | `PatientDemographics.age`, `InsuranceCompany.tax_id_number` |
| `bool` | Flags (`prior_auth_required`, `active_plan`) |
| `Option<T>` | Nullable fields (`coverage_end_date`, `middle_name`) |

---

## Cross-Module Relationships

**Imported by:**
- `handlers/auth.rs` — `LoginRequest`, `AuthResponse`, `User`, `UserProfile`
- `handlers/user.rs` — `User`, `UserProfile`
- `handlers/simulation.rs` — `MessageResponse`, `SimulationStatusResponse`
- `generators/*.rs` — no direct model imports; generators build data via raw SQL columns

**Models import from:**
- `crate::errors::AppError` (user.rs validate methods)
- `crate::engine_state::SimulationCounts` (SimulationStatusResponse)

---

## Common Imports for This Module

```rust
// All model files:
use serde::{Deserialize, Serialize};
use uuid::Uuid;

// Date/time (as needed per file):
use chrono::{DateTime, Utc};      // user.rs
use chrono::NaiveDate;            // insurance.rs, patient.rs
use chrono::NaiveDateTime;        // appointment.rs, medical_record.rs
use chrono::NaiveTime;            // clinic.rs

// Financial:
use sqlx::types::BigDecimal;      // insurance.rs only

// OpenAPI:
use utoipa::ToSchema;             // user.rs structs only (models exposed in Swagger)

// Error handling (user.rs validate methods only):
use crate::errors::AppError;
```
