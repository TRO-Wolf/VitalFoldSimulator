use crate::config::Config;
use crate::errors::AppError;
use aws_config::BehaviorVersion;
use aws_sdk_dsql::auth_token::{AuthTokenGenerator, Config as DsqlTokenConfig};
use sqlx::postgres::{PgConnectOptions, PgPoolOptions, PgSslMode};
use std::time::Duration;

/// Type alias for the database pool
pub type DbPool = sqlx::PgPool;

/// How often to refresh the IAM auth token. DSQL tokens expire in ~15 min;
/// refresh every 12 minutes to stay well clear of that boundary.
const TOKEN_REFRESH_INTERVAL: Duration = Duration::from_secs(12 * 60);

/// Create a connection pool for Aurora DSQL with IAM authentication.
///
/// Uses PgConnectOptions (not a URL string) because IAM tokens contain characters
/// like '=' and '+' that would be misinterpreted if embedded in a URL.
pub async fn create_pool(cfg: &Config) -> Result<DbPool, AppError> {
    let aws_config = aws_config::defaults(BehaviorVersion::latest())
        .region(aws_config::Region::new(cfg.dsql_region.clone()))
        .load()
        .await;

    let token = generate_auth_token(&aws_config, &cfg.dsql_endpoint)
        .await
        .map_err(|e| {
            tracing::error!("Failed to generate DSQL auth token: {}", e);
            AppError::Database(format!("Failed to generate auth token: {}", e))
        })?;

    let connect_opts = build_connect_opts(cfg, &token);

    let pool = PgPoolOptions::new()
        .max_connections(cfg.db_pool_size as u32)
        .connect_with(connect_opts)
        .await
        .map_err(|e| {
            tracing::error!("Failed to create database pool: {}", e);
            AppError::Database(format!("Failed to create pool: {}", e))
        })?;

    tracing::info!(
        "Database pool created (max_size: {}, endpoint: {})",
        cfg.db_pool_size,
        cfg.dsql_endpoint
    );

    Ok(pool)
}

/// Spawn a background task that refreshes the DSQL IAM token every
/// TOKEN_REFRESH_INTERVAL seconds by calling `pool.set_connect_options`.
///
/// This updates the options used for **new** connections while leaving any
/// connections already checked out untouched. The pool itself (and all
/// `web::Data<DbPool>` clones) remain valid — no restart required.
pub fn spawn_token_refresh_task(pool: DbPool, cfg: Config) {
    tokio::spawn(async move {
        loop {
            tokio::time::sleep(TOKEN_REFRESH_INTERVAL).await;

            let aws_config = aws_config::defaults(BehaviorVersion::latest())
                .region(aws_config::Region::new(cfg.dsql_region.clone()))
                .load()
                .await;

            match generate_auth_token(&aws_config, &cfg.dsql_endpoint).await {
                Ok(token) => {
                    let new_opts = build_connect_opts(&cfg, &token);
                    pool.set_connect_options(new_opts);
                    tracing::info!("DSQL IAM token refreshed successfully");
                }
                Err(e) => {
                    // Log and continue — the previous token may still be valid
                    // for the remaining window; the next iteration will retry.
                    tracing::error!("Failed to refresh DSQL IAM token: {}", e);
                }
            }
        }
    });
}

/// Build PgConnectOptions from config and a freshly-generated token.
fn build_connect_opts(cfg: &Config, token: &str) -> PgConnectOptions {
    PgConnectOptions::new()
        .host(&cfg.dsql_endpoint)
        .port(5432)
        .database(&cfg.dsql_db_name)
        .username(&cfg.dsql_user)
        .password(token)
        .ssl_mode(PgSslMode::Require)
}

/// Generate a SigV4-signed IAM auth token for Aurora DSQL.
async fn generate_auth_token(
    aws_config: &aws_config::SdkConfig,
    endpoint: &str,
) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
    let token_config = DsqlTokenConfig::builder()
        .hostname(endpoint)
        .build()
        .map_err(|e| format!("Failed to build DSQL token config: {}", e))?;

    let generator = AuthTokenGenerator::new(token_config);

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
        // Runtime test would require actual AWS configuration
    }
}
