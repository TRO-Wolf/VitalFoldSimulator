use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
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

/// Request body for user registration.
#[derive(Debug, Deserialize)]
pub struct RegisterRequest {
    pub email: String,
    pub password: String,
}

impl RegisterRequest {
    /// Validate registration request.
    ///
    /// Checks:
    /// - Email is not empty and contains @ and at least one .
    /// - Password is at least 8 characters
    pub fn validate(&self) -> Result<(), AppError> {
        // Email validation: must have @ and . and at least 3 characters
        let email = self.email.trim();
        if email.is_empty() {
            return Err(AppError::BadRequest("Email is required".to_string()));
        }
        if !email.contains('@') || !email.contains('.') || email.len() < 5 {
            return Err(AppError::BadRequest("Invalid email format".to_string()));
        }

        // Password validation: minimum 8 characters
        if self.password.is_empty() {
            return Err(AppError::BadRequest("Password is required".to_string()));
        }
        if self.password.len() < 8 {
            return Err(AppError::BadRequest("Password must be at least 8 characters".to_string()));
        }

        Ok(())
    }
}

/// Request body for user login.
#[derive(Debug, Deserialize)]
pub struct LoginRequest {
    pub email: String,
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

/// Response body for successful authentication (register or login).
#[derive(Debug, Serialize)]
pub struct AuthResponse {
    pub token: String,
    pub user: UserProfile,
}

/// Safe user profile returned in API responses.
/// Never includes password or sensitive data.
#[derive(Debug, Serialize, Clone)]
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
#[derive(Debug, Serialize)]
pub struct MessageResponse {
    pub message: String,
}

/// Simulation status response.
#[derive(Debug, Serialize)]
pub struct SimulationStatusResponse {
    pub running: bool,
    pub last_run: Option<DateTime<Utc>>,
    #[serde(flatten)]
    pub counts: crate::engine_state::SimulationCounts,
}
