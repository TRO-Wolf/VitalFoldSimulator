use crate::config::Config;
use crate::db::DbPool;
use crate::errors::AppError;
use crate::middleware::auth::generate_token;
use crate::models::{AuthResponse, LoginRequest, User, UserProfile};
use actix_web::{web, HttpResponse};
use bcrypt::verify;
use chrono::Utc;
use serde::Deserialize;
use uuid::Uuid;

/// Login with email and password.
///
/// # Request Body
/// ```json
/// {
///   "email": "user@example.com",
///   "password": "secure_password"
/// }
/// ```
///
/// # Returns
/// * `200 OK` with JWT token and user profile on success
/// * `401 Unauthorized` if email not found or password is wrong
/// * `500 Internal Server Error` if database fails
#[utoipa::path(
    post,
    path = "/api/v1/auth/login",
    tag = "Authentication",
    request_body = LoginRequest,
    responses(
        (status = 200, description = "User logged in successfully", body = AuthResponse),
        (status = 400, description = "Invalid input (empty email or password)", body = String),
        (status = 401, description = "Invalid credentials", body = String),
        (status = 500, description = "Internal server error", body = String)
    )
)]
pub async fn login(
    pool: web::Data<DbPool>,
    cfg: web::Data<Config>,
    req: web::Json<LoginRequest>,
) -> Result<HttpResponse, AppError> {
    // Validate the login request
    req.validate()?;

    let email = req.email.trim().to_lowercase();

    // Fetch user by email
    let user = sqlx::query_as::<_, User>(
        "SELECT id, email, password_hash, created_at FROM public.users WHERE email = $1",
    )
    .bind(&email)
    .fetch_optional(pool.get_ref())
    .await?
    .ok_or_else(|| {
        // Same error for both unknown email and wrong password (prevent enumeration)
        tracing::warn!("Login attempt with unknown email: {}", email);
        AppError::Unauthorized("Invalid credentials".to_string())
    })?;

    // Verify password
    let password_valid = verify(&req.password, &user.password_hash)
        .map_err(|e| {
            tracing::error!("Password verification error: {}", e);
            AppError::Internal("Password verification failed".to_string())
        })?;

    if !password_valid {
        tracing::warn!("Failed login attempt for user: {}", user.id);
        return Err(AppError::Unauthorized("Invalid credentials".to_string()));
    }

    // Generate JWT token
    let token = generate_token(user.id, user.email.clone(), cfg.get_ref())?;

    let user_profile = UserProfile::from(user);
    let user_id = user_profile.id;

    let response = AuthResponse {
        token,
        user: user_profile,
    };

    tracing::info!("User logged in: {}", user_id);

    Ok(HttpResponse::Ok().json(response))
}

/// Request body for admin login.
#[derive(Debug, Deserialize, utoipa::ToSchema)]
pub struct AdminLoginRequest {
    pub username: String,
    pub password: String,
}

/// Login using admin credentials from environment variables.
///
/// # Request Body
/// ```json
/// {
///   "username": "admin",
///   "password": "secret"
/// }
/// ```
///
/// # Returns
/// * `200 OK` with JWT token and admin profile on success
/// * `401 Unauthorized` if credentials are wrong or admin login is not configured
#[utoipa::path(
    post,
    path = "/api/v1/auth/admin-login",
    tag = "Authentication",
    responses(
        (status = 200, description = "Admin logged in successfully", body = AuthResponse),
        (status = 401, description = "Invalid admin credentials", body = String)
    )
)]
pub async fn admin_login(
    cfg: web::Data<Config>,
    req: web::Json<AdminLoginRequest>,
) -> Result<HttpResponse, AppError> {
    let expected_username = cfg.admin_username.as_deref().ok_or_else(|| {
        tracing::warn!("Admin login attempted but ADMIN_USERNAME is not configured");
        AppError::Unauthorized("Invalid credentials".to_string())
    })?;

    let expected_password = cfg.admin_password.as_deref().ok_or_else(|| {
        tracing::warn!("Admin login attempted but ADMIN_PASSWORD is not configured");
        AppError::Unauthorized("Invalid credentials".to_string())
    })?;

    let username_matches = req.username == expected_username;
    let password_matches = req.password == expected_password;

    if !username_matches || !password_matches {
        tracing::warn!("Failed admin login attempt for username: {}", req.username);
        return Err(AppError::Unauthorized("Invalid credentials".to_string()));
    }

    // Use a fixed UUID for the admin identity so the sub claim is stable
    // across restarts without requiring a database row.
    let admin_id = Uuid::parse_str("00000000-0000-0000-0000-000000000001")
        .expect("hardcoded admin UUID is valid");
    let admin_email = format!("{}@admin.internal", expected_username);

    let token = generate_token(admin_id, admin_email.clone(), cfg.get_ref())?;

    let now = Utc::now();
    let user_profile = UserProfile {
        id: admin_id,
        email: admin_email,
        created_at: now,
    };

    let response = AuthResponse {
        token,
        user: user_profile,
    };

    tracing::info!("Admin logged in: {}", admin_id);

    Ok(HttpResponse::Ok().json(response))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_email_normalization() {
        let email = "  User@Example.COM  ";
        let normalized = email.trim().to_lowercase();
        assert_eq!(normalized, "user@example.com");
    }
}
