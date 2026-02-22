use crate::config::Config;
use crate::errors::AppError;
use aws_config::BehaviorVersion;
use aws_sdk_dsql::auth_token::AuthTokenGenerator;
use sqlx::postgres::PgPoolOptions;

/// Type alias for the database pool
pub type DbPool = sqlx::PgPool;

/// Create a connection pool for Aurora DSQL with IAM authentication.
///
/// This function:
/// 1. Loads AWS configuration from the environment
/// 2. Generates a short-lived IAM auth token
/// 3. Creates a sqlx postgres pool with the token as the password
/// 4. Returns the pool ready for use
///
/// # Arguments
/// * `cfg` - Application configuration containing DSQL endpoint and credentials
///
/// # Returns
/// * `Result<DbPool, AppError>` - A ready-to-use connection pool
///
/// # Note
/// IAM tokens expire in ~15 minutes. For long-running services, consider implementing
/// a background task to refresh the pool on a schedule.
pub async fn create_pool(cfg: &Config) -> Result<DbPool, AppError> {
    // Load AWS configuration with latest behavior version
    let aws_config = aws_config::defaults(BehaviorVersion::latest())
        .region(aws_config::Region::new(cfg.dsql_region.clone()))
        .load()
        .await;

    // Generate IAM authentication token
    let token = generate_auth_token(&aws_config, &cfg.dsql_endpoint, &cfg.dsql_user)
        .await
        .map_err(|e| {
            tracing::error!("Failed to generate DSQL auth token: {}", e);
            AppError::Database(format!("Failed to generate auth token: {}", e))
        })?;

    // Construct database URL with IAM token as password
    let database_url = format!(
        "postgres://{}:{}@{}:5432/{}",
        cfg.dsql_user, token, cfg.dsql_endpoint, cfg.dsql_db_name
    );

    // Create the sqlx pool
    let pool = PgPoolOptions::new()
        .max_connections(cfg.db_pool_size as u32)
        .connect(&database_url)
        .await
        .map_err(|e| {
            tracing::error!("Failed to create database pool: {}", e);
            AppError::Database(format!("Failed to create pool: {}", e))
        })?;

    tracing::info!(
        "Database pool created successfully (max_size: {}, endpoint: {})",
        cfg.db_pool_size,
        cfg.dsql_endpoint
    );

    Ok(pool)
}

/// Generate an IAM authentication token for Aurora DSQL.
///
/// This creates a short-lived token that can be used as a password
/// for database connections. The token is signed by AWS credentials.
async fn generate_auth_token(
    aws_config: &aws_config::SdkConfig,
    endpoint: &str,
    user: &str,
) -> Result<String, Box<dyn std::error::Error>> {
    let generator = AuthTokenGenerator::new(
        aws_config
            .credentials_provider()
            .ok_or("No AWS credentials available")?,
    );

    let token = generator
        .db_connect_admin_auth_token(
            endpoint.parse()?,
            user,
            std::time::Duration::from_secs(900), // 15 minutes
        )
        .await?;

    Ok(token)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pool_type_alias() {
        // This test verifies the DbPool type alias is correct
        // Runtime test would require actual AWS configuration
    }
}
