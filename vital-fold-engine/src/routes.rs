/// Application route configuration.
///
/// # Route Structure
///
/// **Public routes** (no authentication required):
/// - `GET  /health`                 — Health check
/// - `POST /api/v1/auth/login`       — User login
/// - `POST /api/v1/auth/admin-login` — Admin login (env-var credentials, no DB required)
///
/// **Protected routes** (require valid JWT bearer token):
/// - `GET  /api/v1/me`              — Get current user profile
///
/// **Population routes** (JWT required):
/// - `POST /populate`               — Seed all Aurora DSQL tables (legacy, runs all 13 steps)
/// - `POST /populate/static`        — Seed static reference data only (Step 1: 8 steps)
/// - `POST /populate/dynamic`       — Seed date-dependent data for a date range (Step 2: 7 steps)
/// - `GET  /populate/dates`         — List dates that already have appointments
/// - `POST /populate/reset-dynamic` — Delete only dynamic data, preserve static reference data
///
/// **Simulation routes** (JWT required):
/// - `POST /simulate`               — Write DynamoDB records for today's appointments (Phase 2)
/// - `POST /simulate/stop`          — Stop running job
/// - `GET  /simulate/status`        — Poll run status and counts
/// - `GET  /simulate/db-counts`    — Live record counts from Aurora + DynamoDB
/// - `POST /simulate/reset`         — Delete all Aurora DSQL data
/// - `POST /simulate/reset-dynamo`  — Delete all DynamoDB data
/// - `POST /simulate/timelapse`     — Start single-day heatmap visualization (auto-populates DynamoDB)
/// - `GET  /simulate/heatmap`       — Poll per-clinic activity for heatmap
/// - `GET  /simulate/visitors`      — Get today's visitors (patient names) grouped by clinic
/// - `POST /simulate/date-range`    — Sync Aurora visit data to DynamoDB for a date range
/// - `POST /simulate/replay`        — Start read-only heatmap replay (no DynamoDB writes)
/// - `POST /simulate/replay-reset`  — Clear heatmap replay state (no data deleted)

use crate::handlers::{auth, health, simulation, user};
use crate::middleware::auth::jwt_validator;
use actix_web::web;
use actix_web_httpauth::middleware::HttpAuthentication;

/// Configure all application routes.
pub fn configure(cfg: &mut web::ServiceConfig) {
    // Public routes — no authentication required
    cfg.route("/health", web::get().to(health::health_check));

    cfg.service(
        web::scope("/api/v1/auth")
            .route("/login",       web::post().to(auth::login))
            .route("/admin-login", web::post().to(auth::admin_login))
    );

    let auth_middleware = HttpAuthentication::bearer(jwt_validator);

    // Protected user route
    cfg.route(
        "/api/v1/me",
        web::get()
            .to(user::me)
            .wrap(auth_middleware.clone()),
    );

    // Population routes (Phase 1): seeds Aurora DSQL tables
    cfg.service(
        web::scope("/populate")
            .wrap(auth_middleware.clone())
            .route("",               web::post().to(simulation::start_populate))
            .route("/static",        web::post().to(simulation::start_populate_static))
            .route("/dynamic",       web::post().to(simulation::start_populate_dynamic))
            .route("/dates",         web::get().to(simulation::get_populated_dates_handler))
            .route("/reset-dynamic", web::post().to(simulation::reset_dynamic_data))
    );

    // Simulation routes (Phase 2): DynamoDB day-of writes + control endpoints
    cfg.service(
        web::scope("/simulate")
            .wrap(auth_middleware.clone())
            .route("",              web::post().to(simulation::start_simulate))
            .route("/stop",         web::post().to(simulation::stop_simulation))
            .route("/status",       web::get().to(simulation::get_status))
            .route("/db-counts",   web::get().to(simulation::get_db_counts))
            .route("/reset",        web::post().to(simulation::reset_data))
            .route("/reset-dynamo", web::post().to(simulation::reset_dynamo))
            .route("/timelapse",   web::post().to(simulation::start_timelapse))
            .route("/heatmap",      web::get().to(simulation::get_heatmap))
            .route("/visitors",    web::get().to(simulation::get_visitors))
            .route("/date-range",  web::post().to(simulation::start_date_range_simulate))
            .route("/replay",      web::post().to(simulation::start_replay))
            .route("/replay-reset",web::post().to(simulation::reset_replay))
    );

    // Admin routes: schema management
    cfg.service(
        web::scope("/admin")
            .wrap(auth_middleware)
            .route("/init-db", web::post().to(simulation::init_database))
    );
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_route_configuration() {
        // Compile-time check that routes are correctly structured.
        // Full integration tests would require a running server.
    }
}
