use chrono::NaiveDateTime;
use serde::{Deserialize, Serialize};
use sqlx::types::BigDecimal;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct PatientVital {
    pub patient_visit_id: Uuid,
    pub patient_id: Uuid,
    pub clinic_id: i64,
    pub provider_id: i64,
    pub height: BigDecimal,
    pub weight: BigDecimal,
    pub blood_pressure: String,
    pub heart_rate: i32,
    pub temperature: BigDecimal,
    pub oxygen_saturation: BigDecimal,
    pub creation_time: NaiveDateTime,
    pub record_expiration_epoch: i64,
}
