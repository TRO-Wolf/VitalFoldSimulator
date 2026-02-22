use crate::config::Config;
use crate::db::DbPool;
use crate::errors::AppError;
use crate::middleware::auth::{generate_token, Claims};
use crate::models::{AuthResponse, LoginRequest, RegisterRequest, User, UserProfile};
use actix_web::{web, HttpResponse};
use bcrypt::{hash, verify, DEFAULT_COST};
use chrono::Utc;
use uuid::Uuid;

/// Register a new user.
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
/// * `201 Created` with JWT token and user profile on success
/// * `400 Bad Request` if email already exists
/// * `500 Internal Server Error` if hashing or database fails
pub async fn register(
    pool: web::Data<DbPool>,
    cfg: web::Data<Config>,
    req: web::Json<RegisterRequest>,
) -> Result<HttpResponse, AppError> {
    // Validate the registration request
    req.validate()?;

    let email = req.email.trim().to_lowercase();

    // Hash the password with bcrypt
    let password_hash = hash(&req.password, DEFAULT_COST)
        .map_err(|e| {
            tracing::error!("Failed to hash password: {}", e);
            AppError::Internal(format!("Password hashing failed: {}", e))
        })?;

    // Insert the new user
    // Let the database enforce email uniqueness via UNIQUE constraint
    let user_id = Uuid::new_v4();
    let now = Utc::now();

    sqlx::query!(
        "INSERT INTO public.users (id, email, password_hash, created_at) VALUES ($1, $2, $3, $4)",
        user_id,
        email,
        password_hash,
        now
    )
    .execute(pool.get_ref())
    .await
    .map_err(|e| {
        // Check if this is a unique constraint violation
        let error_msg = e.to_string();
        if error_msg.contains("duplicate key") || error_msg.contains("unique constraint") {
            tracing::info!("Registration attempt with duplicate email: {}", email);
            AppError::BadRequest("Email already registered".to_string())
        } else {
            tracing::error!("Database error during registration: {}", e);
            AppError::Database(error_msg)
        }
    })?;

    // Generate JWT token
    let token = generate_token(user_id, email.clone(), cfg.get_ref())?;

    let user_profile = UserProfile {
        id: user_id,
        email,
        created_at: now,
    };

    let response = AuthResponse {
        token,
        user: user_profile,
    };

    tracing::info!("User registered: {}", user_id);

    Ok(HttpResponse::Created().json(response))
}

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

    let response = AuthResponse {
        token,
        user: user_profile,
    };

    tracing::info!("User logged in: {}", user_profile.id);

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
