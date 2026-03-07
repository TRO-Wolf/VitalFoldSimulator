use chrono::NaiveDateTime;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Patient appointment at a clinic with a provider.
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Appointment {
    pub appointment_id: Uuid,
    pub patient_id: Uuid,
    pub provider_id: Uuid,
    pub clinic_id: Uuid,
    pub appointment_date: NaiveDateTime,
    pub reason_for_visit: String,
}


struct Transaction {
    id: u32,
    reference_date: NaiveDateTime,
    note: String
}
