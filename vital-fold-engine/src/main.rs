/// VitalFold Engine — Synthetic Health Data Simulator
///
/// A production-grade REST API for generating and managing synthetic health clinic data
/// using Aurora DSQL and DynamoDB. Includes JWT-based authentication and simulation control.

mod config;
mod db;
mod engine_state;
mod errors;
mod generators;
mod handlers;
mod middleware;
mod models;
mod routes;

use actix_web::{web, App, HttpServer, middleware::Logger};
use engine_state::SimulatorState;
use std::sync::Arc;
use tracing_actix_web::TracingLogger;
use tracing_subscriber::EnvFilter;
use utoipa::OpenApi;
use utoipa_swagger_ui::SwaggerUi;
use handlers::{health, auth, user, simulation};
use models::{RegisterRequest, LoginRequest, AuthResponse, UserProfile, MessageResponse, SimulationStatusResponse};
use engine_state::SimulationCounts;

#[derive(OpenApi)]
#[openapi(
    info(
        title = "VitalFold Engine API",
        version = "1.0.0",
        description = "Synthetic health data generation and simulation API for cardiac clinic data",
        contact(
            name = "VitalFold Team",
            email = "api@vitalfold.example.com"
        )
    ),
    paths(
        health::health_check,
        auth::register,
        auth::login,
        user::me,
        simulation::start_simulation,
        simulation::stop_simulation,
        simulation::get_status,
        simulation::reset_data
    ),
    components(
        schemas(
            RegisterRequest,
            LoginRequest,
            AuthResponse,
            UserProfile,
            MessageResponse,
            SimulationStatusResponse,
            SimulationCounts
        )
    ),
    tags(
        (name = "Health", description = "Health check endpoints"),
        (name = "Authentication", description = "User registration and login"),
        (name = "User", description = "User profile management"),
        (name = "Simulation", description = "Data generation simulation control")
    ),
    modifiers(&SecurityAddon)
)]
struct ApiDoc;

struct SecurityAddon;

impl utoipa::Modify for SecurityAddon {
    fn modify(&self, openapi: &mut utoipa::openapi::OpenApi) {
        if let Some(components) = openapi.components.as_mut() {
            components.add_security_scheme(
                "bearer_auth",
                utoipa::openapi::security::SecurityScheme::Http(
                    utoipa::openapi::security::HttpBuilder::new()
                        .scheme(utoipa::openapi::security::HttpAuthScheme::Bearer)
                        .bearer_format("JWT")
                        .description(Some("Enter your JWT token"))
                        .build()
                )
            )
        }
    }
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    // Initialize tracing
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::from_default_env()
                .add_directive("vital_fold_engine=info".parse().unwrap())
                .add_directive("actix_web=info".parse().unwrap()),
        )
        .init();

    tracing::info!("Starting VitalFold Engine");

    // Load configuration from environment
    let config = config::Config::from_env()
        .expect("Failed to load configuration from environment");

    tracing::info!(
        "Configuration loaded: host={}, port={}, endpoint={}",
        config.host,
        config.port,
        config.dsql_endpoint
    );

    // Create database connection pool
    let pool = db::create_pool(&config)
        .await
        .expect("Failed to create database pool");

    tracing::info!("Database pool created successfully");

    // Create global simulator state
    let simulator_state = web::Data::new(SimulatorState::new());

    // Print startup banner
    tracing::info!(
        "Starting HTTP server on {}:{}",
        config.host,
        config.port
    );

    // Clone config for use after move into closure
    let host = config.host.clone();
    let port = config.port;

    // Start the HTTP server
    HttpServer::new(move || {
        App::new()
            // Logging middleware
            .wrap(TracingLogger::default())
            // App state
            .app_data(web::Data::new(pool.clone()))
            .app_data(web::Data::new(config.clone()))
            .app_data(simulator_state.clone())
            // Swagger UI
            .service(
                SwaggerUi::new("/swagger-ui/{_:.*}")
                    .url("/api-docs/openapi.json", ApiDoc::openapi())
            )
            // Routes
            .configure(routes::configure)
    })
    .bind((host.as_str(), port))?
    .run()
    .await
}
