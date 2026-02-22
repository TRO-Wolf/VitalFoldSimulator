/// Application route configuration.
///
/// Defines all API routes: public health/auth endpoints and protected simulation endpoints.
/// Protected routes use JWT bearer token authentication via the jwt_validator middleware.

use crate::handlers::{auth, health, simulation, user};
use crate::middleware::auth::jwt_validator;
use actix_web::web;
use actix_web_httpauth::middleware::HttpAuthentication;

/// Configure all application routes.
///
/// # Route Structure
/// - **Public routes** (no authentication required):
///   - `GET /health` — Health check
///   - `POST /api/v1/auth/register` — User registration
///   - `POST /api/v1/auth/login` — User login
///
/// - **Protected routes** (require valid JWT bearer token):
///   - `GET /api/v1/me` — Get current user profile
///   - `POST /simulate` — Start simulation
///   - `POST /simulate/stop` — Stop simulation
///   - `GET /simulate/status` — Get simulation status
///   - `POST /simulate/reset` — Reset all data
pub fn configure(cfg: &mut web::ServiceConfig) {
    // Public routes - no authentication required
    cfg.service(
        web::scope("")
            .route("/health", web::get().to(health::health_check))
            .service(
                web::scope("/api/v1/auth")
                    .route("/register", web::post().to(auth::register))
                    .route("/login", web::post().to(auth::login)),
            ),
    );

    // Protected routes - require valid JWT bearer token
    let auth_middleware = HttpAuthentication::bearer(jwt_validator);

    cfg.service(
        web::scope("")
            .wrap(auth_middleware)
            .route("/api/v1/me", web::get().to(user::me))
            .service(
                web::scope("/simulate")
                    .route("", web::post().to(simulation::start_simulation))
                    .route("/stop", web::post().to(simulation::stop_simulation))
                    .route("/status", web::get().to(simulation::get_status))
                    .route("/reset", web::post().to(simulation::reset_data)),
            ),
    );
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_route_configuration() {
        // This is a compile-time check that routes are correctly structured
        // Full integration tests would require a running server
    }
}
