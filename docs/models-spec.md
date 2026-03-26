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
#[derive(Debug, Clone, Serialize, Deserialize, FromRow, ToSchema)]
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
#[derive(Debug, Clone, Serialize, Deserialize, FromRow, ToSchema)]
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

/// Maps to vital_fold.clinic_schedule
#[derive(Debug, Clone, Serialize, Deserialize, FromRow, ToSchema)]
pub struct ClinicSchedule {
    pub schedule_id: Uuid,
    pub clinic_id: Uuid,
    pub provider_id: Uuid,
    pub day_of_week: String,             // "Monday", "Tuesday", etc.
    pub start_time: NaiveTime,
    pub end_time: NaiveTime,
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
#[derive(Debug, Clone, Serialize, Deserialize, FromRow, ToSchema)]
pub struct Appointment {
    pub appointment_id: Uuid,
    pub patient_id: Uuid,
    pub provider_id: Uuid,
    pub clinic_id: Uuid,
    pub appointment_date: NaiveDateTime,
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
#[derive(Debug, Clone, Serialize, Deserialize, FromRow, ToSchema)]
pub struct MedicalRecord {
    pub medical_record_id: Uuid,
    pub patient_id: Uuid,
    pub provider_id: Uuid,
    pub clinic_id: Uuid,
    pub record_date: NaiveDateTime,
    pub diagnosis: String,               // Drawn from cardiac diagnosis codes
    pub treatment: String,               // Drawn from cardiac treatment list
}
```

---

## `src/models/patient_visit.rs`

Maps to: `vital_fold.patient_visits` (Aurora DSQL) and `patient_visit` (DynamoDB)

Vitals are embedded as columns directly on the visit row (wide-column pattern).

```rust
use chrono::NaiveDateTime;
use serde::{Deserialize, Serialize};
use sqlx::types::BigDecimal;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct PatientVisit {
    pub patient_visit_id: Uuid,
    pub patient_id: Uuid,
    pub clinic_id: Uuid,
    pub provider_id: Uuid,
    pub checkin_time: NaiveDateTime,
    pub checkout_time: Option<NaiveDateTime>,
    pub provider_seen_time: Option<NaiveDateTime>,
    pub ekg_usage: bool,
    pub estimated_copay: BigDecimal,        // DECIMAL(10,2)
    pub creation_time: NaiveDateTime,
    pub record_expiration_epoch: i64,
    // Vitals (embedded — no separate patient_vitals table)
    pub height: BigDecimal,                 // DECIMAL(5,2) — inches
    pub weight: BigDecimal,                 // DECIMAL(5,2) — pounds
    pub blood_pressure: String,             // VARCHAR(20) — "SYS/DIA"
    pub heart_rate: i32,                    // INT — bpm
    pub temperature: BigDecimal,            // DECIMAL(4,1) — °F
    pub oxygen_saturation: BigDecimal,      // DECIMAL(4,1) — %SpO2
    pub pulse_rate: i32,                    // INT — bpm
}
```

## `src/engine_state.rs` — SimulationCounts

Row counts from the last completed populate or simulate run.

```rust
#[derive(Debug, Clone, Default, Serialize, Deserialize, ToSchema)]
pub struct SimulationCounts {
    // Aurora DSQL fields (set by POST /populate)
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
    pub patient_visits: usize,
    // DynamoDB fields (set by POST /simulate)
    pub dynamo_patient_visits: usize,
}
```

---

## SQL Type → Rust Type Quick Reference

| SQL Type | Rust Type | Notes |
|---|---|---|
| `UUID` | `Uuid` | `uuid::Uuid` |
| `VARCHAR(N)` / `TEXT` | `String` | |
| `INT` | `i32` | |
| `BOOLEAN` | `bool` | |
| `DATE` | `NaiveDate` | `chrono::NaiveDate` |
| `TIMESTAMP` | `NaiveDateTime` | `chrono::NaiveDateTime` |
| `TIMESTAMPTZ` | `DateTime<Utc>` | `chrono::DateTime<Utc>` |
| `TIME` | `NaiveTime` | `chrono::NaiveTime` |
| `DECIMAL(10,2)` | `BigDecimal` | `bigdecimal::BigDecimal` |
| `VARCHAR(N) NULL` | `Option<String>` | Nullable columns → `Option<T>` |
