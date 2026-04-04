use chrono::NaiveDateTime;
use serde::{Deserialize, Serialize};
use sqlx::types::BigDecimal;
use uuid::Uuid;

/// Visit metadata stored in Aurora DSQL `vital_fold.patient_visit`.
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct PatientVisit {
    pub patient_visit_id: Uuid,
    pub appointment_id: Uuid,
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
}

/// Combined visit + vitals row returned by JOIN queries.
/// Used by `run_simulate` and `run_date_range_simulate` to read from both
/// Aurora tables and write to both DynamoDB tables in one pass.
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
