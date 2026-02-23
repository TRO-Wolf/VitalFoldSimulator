use actix_web::HttpResponse;
use serde::Serialize;
use utoipa::ToSchema;

/// Simple health status response.
#[derive(Debug, Serialize, ToSchema)]
pub struct HealthResponse {
    pub status: String,
}

/// Health check endpoint.
/// Returns 200 OK with status "ok" to indicate the service is running.
#[utoipa::path(
    get,
    path = "/health",
    tag = "Health",
    responses(
        (status = 200, description = "Service is healthy", body = HealthResponse)
    )
)]
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
