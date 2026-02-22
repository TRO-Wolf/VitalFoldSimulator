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
    let simulator_state = web::Data::new(Arc::new(SimulatorState::new()));

    // Print startup banner
    tracing::info!(
        "Starting HTTP server on {}:{}",
        config.host,
        config.port
    );

    // Start the HTTP server
    HttpServer::new(move || {
        App::new()
            // Logging middleware
            .wrap(TracingLogger::default())
            // App state
            .app_data(web::Data::new(pool.clone()))
            .app_data(web::Data::new(config.clone()))
            .app_data(simulator_state.clone())
            // Routes
            .configure(routes::configure)
    })
    .bind((config.host.as_str(), config.port))?
    .run()
    .await
}
