use anyhow::{anyhow, Result};

/// Application configuration loaded from environment variables.
/// All required fields must be present; missing values cause startup failure.
#[derive(Debug, Clone)]
pub struct Config {
    /// Server bind address (default: 0.0.0.0)
    pub host: String,

    /// Server bind port (default: 8787)
    pub port: u16,

    /// Aurora DSQL cluster endpoint (required)
    pub dsql_endpoint: String,

    /// AWS region for DSQL IAM token signing (default: us-east-1)
    pub dsql_region: String,

    /// Database name (default: postgres)
    pub dsql_db_name: String,

    /// Database user (default: admin)
    pub dsql_user: String,

    /// Database connection pool size (default: 10)
    pub db_pool_size: usize,

    /// JWT signing secret (required, minimum 32 chars recommended)
    pub jwt_secret: String,

    /// JWT token expiry in hours (default: 24)
    pub jwt_expiry_hours: i64,

    /// Admin username from ADMIN_USERNAME env var (optional)
    pub admin_username: Option<String>,

    /// Admin password from ADMIN_PASSWORD env var (optional)
    pub admin_password: Option<String>,
}

impl Config {
    /// Load configuration from environment variables.
    /// Panics if required variables are missing.
    pub fn from_env() -> Result<Self> {
        dotenvy::dotenv().ok();

        let host = std::env::var("HOST").unwrap_or_else(|_| "0.0.0.0".to_string());

        let port = std::env::var("PORT")
            .unwrap_or_else(|_| "8787".to_string())
            .parse::<u16>()
            .map_err(|e| anyhow!("PORT must be a valid u16: {}", e))?;

        let dsql_endpoint = std::env::var("DSQL_CLUSTER_ENDPOINT")
            .map_err(|_| anyhow!("DSQL_CLUSTER_ENDPOINT is required"))?;

        let dsql_region =
            std::env::var("DSQL_REGION").unwrap_or_else(|_| "us-east-1".to_string());

        let dsql_db_name =
            std::env::var("DSQL_DB_NAME").unwrap_or_else(|_| "postgres".to_string());

        let dsql_user = std::env::var("DSQL_USER").unwrap_or_else(|_| "admin".to_string());

        let db_pool_size = std::env::var("DB_POOL_SIZE")
            .unwrap_or_else(|_| "10".to_string())
            .parse::<usize>()
            .map_err(|e| anyhow!("DB_POOL_SIZE must be a valid usize: {}", e))?;

        let jwt_secret = std::env::var("JWT_SECRET")
            .map_err(|_| anyhow!("JWT_SECRET is required"))?;

        if jwt_secret.len() < 32 {
            return Err(anyhow!(
                "JWT_SECRET must be at least 32 characters long (got {})",
                jwt_secret.len()
            ));
        }

        let jwt_expiry_hours = std::env::var("JWT_EXPIRY_HOURS")
            .unwrap_or_else(|_| "24".to_string())
            .parse::<i64>()
            .map_err(|e| anyhow!("JWT_EXPIRY_HOURS must be a valid i64: {}", e))?;

        let admin_username = std::env::var("ADMIN_USERNAME").ok();
        let admin_password = std::env::var("ADMIN_PASSWORD").ok();

        Ok(Config {
            host,
            port,
            dsql_endpoint,
            dsql_region,
            dsql_db_name,
            dsql_user,
            db_pool_size,
            jwt_secret,
            jwt_expiry_hours,
            admin_username,
            admin_password,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_jwt_secret_validation() {
        // This test would require mocking environment variables
        // Kept as a placeholder for future test implementation
    }
}
