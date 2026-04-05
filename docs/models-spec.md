# VitalFold Engine — Model Specifications

> Complete Rust struct definitions for every database table. Haiku must implement these exactly.
> All models live under `src/models/`. Each file is re-exported from `src/models/mod.rs`.

---

## Common Imports (used across all model files)

```rust
use chrono::{NaiveDate, NaiveDateTime, NaiveTime, DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;
use utoipa::ToSchema;
```

For `DECIMAL` columns, use `bigdecimal::BigDecimal` (from the `bigdecimal` crate, re-exported by sqlx).

---

## `src/models/mod.rs`

```rust
pub mod user;
pub mod insurance;
pub mod patient;
pub mod provider;
pub mod clinic;
pub mod appointment;
pub mod medical_record;
pub mod patient_visit;
```

---

## `src/models/user.rs`

```rust
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use utoipa::ToSchema;

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct User {
    pub id: Uuid,
    pub email: String,
    #[serde(skip_serializing)]
    pub password_hash: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct LoginRequest {
    pub email: String,
    pub password: String,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct AuthResponse {
    pub token: String,
    pub user: UserProfile,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct UserProfile {
    pub id: Uuid,
    pub email: String,
    pub created_at: DateTime<Utc>,
}

impl From<User> for UserProfile {
    fn from(u: User) -> Self {
        Self {
            id: u.id,
            email: u.email,
            created_at: u.created_at,
        }
    }
}
```

---

## `src/models/insurance.rs`

Maps to: `vital_fold.insurance_company`, `vital_fold.insurance_plan`, `vital_fold.patient_insurance`

```rust
use chrono::NaiveDate;
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;
use utoipa::ToSchema;

/// Maps to vital_fold.insurance_company
#[derive(Debug, Clone, Serialize, Deserialize, FromRow, ToSchema)]
pub struct InsuranceCompany {
    pub company_id: Uuid,
    pub company_name: String,
    pub email: String,
    pub phone_number: String,
    pub tax_id_number: i32,           // INT in SQL — 9-digit integer
}

/// Maps to vital_fold.insurance_plan
#[derive(Debug, Clone, Serialize, Deserialize, FromRow, ToSchema)]
pub struct InsurancePlan {
    pub insurance_plan_id: Uuid,
    pub plan_name: String,
    pub company_id: Uuid,
    pub deductible_amount: bigdecimal::BigDecimal,  // DECIMAL(10,2)
    pub copay_amount: bigdecimal::BigDecimal,        // DECIMAL(10,2)
    pub prior_auth_required: bool,
    pub active_plan: bool,
    pub active_start_date: NaiveDate,
}

/// Maps to vital_fold.patient_insurance
#[derive(Debug, Clone, Serialize, Deserialize, FromRow, ToSchema)]
pub struct PatientInsurance {
    pub patient_insurance_id: Uuid,
    pub patient_id: Uuid,
    pub insurance_plan_id: Uuid,
    pub policy_number: String,
    pub coverage_start_date: NaiveDate,
    pub coverage_end_date: Option<NaiveDate>,  // Nullable in SQL
}
```

---

## `src/models/patient.rs`

Maps to: `vital_fold.patient`, `vital_fold.emergency_contact`, `vital_fold.patient_demographics`

```rust
use chrono::NaiveDate;
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;
use utoipa::ToSchema;

/// Maps to vital_fold.patient
#[derive(Debug, Clone, Serialize, Deserialize, FromRow, ToSchema)]
pub struct Patient {
    pub patient_id: Uuid,
    pub first_name: String,
    pub last_name: String,
    pub middle_name: Option<String>,     // Nullable VARCHAR(255)
    pub date_of_birth: NaiveDate,
    pub street_address: String,
    pub city: String,
    pub state: String,
    pub zip_code: String,
    pub phone_number: String,
    pub email: String,
    pub registration_date: NaiveDate,
    pub emergency_contact_id: String,    // VARCHAR(255), stores UUID as string
}

/// Maps to vital_fold.emergency_contact
#[derive(Debug, Clone, Serialize, Deserialize, FromRow, ToSchema)]
pub struct EmergencyContact {
    pub emergency_contact_id: Uuid,
    pub patient_id: Uuid,
    pub first_name: String,
    pub last_name: String,
    pub relationship: String,
    pub phone_number: String,
    pub email: String,
}

/// Maps to vital_fold.patient_demographics
#[derive(Debug, Clone, Serialize, Deserialize, FromRow, ToSchema)]
pub struct PatientDemographics {
    pub demographics_id: Uuid,
    pub patient_id: Uuid,
    pub first_name: String,              // Duplicated from patient — same source values
    pub last_name: String,               // Duplicated from patient — same source values
    pub date_of_birth: NaiveDate,        // Duplicated from patient — same source values
    pub age: i32,                        // Derived: calculated from date_of_birth at insert time
    pub ssn: String,                     // VARCHAR(11), format: "XXX-XX-XXXX"
    pub ethnicity: String,
    pub birth_gender: String,            // VARCHAR(50)
}
```

---

## `src/models/provider.rs`

Maps to: `vital_fold.provider`

```rust
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;
use utoipa::ToSchema;

/// Maps to vital_fold.provider
/// provider_id is a BIGINT identity column (CACHE 1), not a UUID.
/// license_type is one of: "MD", "DO", "NP" (~30% are Nurse Practitioners)
/// email format: {first_initial}.{last_name}@example.org (e.g., "j.smith@example.org")
#[derive(Debug, Clone, Serialize, Deserialize, FromRow, ToSchema)]
pub struct Provider {
    pub provider_id: i64,
    pub first_name: String,
    pub last_name: String,
    pub specialty: String,
    pub license_type: String,      // "MD" | "DO" | "NP"
    pub phone_number: String,
    pub email: String,
}
```

---

## `src/models/clinic.rs`

Maps to: `vital_fold.clinic`, `vital_fold.clinic_schedule`

```rust
use chrono::NaiveTime;
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;
use utoipa::ToSchema;

/// Maps to vital_fold.clinic
/// clinic_id is a BIGINT identity column (CACHE 1), not a UUID.
/// street_address format: "1234 Elm Blvd, Suite 200" (realistic)
/// email format: vfhc_{city}{n}@vitalfold.org (e.g., "vfhc_miami1@vitalfold.org")
/// zip_code uses metro-area prefix + 2 random digits (e.g., Miami = "331xx")
#[derive(Debug, Clone, Serialize, Deserialize, FromRow, ToSchema)]
pub struct Clinic {
    pub clinic_id: i64,
    pub clinic_name: String,
    pub region: String,
    pub street_address: String,
    pub city: String,
    pub state: String,
    pub zip_code: String,
    pub phone_number: String,
    pub email: String,
}

/// Maps to vital_fold.clinic_schedule
#[derive(Debug, Clone, Serialize, Deserialize, FromRow, ToSchema)]
pub struct ClinicSchedule {
    pub schedule_id: Uuid,
    pub clinic_id: i64,
    pub provider_id: i64,
    pub day_of_week: String,             // "Monday", "Tuesday", etc.
    pub start_time: NaiveTime,           // 08:00
    pub end_time: NaiveTime,             // 17:00
}
```

---

## `src/models/appointment.rs`

Maps to: `vital_fold.appointment`

```rust
use chrono::NaiveDateTime;
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;
use utoipa::ToSchema;

/// Maps to vital_fold.appointment
/// provider_id and clinic_id are BIGINT (not UUID) — identity columns from their respective tables.
/// appointment_datetime uses 15-minute windows only: :00, :15, :30, :45
/// Time range: 8:00 AM to 4:45 PM (9 hours × 4 slots = 36 slots per provider per day)
/// Appointment volume = provider_count × 36 slots/day, distributed by clinic_weights.
#[derive(Debug, Clone, Serialize, Deserialize, FromRow, ToSchema)]
pub struct Appointment {
    pub appointment_id: Uuid,
    pub patient_id: Uuid,
    pub provider_id: i64,
    pub clinic_id: i64,
    pub appointment_datetime: NaiveDateTime,
    pub reason_for_visit: String,        // Drawn from diagnosis codes list
}
```

---

## `src/models/medical_record.rs`

Maps to: `vital_fold.medical_record`

```rust
use chrono::NaiveDateTime;
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;
use utoipa::ToSchema;

/// Maps to vital_fold.medical_record
/// provider_id and clinic_id are BIGINT (not UUID).
#[derive(Debug, Clone, Serialize, Deserialize, FromRow, ToSchema)]
pub struct MedicalRecord {
    pub medical_record_id: Uuid,
    pub patient_id: Uuid,
    pub provider_id: i64,
    pub clinic_id: i64,
    pub record_date: NaiveDateTime,
    pub diagnosis: String,               // Drawn from cardiac diagnosis codes
    pub treatment: String,               // Drawn from cardiac treatment list
}
```

---

## `src/models/patient_visit.rs`

Maps to: `vital_fold.patient_visit` (visit metadata) and `vital_fold.patient_vitals` (1:1 vitals)

Vitals are stored in a **separate** `patient_vitals` table linked by `patient_visit_id` (not embedded).

**Timing rules:**
- `checkin_time` = `appointment_datetime` minus 5-15 minutes (early arrival)
- `provider_seen_time` = `appointment_datetime` plus 0-5 minutes
- `checkout_time` = `appointment_datetime` plus 15-30 minutes

**EKG-based copay:**
- EKG visit (~20%): `estimated_copay` is $150-$350
- Standard visit: `estimated_copay` is $20-$150

```rust
use chrono::NaiveDateTime;
use serde::{Deserialize, Serialize};
use sqlx::types::BigDecimal;
use uuid::Uuid;

/// Maps to vital_fold.patient_visit — visit metadata only.
/// appointment_id is a FK back to vital_fold.appointment for explicit linkage.
/// clinic_id and provider_id are BIGINT (not UUID).
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct PatientVisit {
    pub patient_visit_id: Uuid,
    pub appointment_id: Uuid,               // FK to vital_fold.appointment
    pub patient_id: Uuid,
    pub clinic_id: i64,
    pub provider_id: i64,
    pub checkin_time: NaiveDateTime,
    pub checkout_time: Option<NaiveDateTime>,
    pub provider_seen_time: Option<NaiveDateTime>,
    pub ekg_usage: bool,
    pub estimated_copay: BigDecimal,        // DECIMAL(10,2)
    pub creation_time: NaiveDateTime,
    pub record_expiration_epoch: i64,
}

/// Maps to vital_fold.patient_vitals — 1:1 with patient_visit via patient_visit_id PK.
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct PatientVital {
    pub patient_visit_id: Uuid,             // PK + FK to patient_visit
    pub patient_id: Uuid,
    pub clinic_id: i64,
    pub provider_id: i64,
    pub height: BigDecimal,                 // DECIMAL(5,2) — inches
    pub weight: BigDecimal,                 // DECIMAL(5,2) — pounds
    pub blood_pressure: String,             // VARCHAR(20) — "SYS/DIA"
    pub heart_rate: i32,                    // INT — bpm
    pub temperature: BigDecimal,            // DECIMAL(4,1) — °F
    pub oxygen_saturation: BigDecimal,      // DECIMAL(4,1) — %SpO2
    pub creation_time: NaiveDateTime,
    pub record_expiration_epoch: i64,
}

/// Combined JOIN result used by run_simulate / run_date_range_simulate
/// to read both Aurora tables in one pass before writing to DynamoDB.
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct PatientVisitWithVitals {
    pub patient_visit_id: Uuid,
    pub patient_id: Uuid,
    pub clinic_id: i64,
    pub provider_id: i64,
    pub checkin_time: NaiveDateTime,
    pub checkout_time: Option<NaiveDateTime>,
    pub provider_seen_time: Option<NaiveDateTime>,
    pub ekg_usage: bool,
    pub estimated_copay: BigDecimal,
    pub creation_time: NaiveDateTime,
    pub record_expiration_epoch: i64,
    pub height: BigDecimal,
    pub weight: BigDecimal,
    pub blood_pressure: String,
    pub heart_rate: i32,
    pub temperature: BigDecimal,
    pub oxygen_saturation: BigDecimal,
}

/// Maps to vital_fold.survey — optional 1:1 with patient_visit.
/// Only ~30% of visits produce a survey (realistic response rate).
/// Intent: gold-layer AVG(gene_prissy_score) GROUP BY provider_id.
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Survey {
    pub survey_id: Uuid,
    pub patient_visit_id: Uuid,
    pub gene_prissy_score: i32,          // 1–10
    pub experience_score: i32,           // 1–10
    pub feedback_comments: Option<String>,
    pub creation_time: NaiveDateTime,
}

/// Maps to vital_fold.cpt_code — reference table seeded by POST /admin/init-db.
/// 12 common E/M + EKG codes with CY2024 RVU values.
/// Documentation-only — the RVU generator uses a private CptLookup struct.
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct CptCode {
    pub cpt_code_id: i64,
    pub code: String,                    // '99213', '93000', etc.
    pub short_description: String,
    pub category: String,                // 'E/M' or 'Diagnostic'
    pub work_rvu: BigDecimal,
    pub pe_rvu_nonfacility: BigDecimal,
    pub pe_rvu_facility: BigDecimal,
    pub mp_rvu: BigDecimal,
    pub global_days: Option<i16>,        // 0, 10, 90, or NULL (XXX)
    pub effective_year: i16,
    pub is_active: bool,
}

/// Maps to vital_fold.appointment_cpt — billing line-item fact table.
/// One row per CPT billed on an appointment (typically 1 E/M + optional EKG).
/// RVU values are snapshotted at service time so gold-layer rollups stay
/// stable across annual CMS PPRRVU updates.
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct AppointmentCpt {
    pub appointment_cpt_id: Uuid,
    pub appointment_id: Uuid,            // FK to vital_fold.appointment
    pub cpt_code_id: i64,                // FK to vital_fold.cpt_code
    pub provider_id: i64,
    pub clinic_id: i64,
    pub service_date: NaiveDate,
    pub units: i16,                      // usually 1
    pub modifier_1: Option<String>,      // CPT modifier codes (unused in synthetic data)
    pub modifier_2: Option<String>,
    pub work_rvu_snapshot: BigDecimal,
    pub pe_rvu_snapshot: BigDecimal,
    pub mp_rvu_snapshot: BigDecimal,
    pub total_rvu_snapshot: BigDecimal,  // work + pe_nonfacility + mp
    pub conversion_factor: BigDecimal,   // $32.7442 for CY2024
    pub expected_amount: Option<BigDecimal>,  // total_rvu × CF, rounded to 2dp
    pub creation_time: NaiveDateTime,
}
```

## `src/engine_state.rs` — SimulationCounts

Row counts from the last completed populate or simulate run.

```rust
#[derive(Debug, Clone, Default, Serialize, Deserialize, ToSchema)]
pub struct SimulationCounts {
    // Aurora DSQL static reference data
    pub insurance_companies: usize,
    pub insurance_plans: usize,
    pub clinics: usize,
    pub providers: usize,
    pub patients: usize,
    pub emergency_contacts: usize,
    pub patient_demographics: usize,
    pub patient_insurance: usize,
    // Aurora DSQL dynamic (date-dependent) data
    pub clinic_schedules: usize,
    pub appointments: usize,
    pub medical_records: usize,
    pub patient_visits: usize,
    pub patient_vitals: usize,
    pub surveys: usize,
    // RVU / billing
    pub cpt_codes: usize,         // reference table (usually 12)
    pub appointment_cpt: usize,   // line-items, ~1.2× appointments
    // DynamoDB table counts (set by POST /simulate)
    pub dynamo_patient_visits: usize,
    pub dynamo_patient_vitals: usize,
}
```

---

## SQL Type → Rust Type Quick Reference

| SQL Type | Rust Type | Notes |
|---|---|---|
| `UUID` | `Uuid` | `uuid::Uuid` — used for patient_id, appointment_id, etc. |
| `BIGINT` | `i64` | Used for `provider_id`, `clinic_id` — identity columns |
| `BIGINT GENERATED BY DEFAULT AS IDENTITY (CACHE 1)` | `i64` | Auto-incrementing PK with cache=1 (tight ordering, small tables) |
| `VARCHAR(N)` / `TEXT` | `String` | |
| `INT` | `i32` | |
| `BOOLEAN` | `bool` | |
| `DATE` | `NaiveDate` | `chrono::NaiveDate` |
| `TIMESTAMP` | `NaiveDateTime` | `chrono::NaiveDateTime` |
| `TIMESTAMPTZ` | `DateTime<Utc>` | `chrono::DateTime<Utc>` |
| `TIME` | `NaiveTime` | `chrono::NaiveTime` |
| `DECIMAL(10,2)` | `BigDecimal` | `bigdecimal::BigDecimal` |
| `VARCHAR(N) NULL` | `Option<String>` | Nullable columns → `Option<T>` |

## Patient Insurance Notes

- `coverage_start_date` is a random date within the past 365 days from the simulation run date
- `coverage_end_date` is `NULL` for ~80% of rows (active policies), populated for ~20% (expired)

## Provider Generation Notes

- **License type distribution:** ~30% Nurse Practitioners (NP), ~70% MD/DO (split evenly)
- **Clinic assignment:** Providers are assigned to specific clinics proportionally via `clinic_weights` — busier clinics (Miami, Atlanta) get more providers than smaller ones (Asheville, Tallahassee)
- **Email format:** `{first_initial}.{lastname}@example.org` (e.g., "j.smith@example.org")
