use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use utoipa::ToSchema;
use crate::errors::AppError;

/// User account in the system.
/// password_hash is never serialized in API responses.
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct User {
    pub id: Uuid,
    pub email: String,
    #[serde(skip_serializing)]
    pub password_hash: String,
    pub created_at: DateTime<Utc>,
}

/// Request body for user login.
#[derive(Debug, Deserialize, ToSchema)]
pub struct LoginRequest {
    #[schema(example = "user@example.com")]
    pub email: String,
    #[schema(example = "SecurePassword123")]
    pub password: String,
}

impl LoginRequest {
    /// Validate login request.
    ///
    /// Checks:
    /// - Email is not empty
    /// - Password is not empty
    pub fn validate(&self) -> Result<(), AppError> {
        if self.email.trim().is_empty() {
            return Err(AppError::BadRequest("Email is required".to_string()));
        }

        if self.password.is_empty() {
            return Err(AppError::BadRequest("Password is required".to_string()));
        }

        Ok(())
    }
}

/// Response body for successful authentication.
#[derive(Debug, Serialize, ToSchema)]
pub struct AuthResponse {
    pub token: String,
    pub user: UserProfile,
}

/// Safe user profile returned in API responses.
/// Never includes password or sensitive data.
#[derive(Debug, Serialize, Clone, ToSchema)]
pub struct UserProfile {
    pub id: Uuid,
    pub email: String,
    pub created_at: DateTime<Utc>,
}

impl From<User> for UserProfile {
    fn from(user: User) -> Self {
        UserProfile {
            id: user.id,
            email: user.email,
            created_at: user.created_at,
        }
    }
}

/// Generic message response for simple operations.
#[derive(Debug, Serialize, ToSchema)]
pub struct MessageResponse {
    pub message: String,
}

/// Simulation status response.
#[derive(Debug, Serialize, ToSchema)]
pub struct SimulationStatusResponse {
    pub running: bool,
    pub last_run: Option<DateTime<Utc>>,
    #[serde(flatten)]
    pub counts: crate::engine_state::SimulationCounts,
    /// Present only while an Aurora reset is in progress.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reset_progress: Option<crate::engine_state::ResetProgress>,
    /// Present only while a populate run is in progress.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub populate_progress: Option<crate::engine_state::PopulateProgress>,
    /// Present only while a DynamoDB operation (reset or sync) is in progress.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dynamo_progress: Option<crate::engine_state::DynamoProgress>,
}
