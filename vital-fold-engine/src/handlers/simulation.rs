/// Simulation and population control endpoints.
///
/// # Two-Phase Data Lifecycle
///
/// ## POST /populate — start_populate()
/// Seeds all Aurora DSQL tables with synthetic healthcare data:
/// patients, providers, clinics, appointments (1–89 days out), medical records, etc.
/// Returns 202 Accepted immediately; runs in a background task.
/// Poll GET /simulate/status to check completion.
///
/// ## POST /simulate — start_simulate()
/// Runs the day-of simulation: queries Aurora for appointments scheduled today,
/// then writes patient_visit and patient_vitals records to DynamoDB for each.
/// Models real-time EHR data capture on the day of the visit.
/// Returns 202 Accepted immediately; runs in a background task.
///
/// Both endpoints are guarded by a single running flag — only one may be active at a time.

use crate::db::DbPool;
use crate::engine_state::SimulatorState;
use crate::errors::AppError;
use crate::generators::{run_populate, run_simulate, run_today_heatmap, run_heatmap_replay, SimulationConfig};
use crate::models::{MessageResponse, SimulationStatusResponse};
use actix_web::{web, HttpResponse};
use aws_sdk_dynamodb::Client as DynamoClient;
use aws_sdk_dynamodb::types::{DeleteRequest, WriteRequest};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Request body for starting a populate run.
/// All fields are optional — omit any field to use its default value.
#[derive(Debug, Deserialize, utoipa::ToSchema)]
pub struct PopulateRequest {
    /// Number of insurance plans generated per insurance company (default: 3)
    #[schema(example = 3)]
    pub plans_per_company: Option<usize>,

    /// Number of providers to generate (default: 50)
    #[schema(example = 50)]
    pub providers: Option<usize>,

    /// Number of patients to generate (default: 50000)
    #[schema(example = 50000)]
    pub patients: Option<usize>,

    /// Number of appointments to generate per patient, dated 1–89 days in the future (default: 2)
    #[schema(example = 2)]
    pub appointments_per_patient: Option<usize>,

    /// Number of medical records to generate per appointment (default: 1)
    #[schema(example = 1)]
    pub records_per_appointment: Option<usize>,
}



/// Seed all Aurora DSQL tables with synthetic healthcare data.
///
/// Generates insurance companies (7 fixed), insurance plans, clinics (10 fixed SE US),
/// providers, patients, emergency contacts, demographics, insurance links, clinic schedules,
/// appointments (1–89 days in the future), and medical records.
///
/// Does NOT write to DynamoDB. DynamoDB records are written by POST /simulate on the day
/// of each appointment.
///
/// All body fields are optional. Omit any field to use its default value.
/// Defaults: `plans_per_company=3`, `providers=50`, `patients=50000`,
/// `appointments_per_patient=2`, `records_per_appointment=1`.
///
/// If a run is already in progress, returns 409 Conflict.
/// Otherwise, spawns an async background task and returns 202 Accepted.
#[utoipa::path(
    post,
    path = "/populate",
    tag = "Population",
    security(
        ("bearer_auth" = [])
    ),
    request_body(
        content = PopulateRequest,
        description = "Optional populate parameters. Omit any field to use its default value.",
        content_type = "application/json"
    ),
    responses(
        (status = 202, description = "Population started", body = MessageResponse),
        (status = 409, description = "A run is already in progress", body = String),
        (status = 401, description = "Unauthorized", body = String),
        (status = 500, description = "Internal server error", body = String)
    )
)]
pub async fn start_populate(
    pool: web::Data<DbPool>,
    dynamo: web::Data<DynamoClient>,
    state: web::Data<SimulatorState>,
    body: Option<web::Json<PopulateRequest>>,
) -> Result<HttpResponse, AppError> {
    // Atomically transition from idle → running; reject if already active
    if !state.try_start() {
        tracing::warn!("Populate rejected: a run is already in progress");
        return Err(AppError::BadRequest(
            "A run is already in progress".to_string(),
        ));
    }

    // Build SimulationConfig from request body, falling back to defaults.
    // The nonzero filter treats None AND explicit 0 (e.g. from Swagger UI) as "use default".
    let defaults = SimulationConfig::default();
    let config = match body {
        Some(req) => {
            let nonzero = |v: Option<usize>, d: usize| v.filter(|&n| n > 0).unwrap_or(d);
            SimulationConfig {
                plans_per_company:        nonzero(req.plans_per_company,        defaults.plans_per_company),
                providers:                nonzero(req.providers,                defaults.providers),
                patients:                 nonzero(req.patients,                 defaults.patients),
                appointments_per_patient: nonzero(req.appointments_per_patient, defaults.appointments_per_patient),
                records_per_appointment:  nonzero(req.records_per_appointment,  defaults.records_per_appointment),
            }
        }
        None => defaults,
    };

    let pool_clone   = pool.get_ref().clone();
    let dynamo_clone = dynamo.get_ref().clone();
    let state_clone  = state.clone();

    tokio::spawn(async move {
        match run_populate(pool_clone, dynamo_clone, config, &state_clone).await {
            Ok(_)  => tracing::info!("Populate completed successfully"),
            Err(e) => tracing::error!("Populate failed: {}", e),
        }
        state_clone.stop();
    });

    tracing::info!("Populate started");

    Ok(HttpResponse::Accepted().json(MessageResponse {
        message: "Population started".to_string(),
    }))
}




/// Write DynamoDB records for all appointments scheduled for today.
///
/// Queries Aurora DSQL for appointments where `appointment_date::date = CURRENT_DATE`,
/// then writes a `patient_visit` record and a `patient_vitals` record to DynamoDB for each.
///
/// This models real-time EHR data capture: vitals and check-in data are only recorded
/// on the day the patient actually arrives for their appointment.
///
/// No request body — the set of today's appointments is derived directly from Aurora.
///
/// If no appointments are found for today, returns 202 with a warning logged.
/// If a run is already in progress, returns 409 Conflict.
#[utoipa::path(
    post,
    path = "/simulate",
    tag = "Simulation",
    security(
        ("bearer_auth" = [])
    ),
    responses(
        (status = 202, description = "Simulation started", body = MessageResponse),
        (status = 409, description = "A run is already in progress", body = String),
        (status = 401, description = "Unauthorized", body = String),
        (status = 500, description = "Internal server error", body = String)
    )
)]
pub async fn start_simulate(
    pool: web::Data<DbPool>,
    dynamo: web::Data<DynamoClient>,
    state: web::Data<SimulatorState>,
) -> Result<HttpResponse, AppError> {
    if !state.try_start() {
        tracing::warn!("Simulate rejected: a run is already in progress");
        return Err(AppError::BadRequest(
            "A run is already in progress".to_string(),
        ));
    }

    let pool_clone   = pool.get_ref().clone();
    let dynamo_clone = dynamo.get_ref().clone();
    let state_clone  = state.clone();

    tokio::spawn(async move {
        match run_simulate(pool_clone, dynamo_clone, &state_clone).await {
            Ok(_)  => tracing::info!("Simulate completed successfully"),
            Err(e) => tracing::error!("Simulate failed: {}", e),
        }
        state_clone.stop();
    });

    tracing::info!("Simulate started");

    Ok(HttpResponse::Accepted().json(MessageResponse {
        message: "Simulation started".to_string(),
    }))
}



/// Stop the currently running populate or simulate job.
///
/// Sets the running flag to false. The background task will exit gracefully.
#[utoipa::path(
    post,
    path = "/simulate/stop",
    tag = "Simulation",
    security(
        ("bearer_auth" = [])
    ),
    responses(
        (status = 200, description = "Run stopped", body = MessageResponse),
        (status = 401, description = "Unauthorized", body = String),
        (status = 500, description = "Internal server error", body = String)
    )
)]
pub async fn stop_simulation(state: web::Data<SimulatorState>) -> Result<HttpResponse, AppError> {
    state.stop();
    tracing::info!("Run stopped");

    Ok(HttpResponse::Ok().json(MessageResponse {
        message: "Run stopped".to_string(),
    }))
}



/// Get the current run status and counts from the last completed job.
///
/// Returns whether a populate or simulate job is currently running, the timestamp
/// of the last completed run, and row counts broken down by table.
///
/// Aurora DSQL counts (set by POST /populate):
///   insurance_companies, insurance_plans, clinics, providers, patients,
///   emergency_contacts, patient_demographics, patient_insurance,
///   clinic_schedules, appointments, medical_records
///
/// DynamoDB counts (set by POST /simulate):
///   dynamo_patient_visits, dynamo_patient_vitals
#[utoipa::path(
    get,
    path = "/simulate/status",
    tag = "Simulation",
    security(
        ("bearer_auth" = [])
    ),
    responses(
        (status = 200, description = "Status retrieved", body = SimulationStatusResponse),
        (status = 401, description = "Unauthorized", body = String),
        (status = 500, description = "Internal server error", body = String)
    )
)]
pub async fn get_status(state: web::Data<SimulatorState>) -> Result<HttpResponse, AppError> {
    let running  = state.is_running();
    let last_run = state.get_last_run();
    let counts   = state.get_counts();

    let response = SimulationStatusResponse {
        running,
        last_run,
        counts,
    };

    Ok(HttpResponse::Ok().json(response))
}



/// Request body for starting a heatmap visualization.
/// All fields optional — omit to use defaults.
#[derive(Debug, Deserialize, utoipa::ToSchema)]
pub struct TimelapseRequest {
    /// Seconds between hour-window updates (default: 5)
    #[schema(example = 5)]
    pub window_interval_secs: Option<u64>,
}

/// Start a single-day heatmap visualization for today's appointments.
///
/// Animates hour-by-hour (9am–5pm) appointment counts per clinic on the
/// current date. If DynamoDB hasn't been populated yet, auto-triggers
/// `run_simulate` first to write patient_visit + patient_vitals records.
///
/// Poll `GET /simulate/heatmap` at 1–2 second intervals to drive the
/// frontend heatmap canvas.
///
/// Default: `window_interval_secs=5` (~40 seconds for the full day sweep).
#[utoipa::path(
    post,
    path = "/simulate/timelapse",
    tag = "Simulation",
    security(
        ("bearer_auth" = [])
    ),
    request_body(
        content = TimelapseRequest,
        description = "Optional heatmap parameters.",
        content_type = "application/json"
    ),
    responses(
        (status = 202, description = "Heatmap started", body = MessageResponse),
        (status = 400, description = "A run is already in progress", body = String),
        (status = 401, description = "Unauthorized", body = String),
        (status = 500, description = "Internal server error", body = String)
    )
)]
pub async fn start_timelapse(
    pool: web::Data<DbPool>,
    dynamo: web::Data<DynamoClient>,
    state: web::Data<SimulatorState>,
    body: Option<web::Json<TimelapseRequest>>,
) -> Result<HttpResponse, AppError> {
    if !state.try_start() {
        tracing::warn!("Heatmap rejected: a run is already in progress");
        return Err(AppError::BadRequest(
            "A run is already in progress".to_string(),
        ));
    }

    let interval = body.as_ref().and_then(|b| b.window_interval_secs).unwrap_or(5);

    // Clear any previous timelapse state
    state.set_timelapse(None);

    let pool_clone   = pool.get_ref().clone();
    let dynamo_clone = dynamo.get_ref().clone();
    let state_clone  = state.clone();

    tokio::spawn(async move {
        match run_today_heatmap(pool_clone, dynamo_clone, &state_clone, interval).await {
            Ok(_)  => tracing::info!("Heatmap completed successfully"),
            Err(e) => tracing::error!("Heatmap failed: {}", e),
        }
        state_clone.stop();
    });

    tracing::info!("Heatmap started — {}s per window", interval);

    Ok(HttpResponse::Accepted().json(MessageResponse {
        message: "Heatmap started".to_string(),
    }))
}



/// Get the current timelapse heatmap state.
///
/// Returns per-clinic appointment activity for the current simulated day and hour.
/// Poll this at 1–2 second intervals while a timelapse is running to drive the
/// frontend heatmap visualization.
///
/// Returns `{ "active": false }` if no timelapse has been started.
#[utoipa::path(
    get,
    path = "/simulate/heatmap",
    tag = "Simulation",
    security(
        ("bearer_auth" = [])
    ),
    responses(
        (status = 200, description = "Heatmap data retrieved"),
        (status = 401, description = "Unauthorized", body = String),
        (status = 500, description = "Internal server error", body = String)
    )
)]
pub async fn get_heatmap(state: web::Data<SimulatorState>) -> Result<HttpResponse, AppError> {
    match state.get_timelapse() {
        Some(ts) => Ok(HttpResponse::Ok().json(ts)),
        None => Ok(HttpResponse::Ok().json(serde_json::json!({ "active": false }))),
    }
}

/// Start a read-only heatmap replay using existing Aurora appointment data.
///
/// Unlike `start_timelapse`, this does **not** write to or read from DynamoDB.
/// It replays the hour-by-hour animation for today's appointments, making it
/// safe for non-admin users.
#[utoipa::path(
    post,
    path = "/simulate/replay",
    tag = "Simulation",
    security(
        ("bearer_auth" = [])
    ),
    request_body(
        content = TimelapseRequest,
        description = "Optional replay parameters.",
        content_type = "application/json"
    ),
    responses(
        (status = 202, description = "Heatmap replay started", body = MessageResponse),
        (status = 400, description = "A run is already in progress", body = String),
        (status = 401, description = "Unauthorized", body = String),
        (status = 500, description = "Internal server error", body = String)
    )
)]
pub async fn start_replay(
    pool: web::Data<DbPool>,
    state: web::Data<SimulatorState>,
    body: Option<web::Json<TimelapseRequest>>,
) -> Result<HttpResponse, AppError> {
    if !state.try_start() {
        tracing::warn!("Replay rejected: a run is already in progress");
        return Err(AppError::BadRequest(
            "A run is already in progress".to_string(),
        ));
    }

    let interval = body.as_ref().and_then(|b| b.window_interval_secs).unwrap_or(5);

    state.set_timelapse(None);

    let pool_clone  = pool.get_ref().clone();
    let state_clone = state.clone();

    tokio::spawn(async move {
        match run_heatmap_replay(pool_clone, &state_clone, interval).await {
            Ok(_)  => tracing::info!("Replay completed successfully"),
            Err(e) => tracing::error!("Replay failed: {}", e),
        }
        state_clone.stop();
    });

    tracing::info!("Replay started — {}s per window", interval);

    Ok(HttpResponse::Accepted().json(MessageResponse {
        message: "Heatmap replay started".to_string(),
    }))
}

/// Clear the heatmap replay state without deleting any data.
#[utoipa::path(
    post,
    path = "/simulate/replay-reset",
    tag = "Simulation",
    security(
        ("bearer_auth" = [])
    ),
    responses(
        (status = 200, description = "Replay state cleared", body = MessageResponse),
        (status = 401, description = "Unauthorized", body = String)
    )
)]
pub async fn reset_replay(
    state: web::Data<SimulatorState>,
) -> Result<HttpResponse, AppError> {
    state.set_timelapse(None);
    tracing::info!("Replay state cleared");
    Ok(HttpResponse::Ok().json(MessageResponse {
        message: "Replay state cleared".to_string(),
    }))
}

/// A single visitor row: patient name + clinic info.
#[derive(sqlx::FromRow)]
struct VisitorRow {
    first_name: String,
    last_name: String,
    clinic_name: String,
    city: String,
    state: String,
    hour: i32,
}

/// A visitor entry within a clinic group.
#[derive(Serialize, utoipa::ToSchema)]
pub struct VisitorEntry {
    pub first_name: String,
    pub last_name: String,
    pub hour: u32,
}

/// Visitors grouped by clinic.
#[derive(Serialize, utoipa::ToSchema)]
pub struct ClinicVisitors {
    pub clinic_name: String,
    pub city: String,
    pub state: String,
    pub visitors: Vec<VisitorEntry>,
}

/// Response for the visitors endpoint.
#[derive(Serialize, utoipa::ToSchema)]
pub struct VisitorsResponse {
    pub date: String,
    pub clinics: Vec<ClinicVisitors>,
}

/// Get today's visitors (patient names) grouped by clinic.
///
/// Joins `appointment`, `patient`, and `clinic` tables to return the first/last name
/// of every patient with an appointment today, grouped by clinic with city/state info.
#[utoipa::path(
    get,
    path = "/simulate/visitors",
    tag = "Simulation",
    security(
        ("bearer_auth" = [])
    ),
    responses(
        (status = 200, description = "Visitor list retrieved", body = VisitorsResponse),
        (status = 401, description = "Unauthorized", body = String),
        (status = 500, description = "Internal server error", body = String)
    )
)]
pub async fn get_visitors(
    pool: web::Data<DbPool>,
) -> Result<HttpResponse, AppError> {
    let rows: Vec<VisitorRow> = sqlx::query_as(
        "SELECT p.first_name, p.last_name, c.clinic_name, c.city, c.state, \
                EXTRACT(HOUR FROM a.appointment_date)::INTEGER as hour \
         FROM vital_fold.appointment a \
         JOIN vital_fold.patient p ON p.patient_id = a.patient_id \
         JOIN vital_fold.clinic c ON c.clinic_id = a.clinic_id \
         WHERE a.appointment_date::date = CURRENT_DATE \
         ORDER BY c.clinic_name, a.appointment_date"
    )
    .fetch_all(pool.get_ref())
    .await?;

    let today = chrono::Utc::now().format("%Y-%m-%d").to_string();

    // Group by clinic_name
    let mut clinic_map: Vec<(String, String, String, Vec<VisitorEntry>)> = Vec::new();
    for row in rows {
        let entry = VisitorEntry {
            first_name: row.first_name,
            last_name: row.last_name,
            hour: row.hour as u32,
        };
        if let Some(last) = clinic_map.last_mut() {
            if last.0 == row.clinic_name {
                last.3.push(entry);
                continue;
            }
        }
        clinic_map.push((row.clinic_name, row.city, row.state, vec![entry]));
    }

    let clinics = clinic_map
        .into_iter()
        .map(|(clinic_name, city, state, visitors)| ClinicVisitors {
            clinic_name,
            city,
            state,
            visitors,
        })
        .collect();

    Ok(HttpResponse::Ok().json(VisitorsResponse {
        date: today,
        clinics,
    }))
}



/// Reset all data by deleting all rows from vital_fold schema tables.
///
/// WARNING: This is destructive. All generated Aurora DSQL data will be deleted.
/// DynamoDB tables are not affected — clear those independently if needed.
///
/// Returns 400 Bad Request if a populate or simulate run is currently in progress.
///
/// Aurora DSQL does not support TRUNCATE or ctid. Rows are deleted in 2500-row
/// batches using `DELETE FROM t WHERE pk IN (SELECT pk FROM t LIMIT 2500)`,
/// looped per table until all rows are gone.
#[utoipa::path(
    post,
    path = "/simulate/reset",
    tag = "Simulation",
    security(
        ("bearer_auth" = [])
    ),
    responses(
        (status = 200, description = "All data reset successfully", body = MessageResponse),
        (status = 400, description = "Cannot reset while a run is in progress", body = String),
        (status = 401, description = "Unauthorized", body = String),
        (status = 500, description = "Internal server error", body = String)
    )
)]
pub async fn reset_data(
    pool: web::Data<DbPool>,
    state: web::Data<SimulatorState>,
) -> Result<HttpResponse, AppError> {
    if state.is_running() {
        return Err(AppError::BadRequest(
            "Cannot reset while a run is in progress".to_string(),
        ));
    }

    tracing::warn!("Resetting all vital_fold schema data");

    // Delete in FK-safe dependency order (children before parents).
    // Aurora DSQL does not support ctid or TRUNCATE, and has a 3000-row per-transaction
    // limit. Each table is deleted in 2500-row batches using a subquery on the PK.
    let tables: &[(&str, &str)] = &[
        ("vital_fold.medical_record",      "medical_record_id"),
        ("vital_fold.appointment",         "appointment_id"),
        ("vital_fold.clinic_schedule",     "schedule_id"),
        ("vital_fold.patient_insurance",   "patient_insurance_id"),
        ("vital_fold.patient_demographics","demographics_id"),
        ("vital_fold.emergency_contact",   "emergency_contact_id"),
        ("vital_fold.patient",             "patient_id"),
        ("vital_fold.provider",            "provider_id"),
        ("vital_fold.clinic",              "clinic_id"),
        ("vital_fold.insurance_plan",      "insurance_plan_id"),
        ("vital_fold.insurance_company",   "company_id"),
    ];

    for (table, pk) in tables {
        let mut total_deleted: u64 = 0;
        loop {
            let result = sqlx::query(&format!(
                "DELETE FROM {table} WHERE {pk} IN (SELECT {pk} FROM {table} LIMIT 2500)"
            ))
            .execute(pool.get_ref())
            .await?;

            let rows = result.rows_affected();
            total_deleted += rows;
            if rows == 0 {
                break;
            }
        }
        tracing::debug!("Deleted {} rows from {}", total_deleted, table);
    }

    // Zero out Aurora counts so the dashboard reflects the cleared state.
    let mut counts = state.get_counts();
    counts.insurance_companies = 0;
    counts.insurance_plans = 0;
    counts.clinics = 0;
    counts.providers = 0;
    counts.patients = 0;
    counts.emergency_contacts = 0;
    counts.patient_demographics = 0;
    counts.patient_insurance = 0;
    counts.clinic_schedules = 0;
    counts.appointments = 0;
    counts.medical_records = 0;
    state.set_counts(counts);

    tracing::info!("All data reset successfully");

    Ok(HttpResponse::Ok().json(MessageResponse {
        message: "All data reset successfully".to_string(),
    }))
}



/// Delete all items from both DynamoDB tables: `patient_visit` and `patient_vitals`.
///
/// WARNING: Destructive. Scans each table and deletes all items in batches.
/// DynamoDB has no TRUNCATE equivalent — items must be scanned and deleted one batch at a time.
///
/// Strategy per table:
/// 1. `scan` projecting only key attributes (patient_id, clinic_id) — up to 1 MB per page.
/// 2. Group scan results into chunks of 25 (DynamoDB BatchWriteItem maximum).
/// 3. Call `batch_write_item` with up to 25 `DeleteRequest` entries per call.
/// 4. Retry any `UnprocessedItems` returned by DynamoDB until all are confirmed deleted.
/// 5. Repeat scan pages until `LastEvaluatedKey` is absent (table fully scanned).
///
/// Both tables share the same key schema (PK=patient_id S, SK=clinic_id S).
///
/// Runs inline (synchronous, not spawned). Returns 200 OK when both tables are empty.
/// Returns 400 Bad Request if a populate or simulate run is currently in progress.
#[utoipa::path(
    post,
    path = "/simulate/reset-dynamo",
    tag = "Simulation",
    security(
        ("bearer_auth" = [])
    ),
    responses(
        (status = 200, description = "DynamoDB tables cleared", body = MessageResponse),
        (status = 400, description = "Cannot reset while a run is in progress", body = String),
        (status = 401, description = "Unauthorized", body = String),
        (status = 500, description = "Internal server error", body = String)
    )
)]
pub async fn reset_dynamo(
    dynamo: web::Data<DynamoClient>,
    state: web::Data<SimulatorState>,
) -> Result<HttpResponse, AppError> {
    if state.is_running() {
        return Err(AppError::BadRequest(
            "Cannot reset while a run is in progress".to_string(),
        ));
    }

    tracing::warn!("Resetting all DynamoDB data");

    // Both tables share the same key schema: PK=patient_id (S), SK=clinic_id (S)
    let tables = [
        ("patient_visit",   "patient_id", "clinic_id"),
        ("patient_vitals",  "patient_id", "clinic_id"),
    ];

    for (table, pk_name, sk_name) in &tables {
        let deleted = delete_dynamo_table(dynamo.get_ref(), table, pk_name, sk_name).await?;
        tracing::info!("Deleted {} items from DynamoDB table '{}'", deleted, table);
    }

    // Zero out the in-memory DynamoDB counts so that run_today_heatmap's
    // auto-populate check (dynamo_patient_visits == 0) correctly re-triggers
    // run_simulate on the next heatmap run.
    let mut counts = state.get_counts();
    counts.dynamo_patient_visits = 0;
    counts.dynamo_patient_vitals = 0;
    state.set_counts(counts);

    tracing::info!("DynamoDB reset complete");

    Ok(HttpResponse::Ok().json(MessageResponse {
        message: "DynamoDB tables cleared successfully".to_string(),
    }))
}




/// Scan a DynamoDB table and delete every item using BatchWriteItem.
///
/// Scans projecting only the two key attributes to minimise read cost. Items are
/// deleted in chunks of 25 (the BatchWriteItem maximum).
///
/// ## Throttling handling
/// On-demand tables start with a burst allowance then cap at a provisioned baseline.
/// Deleting tens of thousands of items in rapid succession exhausts that burst,
/// causing `ThrottlingException` on the *entire* BatchWriteItem call (not just
/// individual items in `UnprocessedItems`).
///
/// Two mechanisms keep throughput under control:
/// 1. **Inter-chunk pacing** — a fixed `CHUNK_DELAY_MS` sleep between every chunk
///    limits the sustained write rate to roughly (25 deletes / delay) per second.
///    At 50 ms this is ~500 WCU/s, well under the on-demand baseline.
/// 2. **Exponential backoff on ThrottlingException** — if the pacing is
///    insufficient (e.g. the table was recently hammered by a populate run),
///    the batch call is retried up to `MAX_RETRIES` times with doubling delays
///    starting at `BACKOFF_BASE_MS`.
///
/// `UnprocessedItems` (partial-batch failures unrelated to throttling) are also
/// retried under the same backoff policy.
///
/// Returns the total number of items deleted.
async fn delete_dynamo_table(
    dynamo: &DynamoClient,
    table: &str,
    pk_name: &str,
    sk_name: &str,
) -> Result<u64, AppError> {
    // 50 ms between chunks ≈ 500 WCU/s sustained — stays under on-demand baseline.
    const CHUNK_DELAY_MS: u64 = 50;
    // Backoff starts at 1 s and doubles each retry: 1 s, 2 s, 4 s, 8 s, 16 s.
    const BACKOFF_BASE_MS: u64 = 1_000;
    const MAX_RETRIES: u32 = 5;

    let mut deleted: u64 = 0;
    let mut last_key: Option<HashMap<String, aws_sdk_dynamodb::types::AttributeValue>> = None;

    loop {
        // Scan one page, projecting only key attributes to minimise RCU cost.
        let mut scan = dynamo
            .scan()
            .table_name(table)
            .projection_expression("#pk, #sk")
            .expression_attribute_names("#pk", pk_name)
            .expression_attribute_names("#sk", sk_name);

        if let Some(ref key) = last_key {
            scan = scan.set_exclusive_start_key(Some(key.clone()));
        }

        let scan_result = scan
            .send()
            .await
            .map_err(|e| AppError::Internal(format!("DynamoDB scan failed on '{}': {:?}", table, e)))?;

        let items = scan_result.items.unwrap_or_default();
        last_key = scan_result.last_evaluated_key;

        if !items.is_empty() {
            // BatchWriteItem accepts at most 25 requests per call.
            for chunk in items.chunks(25) {
                let requests: Vec<WriteRequest> = chunk
                    .iter()
                    .filter_map(|item| {
                        let pk_val = item.get(pk_name)?.clone();
                        let sk_val = item.get(sk_name)?.clone();
                        let del = DeleteRequest::builder()
                            .key(pk_name, pk_val)
                            .key(sk_name, sk_val)
                            .build()
                            .ok()?;
                        Some(WriteRequest::builder().delete_request(del).build())
                    })
                    .collect();

                let chunk_len = requests.len() as u64;

                // Retry loop — handles both UnprocessedItems and ThrottlingException.
                // On a throttling error the entire batch is retried after backoff;
                // on partial failure only the unprocessed subset is retried.
                let mut pending: Option<HashMap<String, Vec<WriteRequest>>> =
                    Some(HashMap::from([(table.to_string(), requests)]));
                let mut attempt: u32 = 0;

                while let Some(batch) = pending.take() {
                    if batch.is_empty() {
                        break;
                    }

                    match dynamo
                        .batch_write_item()
                        .set_request_items(Some(batch.clone()))
                        .send()
                        .await
                    {
                        Ok(resp) => {
                            let unprocessed = resp.unprocessed_items.unwrap_or_default();
                            pending = if unprocessed.is_empty() { None } else { Some(unprocessed) };
                            attempt = 0; // reset on success
                        }
                        Err(e) => {
                            // Check if this is a retryable throttling error.
                            let is_throttle = format!("{:?}", e).contains("ThrottlingException");
                            attempt += 1;
                            if is_throttle && attempt <= MAX_RETRIES {
                                let delay_ms = BACKOFF_BASE_MS * (1 << (attempt - 1));
                                tracing::warn!(
                                    "DynamoDB throttled on '{}' (attempt {}/{}), backing off {}ms",
                                    table, attempt, MAX_RETRIES, delay_ms
                                );
                                tokio::time::sleep(tokio::time::Duration::from_millis(delay_ms)).await;
                                pending = Some(batch); // retry the same batch
                            } else {
                                return Err(AppError::Internal(format!(
                                    "DynamoDB batch_write_item failed on '{}' after {} attempt(s): {:?}",
                                    table, attempt, e
                                )));
                            }
                        }
                    }
                }

                deleted += chunk_len;

                // Pace writes to avoid exhausting the on-demand burst allowance.
                tokio::time::sleep(tokio::time::Duration::from_millis(CHUNK_DELAY_MS)).await;
            }
        }

        if last_key.is_none() {
            break;
        }
    }

    Ok(deleted)
}
