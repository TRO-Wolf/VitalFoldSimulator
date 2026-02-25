use crate::config::Config;
use crate::errors::AppError;
use actix_web::{dev::ServiceRequest, Error, HttpMessage};
use actix_web_httpauth::extractors::bearer::BearerAuth;
use chrono::Utc;
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};
use std::str;
use uuid::Uuid;

/// JWT claims embedded in the token.
/// Used to identify and authorize users.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Claims {
    /// User ID (subject)
    pub sub: String,
    /// User email
    pub email: String,
    /// Expiration time (unix timestamp)
    pub exp: i64,
    /// Issued at time (unix timestamp)
    pub iat: i64,
}



/// Generate a JWT token for a user.
///
/// # Arguments
/// * `user_id` - The UUID of the user
/// * `email` - The user's email address
/// * `cfg` - Application configuration (contains JWT_SECRET and JWT_EXPIRY_HOURS)
///
/// # Returns
/// * `Result<String, AppError>` - The encoded JWT token
pub fn generate_token(user_id: Uuid, email: String, cfg: &Config) -> Result<String, AppError> {
    let now = Utc::now();
    let expiration = now + chrono::Duration::hours(cfg.jwt_expiry_hours);

    let claims = Claims {
        sub: user_id.to_string(),
        email,
        iat: now.timestamp(),
        exp: expiration.timestamp(),
    };

    let encoding_key = EncodingKey::from_secret(cfg.jwt_secret.as_ref());

    encode(&Header::default(), &claims, &encoding_key)
        .map_err(|e| {
            tracing::error!("Failed to encode JWT: {}", e);
            AppError::Internal(format!("Failed to generate token: {}", e))
        })
}



/// Validate and decode a JWT token.
///
/// # Arguments
/// * `token` - The JWT token string
/// * `secret` - The JWT secret key
///
/// # Returns
/// * `Result<Claims, AppError>` - The decoded claims if valid
pub fn validate_token(token: &str, secret: &str) -> Result<Claims, AppError> {
    let decoding_key = DecodingKey::from_secret(secret.as_ref());

    decode::<Claims>(token, &decoding_key, &Validation::default())
        .map(|data| data.claims)
        .map_err(|e| {
            tracing::warn!("JWT validation failed: {}", e);
            AppError::Unauthorized("Invalid or expired token".to_string())
        })
}



/// Actix Web extractor for JWT bearer token validation.
///
/// This function extracts the bearer token from the Authorization header,
/// validates it, and inserts the Claims into the request extensions
/// for use by handlers.
///
/// # Arguments
/// * `req` - The service request
/// * `credentials` - The bearer token from the Authorization header
///
/// # Returns
/// * `Result<ServiceRequest, (Error, ServiceRequest)>` - The request with Claims inserted, or error with request
pub async fn jwt_validator(
    mut req: ServiceRequest,
    credentials: BearerAuth,
) -> Result<ServiceRequest, (Error, ServiceRequest)> {
    // Extract config first, before any early returns
    let cfg = match req.app_data::<actix_web::web::Data<Config>>() {
        Some(data) => data.get_ref(),
        None => {
            tracing::error!("Config not found in app_data");
            let err = actix_web::error::ErrorInternalServerError("Internal error");
            return Err((err, req));
        }
    };

    // Validate the token
    let token = credentials.token();
    match validate_token(token, &cfg.jwt_secret) {
        Ok(claims) => {
            req.extensions_mut().insert(claims);
            Ok(req)
        }
        Err(e) => {
            tracing::debug!("JWT validation error: {:?}", e);
            let err = actix_web::error::ErrorUnauthorized(e.to_string());
            Err((err, req))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_claims_structure() {
        let claims = Claims {
            sub: "user123".to_string(),
            email: "user@example.com".to_string(),
            exp: 1000000,
            iat: 900000,
        };

        assert_eq!(claims.sub, "user123");
        assert_eq!(claims.email, "user@example.com");
    }

    #[test]
    fn test_token_generation_and_validation() {
        let cfg = Config {
            host: "127.0.0.1".to_string(),
            port: 8787,
            dsql_endpoint: "test.cluster.dsql.amazonaws.com".to_string(),
            dsql_region: "us-east-1".to_string(),
            dsql_db_name: "postgres".to_string(),
            dsql_user: "admin".to_string(),
            db_pool_size: 10,
            jwt_secret: "test-secret-key-that-is-long-enough-for-validation".to_string(),
            jwt_expiry_hours: 24,
            admin_username: None,
            admin_password: None,
        };

        let user_id = Uuid::new_v4();
        let email = "test@example.com".to_string();

        let token = generate_token(user_id, email.clone(), &cfg).expect("Failed to generate token");
        assert!(!token.is_empty());

        let claims = validate_token(&token, &cfg.jwt_secret).expect("Failed to validate token");
        assert_eq!(claims.sub, user_id.to_string());
        assert_eq!(claims.email, email);
    }

    #[test]
    fn test_invalid_token_rejection() {
        let cfg = Config {
            host: "127.0.0.1".to_string(),
            port: 8787,
            dsql_endpoint: "test.cluster.dsql.amazonaws.com".to_string(),
            dsql_region: "us-east-1".to_string(),
            dsql_db_name: "postgres".to_string(),
            dsql_user: "admin".to_string(),
            db_pool_size: 10,
            jwt_secret: "test-secret-key-that-is-long-enough-for-validation".to_string(),
            jwt_expiry_hours: 24,
            admin_username: None,
            admin_password: None,
        };

        let invalid_token = "invalid.token.here";
        let result = validate_token(invalid_token, &cfg.jwt_secret);

        assert!(result.is_err());
        match result {
            Err(AppError::Unauthorized(_)) => (),
            _ => panic!("Expected Unauthorized error"),
        }
    }

    #[test]
    fn test_wrong_secret_rejection() {
        let cfg = Config {
            host: "127.0.0.1".to_string(),
            port: 8787,
            dsql_endpoint: "test.cluster.dsql.amazonaws.com".to_string(),
            dsql_region: "us-east-1".to_string(),
            dsql_db_name: "postgres".to_string(),
            dsql_user: "admin".to_string(),
            db_pool_size: 10,
            jwt_secret: "test-secret-key-that-is-long-enough-for-validation".to_string(),
            jwt_expiry_hours: 24,
            admin_username: None,
            admin_password: None,
        };

        let user_id = Uuid::new_v4();
        let email = "test@example.com".to_string();

        let token = generate_token(user_id, email, &cfg).expect("Failed to generate token");

        let wrong_secret = "wrong-secret-key-that-is-long-enough-for-validation";
        let result = validate_token(&token, wrong_secret);

        assert!(result.is_err());
    }
}
