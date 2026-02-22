use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Healthcare provider (physician, nurse, etc.)
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Provider {
    pub provider_id: Uuid,
    pub first_name: String,
    pub last_name: String,
    /// Medical specialty (e.g., "Cardiologist", "Cardiac Surgeon")
    pub specialty: String,
    /// License type (e.g., "MD", "DO")
    pub license_type: String,
    pub phone_number: String,
    pub email: String,
}
