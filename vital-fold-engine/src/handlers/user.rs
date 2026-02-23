use crate::db::DbPool;
use crate::errors::AppError;
use crate::middleware::auth::Claims;
use crate::models::{User, UserProfile};
use actix_web::{web, HttpMessage, HttpRequest, HttpResponse};
use uuid::Uuid;

/// Get the current authenticated user's profile.
///
/// This endpoint requires a valid JWT bearer token in the Authorization header.
///
/// # Returns
/// * `200 OK` with user profile if user exists
/// * `401 Unauthorized` if no valid token provided
/// * `404 Not Found` if user ID from token doesn't exist in database
/// * `500 Internal Server Error` if database fails
#[utoipa::path(
    get,
    path = "/api/v1/me",
    tag = "User",
    security(
        ("bearer_auth" = [])
    ),
    responses(
        (status = 200, description = "User profile retrieved", body = UserProfile),
        (status = 401, description = "Unauthorized - invalid or missing token", body = String),
        (status = 404, description = "User not found", body = String),
        (status = 500, description = "Internal server error", body = String)
    )
)]
pub async fn me(
    req: HttpRequest,
    pool: web::Data<DbPool>,
) -> Result<HttpResponse, AppError> {
    // Extract Claims from request extensions (inserted by jwt_validator middleware)
    let claims = req
        .extensions()
        .get::<Claims>()
        .ok_or_else(|| {
            tracing::error!("Claims not found in request extensions");
            AppError::Unauthorized("Authentication required".to_string())
        })?
        .clone();

    // Parse the user ID from the JWT subject claim
    let user_id = Uuid::parse_str(&claims.sub).map_err(|e| {
        tracing::error!("Failed to parse user_id from JWT: {}", e);
        AppError::Unauthorized("Invalid token".to_string())
    })?;

    // Fetch the user from the database
    let user = sqlx::query_as::<_, User>(
        "SELECT id, email, password_hash, created_at FROM public.users WHERE id = $1",
    )
    .bind(user_id)
    .fetch_optional(pool.get_ref())
    .await?
    .ok_or_else(|| {
        tracing::warn!("User not found: {}", user_id);
        AppError::NotFound("User not found".to_string())
    })?;

    let user_profile = UserProfile::from(user);

    tracing::debug!("Retrieved user profile: {}", user_profile.id);

    Ok(HttpResponse::Ok().json(user_profile))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_uuid_parsing() {
        let uuid_str = "550e8400-e29b-41d4-a716-446655440000";
        let parsed = Uuid::parse_str(uuid_str);
        assert!(parsed.is_ok());
    }
}
