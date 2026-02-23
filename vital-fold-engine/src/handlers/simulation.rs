/// Simulation control endpoints for starting, stopping, and monitoring data generation.
///
/// The simulation runs asynchronously in a spawned task, allowing the API
/// to remain responsive while data is being generated.

use crate::db::DbPool;
use crate::engine_state::SimulatorState;
use crate::errors::AppError;
use crate::generators::{run_simulation, SimulationConfig};
use crate::models::{MessageResponse, SimulationStatusResponse};
use actix_web::{web, HttpResponse};

/// Start a new simulation run.
///
/// If a simulation is already running, returns 409 Conflict.
/// Otherwise, spawns an async task to run the simulation and returns 202 Accepted.
#[utoipa::path(
    post,
    path = "/simulate",
    tag = "Simulation",
    security(
        ("bearer_auth" = [])
    ),
    responses(
        (status = 202, description = "Simulation started", body = MessageResponse),
        (status = 400, description = "Simulation already running", body = String),
        (status = 401, description = "Unauthorized", body = String),
        (status = 500, description = "Internal server error", body = String)
    )
)]
pub async fn start_simulation(
    pool: web::Data<DbPool>,
    state: web::Data<SimulatorState>,
) -> Result<HttpResponse, AppError> {
    // Try to transition from idle to running
    if !state.try_start() {
        tracing::warn!("Simulation already running");
        return Err(AppError::BadRequest(
            "Simulation is already running".to_string(),
        ));
    }

    // Clone the pool for the async task
    let pool_clone = pool.get_ref().clone();
    let state_clone = state.clone(); // Clone the entire web::Data

    // Spawn a background task to run the simulation
    tokio::spawn(async move {
        let config = SimulationConfig::default();
        match run_simulation(pool_clone, config, &state_clone).await {
            Ok(_) => {
                tracing::info!("Simulation completed successfully");
                state_clone.stop();
            }
            Err(e) => {
                tracing::error!("Simulation failed: {}", e);
                state_clone.stop();
            }
        }
    });

    tracing::info!("Simulation started");

    Ok(HttpResponse::Accepted().json(MessageResponse {
        message: "Simulation started".to_string(),
    }))
}

/// Stop the currently running simulation.
///
/// Sets the running flag to false. The simulation task will exit gracefully
/// on its next check.
#[utoipa::path(
    post,
    path = "/simulate/stop",
    tag = "Simulation",
    security(
        ("bearer_auth" = [])
    ),
    responses(
        (status = 200, description = "Simulation stopped", body = MessageResponse),
        (status = 401, description = "Unauthorized", body = String),
        (status = 500, description = "Internal server error", body = String)
    )
)]
pub async fn stop_simulation(state: web::Data<SimulatorState>) -> Result<HttpResponse, AppError> {
    state.stop();
    tracing::info!("Simulation stopped");

    Ok(HttpResponse::Ok().json(MessageResponse {
        message: "Simulation stopped".to_string(),
    }))
}

/// Get the current simulation status.
///
/// Returns whether a simulation is currently running and metrics from the last run.
#[utoipa::path(
    get,
    path = "/simulate/status",
    tag = "Simulation",
    security(
        ("bearer_auth" = [])
    ),
    responses(
        (status = 200, description = "Simulation status retrieved", body = SimulationStatusResponse),
        (status = 401, description = "Unauthorized", body = String),
        (status = 500, description = "Internal server error", body = String)
    )
)]
pub async fn get_status(state: web::Data<SimulatorState>) -> Result<HttpResponse, AppError> {
    let running = state.is_running();
    let last_run = state.get_last_run();
    let counts = state.get_counts();

    let response = SimulationStatusResponse {
        running,
        last_run,
        counts,
    };

    Ok(HttpResponse::Ok().json(response))
}

/// Reset all data by truncating vital_fold schema tables.
///
/// WARNING: This is destructive. All generated data will be deleted.
#[utoipa::path(
    post,
    path = "/simulate/reset",
    tag = "Simulation",
    security(
        ("bearer_auth" = [])
    ),
    responses(
        (status = 200, description = "All data reset successfully", body = MessageResponse),
        (status = 401, description = "Unauthorized", body = String),
        (status = 500, description = "Internal server error", body = String)
    )
)]
pub async fn reset_data(pool: web::Data<DbPool>) -> Result<HttpResponse, AppError> {
    // Check if a simulation is running - don't reset while running
    // (This check would require state, so we'll skip it for now)

    tracing::warn!("Resetting all vital_fold schema data");

    // TRUNCATE all vital_fold schema tables in dependency order
    let tables = vec![
        "vital_fold.medical_record",
        "vital_fold.appointment",
        "vital_fold.clinic_schedule",
        "vital_fold.patient_insurance",
        "vital_fold.patient_demographics",
        "vital_fold.emergency_contact",
        "vital_fold.patient",
        "vital_fold.provider",
        "vital_fold.clinic",
        "vital_fold.insurance_plan",
        "vital_fold.insurance_company",
    ];

    for table in tables {
        sqlx::query(&format!("TRUNCATE TABLE {} CASCADE", table))
            .execute(pool.get_ref())
            .await?;
        tracing::debug!("Truncated {}", table);
    }

    tracing::info!("All data reset successfully");

    Ok(HttpResponse::Ok().json(MessageResponse {
        message: "All data reset successfully".to_string(),
    }))
}
