use actix_web::{web, HttpResponse};
use serde::Serialize;

/// Simple health status response.
#[derive(Debug, Serialize)]
pub struct HealthResponse {
    pub status: String,
}

/// Health check endpoint.
/// Returns 200 OK with status "ok" to indicate the service is running.
pub async fn health_check() -> HttpResponse {
    tracing::info!("Health check requested");
    HttpResponse::Ok().json(HealthResponse {
        status: "ok".to_string(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_health_response() {
        let resp = HealthResponse {
            status: "ok".to_string(),
        };
        assert_eq!(resp.status, "ok");
    }
}
