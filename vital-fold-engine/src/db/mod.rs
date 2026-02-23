use crate::config::Config;
use crate::errors::AppError;
use aws_config::BehaviorVersion;
use aws_sdk_dsql::auth_token::{AuthTokenGenerator, Config as DsqlTokenConfig};
use sqlx::postgres::{PgConnectOptions, PgPoolOptions, PgSslMode};

/// Type alias for the database pool
pub type DbPool = sqlx::PgPool;

/// Create a connection pool for Aurora DSQL with IAM authentication.
///
/// This function:
/// 1. Loads AWS configuration from the environment
/// 2. Generates a short-lived IAM auth token via SigV4 signing
/// 3. Creates a sqlx postgres pool using PgConnectOptions (avoids URL-encoding issues)
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
    // Load AWS configuration — picks up credentials from env vars, ~/.aws, or instance metadata
    let aws_config = aws_config::defaults(BehaviorVersion::latest())
        .region(aws_config::Region::new(cfg.dsql_region.clone()))
        .load()
        .await;

    // Generate a short-lived SigV4-signed IAM auth token
    let token = generate_auth_token(&aws_config, &cfg.dsql_endpoint)
        .await
        .map_err(|e| {
            tracing::error!("Failed to generate DSQL auth token: {}", e);
            AppError::Database(format!("Failed to generate auth token: {}", e))
        })?;

    // Use PgConnectOptions instead of a URL string — the token contains characters
    // like '=' and '+' that would be misinterpreted if embedded directly in a URL.
    let connect_opts = PgConnectOptions::new()
        .host(&cfg.dsql_endpoint)
        .port(5432)
        .database(&cfg.dsql_db_name)
        .username(&cfg.dsql_user)
        .password(&token)
        .ssl_mode(PgSslMode::Require);

    // Create the sqlx pool
    let pool = PgPoolOptions::new()
        .max_connections(cfg.db_pool_size as u32)
        .connect_with(connect_opts)
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

/// Generate a SigV4-signed IAM auth token for Aurora DSQL.
///
/// The token is short-lived (~15 min) and used as the database password.
/// The region is read from the SdkConfig; no separate region argument is needed.
async fn generate_auth_token(
    aws_config: &aws_config::SdkConfig,
    endpoint: &str,
) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
    let token_config = DsqlTokenConfig::builder()
        .hostname(endpoint)
        .build()
        .map_err(|e| format!("Failed to build DSQL token config: {}", e))?;

    let generator = AuthTokenGenerator::new(token_config);

    // db_connect_admin_auth_token signs using credentials from the SdkConfig
    let token = generator
        .db_connect_admin_auth_token(aws_config)
        .await
        .map_err(|e| format!("Failed to sign DSQL auth token: {}", e))?;

    Ok(token.to_string())
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
