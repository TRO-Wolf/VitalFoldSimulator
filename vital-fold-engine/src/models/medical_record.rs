use chrono::NaiveDateTime;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Medical record for a patient visit.
/// Links diagnosis and treatment to an appointment.
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct MedicalRecord {
    pub medical_record_id: Uuid,
    pub patient_id: Uuid,
    pub provider_id: i64,
    pub clinic_id: i64,
    pub record_date: NaiveDateTime,
    /// Diagnosis code (e.g., "Atrial Fibrillation (AFib)")
    pub diagnosis: String,
    /// Treatment plan (e.g., "Anticoagulation therapy")
    pub treatment: String,
}
