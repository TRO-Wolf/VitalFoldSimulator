use chrono::NaiveDate;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Patient record in the clinic system.
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
    // Note: emergency_contact_id is VARCHAR(255) in DB, populated after emergency_contact insert
    pub emergency_contact_id: String,
}

/// Emergency contact for a patient.
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

/// Patient demographic information.
/// Duplicates some fields from Patient (first_name, last_name, date_of_birth)
/// to maintain demographic history.
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
