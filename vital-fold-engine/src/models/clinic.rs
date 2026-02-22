use chrono::NaiveTime;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Clinic location (e.g., Miami, FL; Atlanta, GA)
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

/// Provider schedule for a specific clinic.
/// One row per provider-clinic-day combination.
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct ClinicSchedule {
    pub schedule_id: Uuid,
    pub clinic_id: Uuid,
    pub provider_id: Uuid,
    /// Day of week (e.g., "Monday", "Tuesday")
    pub day_of_week: String,
    pub start_time: NaiveTime,
    pub end_time: NaiveTime,
}
