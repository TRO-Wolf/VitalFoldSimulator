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
/// then writes to both DynamoDB tables (patient_visit + patient_vitals) for each.
/// Models real-time EHR data capture on the day of the visit.
/// Returns 202 Accepted immediately; runs in a background task.
///
/// Both endpoints are guarded by a single running flag — only one may be active at a time.

use crate::db::DbPool;
use crate::engine_state::SimulatorState;
use crate::errors::AppError;
use crate::generators::{run_populate, run_populate_static, run_populate_dynamic, get_populated_dates, run_simulate, run_date_range_simulate, run_today_heatmap, run_heatmap_replay, SimulationConfig, NUM_CLINICS, DEFAULT_CLINIC_WEIGHTS};
use chrono::NaiveDate;
use std::collections::HashSet;
use crate::models::{MessageResponse, SimulationStatusResponse};
use actix_web::{web, HttpResponse};
use aws_sdk_dynamodb::Client as DynamoClient;
use aws_sdk_dynamodb::types::{DeleteRequest, WriteRequest};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Validate and resolve clinic weights from an optional API input.
/// Returns DEFAULT_CLINIC_WEIGHTS if None, or validates length and positivity.
fn resolve_clinic_weights(input: Option<Vec<u32>>) -> Result<Vec<u32>, AppError> {
    match input {
        None => Ok(DEFAULT_CLINIC_WEIGHTS.to_vec()),
        Some(w) if w.len() != NUM_CLINICS => Err(AppError::BadRequest(
            format!("clinic_weights must have exactly {} entries (one per clinic), got {}", NUM_CLINICS, w.len()),
        )),
        Some(w) if w.iter().any(|&v| v == 0) => Err(AppError::BadRequest(
            "clinic_weights entries must all be > 0".to_string(),
        )),
        Some(w) => Ok(w),
    }
}

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

    /// Number of medical records to generate per appointment (default: 1)
    #[schema(example = 1)]
    pub records_per_appointment: Option<usize>,

    /// Appointment start date (inclusive), ISO 8601 YYYY-MM-DD (default: tomorrow)
    #[schema(example = "2026-03-23")]
    pub start_date: Option<String>,

    /// Appointment end date (inclusive), ISO 8601 YYYY-MM-DD (default: start_date + 89 days)
    #[schema(example = "2026-06-20")]
    pub end_date: Option<String>,

    /// Per-clinic weights (10 entries, one per clinic). Controls distribution of
    /// patients, providers, and appointments. Higher weight = more volume.
    /// Order: Charlotte, Asheville, Atlanta×2, Tallahassee, Miami×2, Orlando, Jacksonville×2.
    /// Default: [12, 3, 14, 14, 2, 14, 14, 12, 8, 8]
    pub clinic_weights: Option<Vec<u32>>,
}



/// Seed all Aurora DSQL tables with synthetic healthcare data.
///
/// Generates insurance companies (7 fixed), insurance plans, clinics (10 fixed SE US),
/// providers, patients, emergency contacts, demographics, insurance links, clinic schedules,
/// appointments (within configured date range), and medical records.
///
/// Does NOT write to DynamoDB. DynamoDB records are written by POST /simulate on the day
/// of each appointment.
///
/// All body fields are optional. Omit any field to use its default value.
/// Defaults: `plans_per_company=3`, `providers=50`, `patients=50000`,
/// `records_per_appointment=1`, `start_date=tomorrow`, `end_date=start_date + 89 days`.
/// Appointment volume is provider-driven: each provider fills 36 slots/day at their clinic.
///
/// If a run is already in progress, returns 409 Conflict.
/// Otherwise, spawns an async background task and returns 202 Accepted.
/// Poll `GET /simulate/status` to track progress via the `populate_progress` field.
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
        (status = 400, description = "Invalid date range", body = String),
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
    // Build and validate config before acquiring the running flag so that
    // date-parsing errors don't leave the state stuck in "running".
    let defaults = SimulationConfig::default();
    let config = match body {
        Some(json) => {
            let req = json.into_inner();
            let nonzero = |v: Option<usize>, d: usize| v.filter(|&n| n > 0).unwrap_or(d);

            let start_date = req.start_date.as_ref()
                .map(|s| NaiveDate::parse_from_str(s, "%Y-%m-%d"))
                .transpose()
                .map_err(|_| AppError::BadRequest("Invalid start_date format. Use YYYY-MM-DD.".to_string()))?
                .unwrap_or(defaults.start_date);

            let end_date = req.end_date.as_ref()
                .map(|s| NaiveDate::parse_from_str(s, "%Y-%m-%d"))
                .transpose()
                .map_err(|_| AppError::BadRequest("Invalid end_date format. Use YYYY-MM-DD.".to_string()))?
                .unwrap_or(defaults.end_date);

            if start_date > end_date {
                return Err(AppError::BadRequest(
                    "start_date must be on or before end_date".to_string(),
                ));
            }

            let clinic_weights = resolve_clinic_weights(req.clinic_weights)?;

            SimulationConfig {
                plans_per_company:        nonzero(req.plans_per_company,        defaults.plans_per_company),
                providers:                nonzero(req.providers,                defaults.providers),
                patients:                 nonzero(req.patients,                 defaults.patients),
                records_per_appointment:  nonzero(req.records_per_appointment,  defaults.records_per_appointment),
                start_date,
                end_date,
                clinic_weights,
            }
        }
        None => defaults,
    };

    // Atomically transition from idle → running; reject if already active
    if !state.try_start() {
        tracing::warn!("Populate rejected: a run is already in progress");
        return Err(AppError::Conflict(
            "A run is already in progress".to_string(),
        ));
    }

    let pool_clone   = pool.get_ref().clone();
    let dynamo_clone = dynamo.get_ref().clone();
    let state_clone  = state.clone();

    tokio::spawn(async move {
        match run_populate(pool_clone, dynamo_clone, config, &state_clone).await {
            Ok(_) => {
                tracing::info!("Populate completed successfully");
                // Keep "complete" visible for a few poll cycles.
                tokio::time::sleep(tokio::time::Duration::from_secs(3)).await;
            }
            Err(e) => tracing::error!("Populate failed: {}", e),
        }
        state_clone.set_populate_progress(None);
        state_clone.stop();
    });

    tracing::info!("Populate started");

    Ok(HttpResponse::Accepted().json(MessageResponse {
        message: "Population started — poll GET /simulate/status for progress".to_string(),
    }))
}

/// Request body for starting a static populate run (Step 1).
/// Only reference data counts — no date fields.
#[derive(Debug, Deserialize, utoipa::ToSchema)]
pub struct StaticPopulateRequest {
    /// Number of insurance plans per company (default: 3)
    #[schema(example = 3)]
    pub plans_per_company: Option<usize>,

    /// Number of providers to generate (default: 50)
    #[schema(example = 50)]
    pub providers: Option<usize>,

    /// Number of patients to generate (default: 50000)
    #[schema(example = 50000)]
    pub patients: Option<usize>,

    /// Per-clinic weights (10 entries). Controls distribution of patients and providers.
    /// Default: [12, 3, 14, 14, 2, 14, 14, 12, 8, 8]
    pub clinic_weights: Option<Vec<u32>>,
}

/// Request body for starting a dynamic populate run (Step 2).
/// Date range and per-appointment counts.
///
/// Appointment volume is determined by provider count × 36 slots/day,
/// distributed across clinics by clinic_weights.
#[derive(Debug, Deserialize, utoipa::ToSchema)]
pub struct DynamicPopulateRequest {
    /// Start date (inclusive), ISO 8601 YYYY-MM-DD
    #[schema(example = "2026-04-01")]
    pub start_date: String,

    /// End date (inclusive), ISO 8601 YYYY-MM-DD
    #[schema(example = "2026-06-30")]
    pub end_date: String,

    /// Medical records per appointment (default: 1)
    #[schema(example = 1)]
    pub records_per_appointment: Option<usize>,

    /// Per-clinic weights (10 entries). Controls how providers and appointments
    /// are distributed across clinics. Default: [12, 3, 14, 14, 2, 14, 14, 12, 8, 8]
    pub clinic_weights: Option<Vec<u32>>,
}

/// Seed Aurora DSQL with static reference data only (Step 1).
///
/// Generates insurance companies (7), plans, clinics (10), providers, patients,
/// emergency contacts, demographics, and patient insurance links.
///
/// Does NOT generate appointments, schedules, or any date-dependent data.
/// Run POST /populate/dynamic to generate those.
///
/// Returns 409 if static data already exists (patients > 0). Reset first.
#[utoipa::path(
    post,
    path = "/populate/static",
    tag = "Population",
    security(("bearer_auth" = [])),
    request_body(
        content = StaticPopulateRequest,
        description = "Optional static populate parameters.",
        content_type = "application/json"
    ),
    responses(
        (status = 202, description = "Static populate started", body = MessageResponse),
        (status = 409, description = "Static data already exists or a run is in progress", body = String),
        (status = 401, description = "Unauthorized", body = String),
        (status = 500, description = "Internal server error", body = String)
    )
)]
pub async fn start_populate_static(
    pool: web::Data<DbPool>,
    dynamo: web::Data<DynamoClient>,
    state: web::Data<SimulatorState>,
    body: Option<web::Json<StaticPopulateRequest>>,
) -> Result<HttpResponse, AppError> {
    // Check if static data already exists.
    let counts = state.get_counts();
    if counts.patients > 0 {
        return Err(AppError::Conflict(
            "Static data already exists. Reset Aurora data first.".to_string(),
        ));
    }

    let defaults = SimulationConfig::default();
    let config = match body {
        Some(json) => {
            let req = json.into_inner();
            let nonzero = |v: Option<usize>, d: usize| v.filter(|&n| n > 0).unwrap_or(d);
            let clinic_weights = resolve_clinic_weights(req.clinic_weights)?;
            SimulationConfig {
                plans_per_company: nonzero(req.plans_per_company, defaults.plans_per_company),
                providers:         nonzero(req.providers,         defaults.providers),
                patients:          nonzero(req.patients,          defaults.patients),
                clinic_weights,
                ..defaults
            }
        }
        None => defaults,
    };

    if !state.try_start() {
        return Err(AppError::Conflict(
            "A run is already in progress".to_string(),
        ));
    }

    let pool_clone   = pool.get_ref().clone();
    let dynamo_clone = dynamo.get_ref().clone();
    let state_clone  = state.clone();

    tokio::spawn(async move {
        match run_populate_static(pool_clone, dynamo_clone, config, &state_clone).await {
            Ok(_) => {
                tracing::info!("Static populate completed successfully");
                tokio::time::sleep(tokio::time::Duration::from_secs(3)).await;
            }
            Err(e) => tracing::error!("Static populate failed: {}", e),
        }
        state_clone.set_populate_progress(None);
        state_clone.stop();
    });

    Ok(HttpResponse::Accepted().json(MessageResponse {
        message: "Static populate started — poll GET /simulate/status for progress".to_string(),
    }))
}

/// Generate date-dependent data for a specific date range (Step 2).
///
/// Generates clinic schedules (first run only), appointments, medical records,
/// and patient visits within the specified date range.
///
/// Does NOT write to DynamoDB. Use POST /simulate for that.
///
/// Requires static data from POST /populate/static. Can be called multiple
/// times for non-overlapping date ranges (additive).
///
/// Returns 400 if no static data exists or if the date range overlaps
/// already-populated dates.
#[utoipa::path(
    post,
    path = "/populate/dynamic",
    tag = "Population",
    security(("bearer_auth" = [])),
    request_body(
        content = DynamicPopulateRequest,
        description = "Date range and optional count parameters.",
        content_type = "application/json"
    ),
    responses(
        (status = 202, description = "Dynamic populate started", body = MessageResponse),
        (status = 400, description = "Missing static data or date overlap", body = String),
        (status = 409, description = "A run is already in progress", body = String),
        (status = 401, description = "Unauthorized", body = String),
        (status = 500, description = "Internal server error", body = String)
    )
)]
pub async fn start_populate_dynamic(
    pool: web::Data<DbPool>,
    dynamo: web::Data<DynamoClient>,
    state: web::Data<SimulatorState>,
    body: web::Json<DynamicPopulateRequest>,
) -> Result<HttpResponse, AppError> {
    let req = body.into_inner();

    // Check static data exists.
    let counts = state.get_counts();
    if counts.patients == 0 {
        return Err(AppError::BadRequest(
            "No static data found. Run POST /populate/static first.".to_string(),
        ));
    }

    // Parse and validate dates.
    let start_date = NaiveDate::parse_from_str(&req.start_date, "%Y-%m-%d")
        .map_err(|_| AppError::BadRequest("Invalid start_date format. Use YYYY-MM-DD.".to_string()))?;
    let end_date = NaiveDate::parse_from_str(&req.end_date, "%Y-%m-%d")
        .map_err(|_| AppError::BadRequest("Invalid end_date format. Use YYYY-MM-DD.".to_string()))?;

    if start_date > end_date {
        return Err(AppError::BadRequest(
            "start_date must be on or before end_date".to_string(),
        ));
    }

    let range_days = (end_date - start_date).num_days();
    if range_days > 90 {
        return Err(AppError::BadRequest(
            "Date range cannot exceed 90 days".to_string(),
        ));
    }

    // Check for date overlap with already-populated dates.
    let existing_dates = get_populated_dates(pool.get_ref()).await?;
    let existing_set: HashSet<NaiveDate> = existing_dates.into_iter().collect();
    let mut overlap_dates: Vec<NaiveDate> = Vec::new();
    let mut check = start_date;
    while check <= end_date {
        if existing_set.contains(&check) {
            overlap_dates.push(check);
        }
        check += chrono::TimeDelta::days(1);
    }

    if !overlap_dates.is_empty() {
        return Err(AppError::BadRequest(format!(
            "Date range overlaps {} already-populated date(s). First conflict: {}. \
             Reset dynamic data first or choose a non-overlapping range.",
            overlap_dates.len(),
            overlap_dates[0],
        )));
    }

    if !state.try_start() {
        return Err(AppError::Conflict(
            "A run is already in progress".to_string(),
        ));
    }

    let nonzero = |v: Option<usize>, d: usize| v.filter(|&n| n > 0).unwrap_or(d);
    let records_per_appointment = nonzero(req.records_per_appointment, 1);
    let clinic_weights = resolve_clinic_weights(req.clinic_weights)?;

    let pool_clone   = pool.get_ref().clone();
    let dynamo_clone = dynamo.get_ref().clone();
    let state_clone  = state.clone();

    tokio::spawn(async move {
        match run_populate_dynamic(
            pool_clone, dynamo_clone, &state_clone,
            start_date, end_date,
            records_per_appointment,
            clinic_weights,
        ).await {
            Ok(_) => {
                tracing::info!("Dynamic populate completed successfully");
                tokio::time::sleep(tokio::time::Duration::from_secs(3)).await;
            }
            Err(e) => tracing::error!("Dynamic populate failed: {}", e),
        }
        state_clone.set_populate_progress(None);
        state_clone.stop();
    });

    Ok(HttpResponse::Accepted().json(MessageResponse {
        message: format!(
            "Dynamic populate started ({} to {}) — poll GET /simulate/status for progress",
            start_date, end_date
        ),
    }))
}

/// Return the distinct dates that have appointments in Aurora DSQL.
///
/// Used by the frontend calendar to show which dates are already populated
/// and prevent double-population.
#[utoipa::path(
    get,
    path = "/populate/dates",
    tag = "Population",
    security(("bearer_auth" = [])),
    responses(
        (status = 200, description = "List of populated dates", body = Vec<String>),
        (status = 401, description = "Unauthorized", body = String),
        (status = 500, description = "Internal server error", body = String)
    )
)]
pub async fn get_populated_dates_handler(
    pool: web::Data<DbPool>,
) -> Result<HttpResponse, AppError> {
    let dates = get_populated_dates(pool.get_ref()).await?;
    let date_strings: Vec<String> = dates.iter().map(|d| d.format("%Y-%m-%d").to_string()).collect();
    Ok(HttpResponse::Ok().json(date_strings))
}

/// Dynamic-only tables for reset (FK-safe deletion order).
const DYNAMIC_RESET_TABLES: &[(&str, &str, &str)] = &[
    ("vital_fold.patient_vitals",  "patient_visit_id",  "Patient Vitals"),
    ("vital_fold.patient_visit",   "patient_visit_id",  "Patient Visits"),
    ("vital_fold.medical_record",  "medical_record_id", "Medical Records"),
    ("vital_fold.appointment",     "appointment_id",    "Appointments"),
    ("vital_fold.clinic_schedule", "schedule_id",       "Clinic Schedules"),
];

/// Delete only dynamic data (schedules, appointments, records, visits),
/// preserving static reference data (patients, providers, clinics, insurance).
///
/// Useful for re-populating with different date ranges without regenerating
/// the 50,000+ static reference records.
#[utoipa::path(
    post,
    path = "/populate/reset-dynamic",
    tag = "Population",
    security(("bearer_auth" = [])),
    responses(
        (status = 202, description = "Dynamic reset started", body = MessageResponse),
        (status = 409, description = "Cannot reset while a run is in progress", body = String),
        (status = 401, description = "Unauthorized", body = String),
        (status = 500, description = "Internal server error", body = String)
    )
)]
pub async fn reset_dynamic_data(
    pool: web::Data<DbPool>,
    state: web::Data<SimulatorState>,
) -> Result<HttpResponse, AppError> {
    if !state.try_start() {
        return Err(AppError::Conflict(
            "Cannot reset while a run is in progress".to_string(),
        ));
    }

    let pool_clone  = pool.get_ref().clone();
    let state_clone = state.clone();

    tokio::spawn(async move {
        match run_dynamic_reset(&pool_clone, &state_clone).await {
            Ok(_)  => tracing::info!("Dynamic reset completed successfully"),
            Err(e) => tracing::error!("Dynamic reset failed: {}", e),
        }
        tokio::time::sleep(tokio::time::Duration::from_secs(3)).await;
        state_clone.set_reset_progress(None);
        state_clone.stop();
    });

    tracing::warn!("Dynamic reset started");

    Ok(HttpResponse::Accepted().json(MessageResponse {
        message: "Dynamic reset started — poll GET /simulate/status for progress".to_string(),
    }))
}

/// Background worker that deletes only dynamic Aurora DSQL rows with progress tracking.
async fn run_dynamic_reset(
    pool: &DbPool,
    state: &SimulatorState,
) -> Result<(), AppError> {
    use crate::engine_state::ResetProgress;

    const MAX_RETRIES: u32 = 5;
    const BACKOFF_BASE_MS: u64 = 500;

    let total_tables = DYNAMIC_RESET_TABLES.len();
    let mut cumulative_rows: u64 = 0;

    for (i, (table, pk, display_name)) in DYNAMIC_RESET_TABLES.iter().enumerate() {
        state.set_reset_progress(Some(ResetProgress {
            current_table: display_name.to_string(),
            tables_done: i,
            total_tables,
            rows_deleted: cumulative_rows,
            is_complete: false,
        }));

        let mut table_deleted: u64 = 0;
        loop {
            let mut attempt: u32 = 0;
            let rows = loop {
                match sqlx::query(&format!(
                    "DELETE FROM {table} WHERE {pk} IN (SELECT {pk} FROM {table} LIMIT 2500)"
                ))
                .execute(pool)
                .await
                {
                    Ok(result) => break result.rows_affected(),
                    Err(e) => {
                        let is_oc = format!("{}", e).contains("OC000");
                        attempt += 1;
                        if is_oc && attempt <= MAX_RETRIES {
                            let delay_ms = BACKOFF_BASE_MS * (1 << (attempt - 1));
                            tracing::warn!(
                                "Aurora OC000 on '{}' (attempt {}/{}), retrying in {}ms",
                                table, attempt, MAX_RETRIES, delay_ms
                            );
                            tokio::time::sleep(tokio::time::Duration::from_millis(delay_ms)).await;
                        } else {
                            state.set_reset_progress(None);
                            return Err(AppError::from(e));
                        }
                    }
                }
            };

            table_deleted += rows;
            cumulative_rows += rows;

            state.set_reset_progress(Some(ResetProgress {
                current_table: display_name.to_string(),
                tables_done: i,
                total_tables,
                rows_deleted: cumulative_rows,
                is_complete: false,
            }));

            if rows == 0 {
                break;
            }
        }
        tracing::debug!("Deleted {} rows from {}", table_deleted, table);
    }

    // Zero only the dynamic count fields.
    let mut counts = state.get_counts();
    counts.clinic_schedules = 0;
    counts.appointments = 0;
    counts.medical_records = 0;
    counts.patient_visits = 0;
    counts.patient_vitals = 0;
    state.set_counts(counts);

    state.set_reset_progress(Some(ResetProgress {
        current_table: String::new(),
        tables_done: total_tables,
        total_tables,
        rows_deleted: cumulative_rows,
        is_complete: true,
    }));

    Ok(())
}


/// Write DynamoDB records for all appointments scheduled for today.
///
/// Queries Aurora DSQL for appointments where `appointment_datetime::date = CURRENT_DATE`,
/// then writes to both DynamoDB tables (patient_visit + patient_vitals) for each.
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
        return Err(AppError::Conflict(
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



/// Request body for date-range DynamoDB sync.
/// Syncs existing Aurora patient visit data to DynamoDB for a specific date range.
/// Requires a prior Dynamic Populate run to have created visits for the target dates.
#[derive(Debug, Deserialize, utoipa::ToSchema)]
pub struct DateRangeRequest {
    /// Start date (inclusive), ISO 8601 YYYY-MM-DD
    #[schema(example = "2026-03-23")]
    pub start_date: String,

    /// End date (inclusive), ISO 8601 YYYY-MM-DD
    #[schema(example = "2026-03-23")]
    pub end_date: String,
}

/// Sync existing Aurora visit data to DynamoDB for a date range.
///
/// Reads patient_visit + patient_vitals from Aurora for the specified date range
/// and writes them to both DynamoDB tables. No Aurora data is generated.
///
/// Requires a prior Dynamic Populate run to have created visits for the target dates.
/// If no visits exist for the date range, returns 400 Bad Request.
///
/// The date range is inclusive on both ends. Maximum range is 90 days.
#[utoipa::path(
    post,
    path = "/simulate/date-range",
    tag = "Simulation",
    security(
        ("bearer_auth" = [])
    ),
    request_body(
        content = DateRangeRequest,
        description = "Date range for DynamoDB sync.",
        content_type = "application/json"
    ),
    responses(
        (status = 202, description = "DynamoDB sync started", body = MessageResponse),
        (status = 400, description = "Invalid date range or no visits found", body = String),
        (status = 401, description = "Unauthorized", body = String),
        (status = 500, description = "Internal server error", body = String)
    )
)]
pub async fn start_date_range_simulate(
    pool: web::Data<DbPool>,
    dynamo: web::Data<DynamoClient>,
    state: web::Data<SimulatorState>,
    body: web::Json<DateRangeRequest>,
) -> Result<HttpResponse, AppError> {
    // Parse and validate dates.
    let start_date = NaiveDate::parse_from_str(&body.start_date, "%Y-%m-%d")
        .map_err(|_| AppError::BadRequest("Invalid start_date format. Use YYYY-MM-DD.".to_string()))?;
    let end_date = NaiveDate::parse_from_str(&body.end_date, "%Y-%m-%d")
        .map_err(|_| AppError::BadRequest("Invalid end_date format. Use YYYY-MM-DD.".to_string()))?;

    if start_date > end_date {
        return Err(AppError::BadRequest(
            "start_date must be on or before end_date".to_string(),
        ));
    }

    let range_days = (end_date - start_date).num_days();
    if range_days > 90 {
        return Err(AppError::BadRequest(
            "Date range cannot exceed 90 days".to_string(),
        ));
    }

    // Atomically transition from idle → running.
    if !state.try_start() {
        tracing::warn!("Date-range DynamoDB sync rejected: a run is already in progress");
        return Err(AppError::Conflict(
            "A run is already in progress".to_string(),
        ));
    }

    // Pre-flight: verify that visits exist in Aurora for this date range.
    let visit_count: (i64,) = sqlx::query_as(
        "SELECT COUNT(*) FROM vital_fold.patient_visit \
         WHERE checkin_time::date >= $1 AND checkin_time::date <= $2"
    )
    .bind(start_date)
    .bind(end_date)
    .fetch_one(pool.get_ref())
    .await
    .map_err(|e| {
        state.stop();
        AppError::Internal(format!("Failed to check visit data: {}", e))
    })?;

    if visit_count.0 == 0 {
        state.stop();
        return Err(AppError::BadRequest(
            format!(
                "No patient visits found in Aurora for {} to {}. \
                 Run Dynamic Populate for this date range first.",
                body.start_date, body.end_date
            )
        ));
    }

    let pool_clone   = pool.get_ref().clone();
    let dynamo_clone = dynamo.get_ref().clone();
    let state_clone  = state.clone();

    tokio::spawn(async move {
        match run_date_range_simulate(
            pool_clone, dynamo_clone, &state_clone,
            start_date, end_date,
        ).await {
            Ok(_)  => tracing::info!("Date-range DynamoDB sync completed successfully"),
            Err(e) => tracing::error!("Date-range DynamoDB sync failed: {}", e),
        }
        // Keep the "complete" state visible for a few poll cycles, then clear.
        tokio::time::sleep(tokio::time::Duration::from_secs(3)).await;
        state_clone.set_dynamo_progress(None);
        state_clone.stop();
    });

    tracing::info!(
        "Date-range DynamoDB sync started: {} to {} ({} visits)",
        body.start_date, body.end_date, visit_count.0
    );

    Ok(HttpResponse::Accepted().json(MessageResponse {
        message: format!(
            "DynamoDB sync started ({} to {}), syncing {} visits",
            body.start_date, body.end_date, visit_count.0
        ),
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
///   dynamo_patient_visits
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

    let reset_progress    = state.get_reset_progress();
    let populate_progress = state.get_populate_progress();
    let dynamo_progress   = state.get_dynamo_progress();

    let response = SimulationStatusResponse {
        running,
        last_run,
        counts,
        reset_progress,
        populate_progress,
        dynamo_progress,
    };

    Ok(HttpResponse::Ok().json(response))
}

/// Query live record counts from Aurora DSQL and DynamoDB.
///
/// Runs `SELECT COUNT(*)` against every Aurora table and uses
/// `Scan` with `Select::Count` on both DynamoDB tables for exact counts.
/// Returns the same shape as `SimulationCounts` but with actual database values.
#[utoipa::path(
    get,
    path = "/simulate/db-counts",
    tag = "Simulation",
    security(
        ("bearer_auth" = [])
    ),
    responses(
        (status = 200, description = "Live database counts", body = crate::engine_state::SimulationCounts),
        (status = 401, description = "Unauthorized", body = String),
        (status = 500, description = "Internal server error", body = String)
    )
)]
pub async fn get_db_counts(
    pool: web::Data<DbPool>,
    dynamo: web::Data<DynamoClient>,
) -> Result<HttpResponse, AppError> {
    use crate::engine_state::SimulationCounts;

    // Single round-trip: 13 scalar sub-selects.
    let row: (i64,i64,i64,i64,i64,i64,i64,i64,i64,i64,i64,i64,i64) = sqlx::query_as(
        "SELECT \
            (SELECT COUNT(*) FROM vital_fold.insurance_company),  \
            (SELECT COUNT(*) FROM vital_fold.insurance_plan),     \
            (SELECT COUNT(*) FROM vital_fold.clinic),             \
            (SELECT COUNT(*) FROM vital_fold.provider),           \
            (SELECT COUNT(*) FROM vital_fold.patient),            \
            (SELECT COUNT(*) FROM vital_fold.emergency_contact),  \
            (SELECT COUNT(*) FROM vital_fold.patient_demographics), \
            (SELECT COUNT(*) FROM vital_fold.patient_insurance),  \
            (SELECT COUNT(*) FROM vital_fold.clinic_schedule),    \
            (SELECT COUNT(*) FROM vital_fold.appointment),        \
            (SELECT COUNT(*) FROM vital_fold.medical_record),     \
            (SELECT COUNT(*) FROM vital_fold.patient_visit),      \
            (SELECT COUNT(*) FROM vital_fold.patient_vitals)"
    )
    .fetch_one(pool.get_ref())
    .await?;

    // DynamoDB: scan with SELECT COUNT for exact item counts.
    let (dyn_visits, dyn_vitals) = tokio::join!(
        scan_table_count(&dynamo, "patient_visit"),
        scan_table_count(&dynamo, "patient_vitals"),
    );

    let counts = SimulationCounts {
        insurance_companies:   row.0  as usize,
        insurance_plans:       row.1  as usize,
        clinics:               row.2  as usize,
        providers:             row.3  as usize,
        patients:              row.4  as usize,
        emergency_contacts:    row.5  as usize,
        patient_demographics:  row.6  as usize,
        patient_insurance:     row.7  as usize,
        clinic_schedules:      row.8  as usize,
        appointments:          row.9  as usize,
        medical_records:       row.10 as usize,
        patient_visits:        row.11 as usize,
        patient_vitals:        row.12 as usize,
        dynamo_patient_visits: dyn_visits,
        dynamo_patient_vitals: dyn_vitals,
    };

    Ok(HttpResponse::Ok().json(counts))
}

/// Helper: return exact item count for a DynamoDB table via Scan SELECT COUNT.
/// `describe_table` only updates every ~6 hours, so we scan for accuracy.
async fn scan_table_count(client: &DynamoClient, table: &str) -> usize {
    let mut total: usize = 0;
    let mut exclusive_start_key = None;

    loop {
        let mut req = client
            .scan()
            .table_name(table)
            .select(aws_sdk_dynamodb::types::Select::Count);

        if let Some(key) = exclusive_start_key.take() {
            req = req.set_exclusive_start_key(Some(key));
        }

        match req.send().await {
            Ok(resp) => {
                total += resp.count().max(0) as usize;
                if let Some(key) = resp.last_evaluated_key() {
                    exclusive_start_key = Some(key.to_owned());
                } else {
                    break;
                }
            }
            Err(e) => {
                tracing::warn!("scan count({}) failed: {}", table, e);
                break;
            }
        }
    }

    total
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
/// `run_simulate` first to write patient_visit records.
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
        (status = 409, description = "A run is already in progress", body = String),
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
        return Err(AppError::Conflict(
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
        (status = 409, description = "A run is already in progress", body = String),
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
        return Err(AppError::Conflict(
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
                EXTRACT(HOUR FROM a.appointment_datetime)::INTEGER as hour \
         FROM vital_fold.appointment a \
         JOIN vital_fold.patient p ON p.patient_id = a.patient_id \
         JOIN vital_fold.clinic c ON c.clinic_id = a.clinic_id \
         WHERE a.appointment_datetime::date = CURRENT_DATE \
         ORDER BY c.clinic_name, a.appointment_datetime"
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
/// Returns 202 Accepted immediately; the deletion runs in a background task.
/// Poll `GET /simulate/status` to track progress via the `reset_progress` field.
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
        (status = 202, description = "Reset started", body = MessageResponse),
        (status = 409, description = "Cannot reset while a run is in progress", body = String),
        (status = 401, description = "Unauthorized", body = String),
        (status = 500, description = "Internal server error", body = String)
    )
)]
pub async fn reset_data(
    pool: web::Data<DbPool>,
    state: web::Data<SimulatorState>,
) -> Result<HttpResponse, AppError> {
    if !state.try_start() {
        return Err(AppError::Conflict(
            "Cannot reset while a run is in progress".to_string(),
        ));
    }

    let pool_clone  = pool.get_ref().clone();
    let state_clone = state.clone();

    tokio::spawn(async move {
        match run_aurora_reset(&pool_clone, &state_clone).await {
            Ok(_)  => tracing::info!("Aurora reset completed successfully"),
            Err(e) => tracing::error!("Aurora reset failed: {}", e),
        }
        // Keep the "complete" state visible for a few poll cycles, then clear.
        tokio::time::sleep(tokio::time::Duration::from_secs(3)).await;
        state_clone.set_reset_progress(None);
        state_clone.stop();
    });

    tracing::warn!("Aurora reset started");

    Ok(HttpResponse::Accepted().json(MessageResponse {
        message: "Aurora reset started — poll GET /simulate/status for progress".to_string(),
    }))
}

/// Table display names for the reset progress UI (matches FK-safe deletion order).
const RESET_TABLES: &[(&str, &str, &str)] = &[
    ("vital_fold.patient_vitals",       "patient_visit_id",      "Patient Vitals"),
    ("vital_fold.patient_visit",        "patient_visit_id",      "Patient Visits"),
    ("vital_fold.medical_record",       "medical_record_id",     "Medical Records"),
    ("vital_fold.appointment",          "appointment_id",        "Appointments"),
    ("vital_fold.clinic_schedule",      "schedule_id",           "Clinic Schedules"),
    ("vital_fold.patient_insurance",    "patient_insurance_id",  "Patient Insurance"),
    ("vital_fold.patient_demographics", "demographics_id",       "Demographics"),
    ("vital_fold.emergency_contact",    "emergency_contact_id",  "Emergency Contacts"),
    ("vital_fold.patient",              "patient_id",            "Patients"),
    ("vital_fold.provider",             "provider_id",           "Providers"),
    ("vital_fold.clinic",               "clinic_id",             "Clinics"),
    ("vital_fold.insurance_plan",       "insurance_plan_id",     "Insurance Plans"),
    ("vital_fold.insurance_company",    "company_id",            "Insurance Companies"),
];

/// Background worker that deletes all Aurora DSQL rows with progress tracking.
async fn run_aurora_reset(
    pool: &DbPool,
    state: &SimulatorState,
) -> Result<(), AppError> {
    use crate::engine_state::ResetProgress;

    const MAX_RETRIES: u32 = 5;
    const BACKOFF_BASE_MS: u64 = 500;

    let total_tables = RESET_TABLES.len();
    let mut cumulative_rows: u64 = 0;

    for (i, (table, pk, display_name)) in RESET_TABLES.iter().enumerate() {
        // Publish progress before starting this table.
        state.set_reset_progress(Some(ResetProgress {
            current_table: display_name.to_string(),
            tables_done: i,
            total_tables,
            rows_deleted: cumulative_rows,
            is_complete: false,
        }));

        let mut table_deleted: u64 = 0;
        loop {
            let mut attempt: u32 = 0;
            let rows = loop {
                match sqlx::query(&format!(
                    "DELETE FROM {table} WHERE {pk} IN (SELECT {pk} FROM {table} LIMIT 2500)"
                ))
                .execute(pool)
                .await
                {
                    Ok(result) => break result.rows_affected(),
                    Err(e) => {
                        let is_oc = format!("{}", e).contains("OC000");
                        attempt += 1;
                        if is_oc && attempt <= MAX_RETRIES {
                            let delay_ms = BACKOFF_BASE_MS * (1 << (attempt - 1));
                            tracing::warn!(
                                "Aurora OC000 on '{}' (attempt {}/{}), retrying in {}ms",
                                table, attempt, MAX_RETRIES, delay_ms
                            );
                            tokio::time::sleep(tokio::time::Duration::from_millis(delay_ms)).await;
                        } else {
                            // Clear progress on failure so the UI doesn't show stale state.
                            state.set_reset_progress(None);
                            return Err(AppError::from(e));
                        }
                    }
                }
            };

            table_deleted += rows;
            cumulative_rows += rows;

            // Update progress after each batch so the UI shows row counts ticking up.
            state.set_reset_progress(Some(ResetProgress {
                current_table: display_name.to_string(),
                tables_done: i,
                total_tables,
                rows_deleted: cumulative_rows,
                is_complete: false,
            }));

            if rows == 0 {
                break;
            }
        }
        tracing::debug!("Deleted {} rows from {}", table_deleted, table);
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
    counts.patient_visits = 0;
    counts.patient_vitals = 0;
    state.set_counts(counts);

    // Mark complete — UI can show the final state briefly before clearing.
    state.set_reset_progress(Some(ResetProgress {
        current_table: String::new(),
        tables_done: total_tables,
        total_tables,
        rows_deleted: cumulative_rows,
        is_complete: true,
    }));

    Ok(())
}



/// Delete all items from both DynamoDB tables.
///
/// WARNING: Destructive. Scans each table and deletes all items in batches.
/// DynamoDB has no TRUNCATE equivalent — items must be scanned and deleted one batch at a time.
///
/// Runs in a background task with progress tracking via `DynamoProgress`.
/// Returns 202 Accepted immediately. Poll `GET /simulate/status` to track progress.
#[utoipa::path(
    post,
    path = "/simulate/reset-dynamo",
    tag = "Simulation",
    security(
        ("bearer_auth" = [])
    ),
    responses(
        (status = 202, description = "DynamoDB reset started", body = MessageResponse),
        (status = 409, description = "Cannot reset while a run is in progress", body = String),
        (status = 401, description = "Unauthorized", body = String),
        (status = 500, description = "Internal server error", body = String)
    )
)]
pub async fn reset_dynamo(
    dynamo: web::Data<DynamoClient>,
    state: web::Data<SimulatorState>,
) -> Result<HttpResponse, AppError> {
    if !state.try_start() {
        return Err(AppError::Conflict(
            "Cannot reset while a run is in progress".to_string(),
        ));
    }

    let dynamo_clone = dynamo.get_ref().clone();
    let state_clone = state.clone();

    tokio::spawn(async move {
        match run_dynamo_reset(&dynamo_clone, &state_clone).await {
            Ok(_)  => tracing::info!("DynamoDB reset completed successfully"),
            Err(e) => tracing::error!("DynamoDB reset failed: {}", e),
        }
        // Keep the "complete" state visible for a few poll cycles, then clear.
        tokio::time::sleep(tokio::time::Duration::from_secs(3)).await;
        state_clone.set_dynamo_progress(None);
        state_clone.stop();
    });

    tracing::warn!("DynamoDB reset started");

    Ok(HttpResponse::Accepted().json(MessageResponse {
        message: "DynamoDB reset started — poll GET /simulate/status for progress".to_string(),
    }))
}

/// Background worker that deletes all DynamoDB items with progress tracking.
async fn run_dynamo_reset(
    dynamo: &DynamoClient,
    state: &SimulatorState,
) -> Result<(), AppError> {
    use crate::engine_state::DynamoProgress;

    const DYNAMO_TABLES: &[(&str, &str, &str, &str)] = &[
        ("patient_visit",  "patient_id", "clinic_id", "Patient Visits"),
        ("patient_vitals", "patient_id", "clinic_id", "Patient Vitals"),
    ];

    let total_tables = DYNAMO_TABLES.len();
    let mut cumulative_deleted: u64 = 0;

    for (i, (table, pk_name, sk_name, display_name)) in DYNAMO_TABLES.iter().enumerate() {
        state.set_dynamo_progress(Some(DynamoProgress {
            operation: "Resetting DynamoDB".to_string(),
            current_table: display_name.to_string(),
            tables_done: i,
            total_tables,
            items_processed: cumulative_deleted,
            total_items: 0,
            is_complete: false,
        }));

        let deleted = delete_dynamo_table_with_progress(
            dynamo, state, table, pk_name, sk_name, display_name,
            i, total_tables, cumulative_deleted,
        ).await?;

        cumulative_deleted += deleted;
        tracing::info!("Deleted {} items from DynamoDB table '{}'", deleted, table);
    }

    // Zero out the in-memory DynamoDB counts.
    let mut counts = state.get_counts();
    counts.dynamo_patient_visits = 0;
    counts.dynamo_patient_vitals = 0;
    state.set_counts(counts);

    state.set_dynamo_progress(Some(DynamoProgress {
        operation: "Resetting DynamoDB".to_string(),
        current_table: String::new(),
        tables_done: total_tables,
        total_tables,
        items_processed: cumulative_deleted,
        total_items: cumulative_deleted,
        is_complete: true,
    }));

    Ok(())
}




/// Scan a DynamoDB table and delete every item using BatchWriteItem, publishing
/// progress to `DynamoProgress` after each chunk.
///
/// ## Throttling handling
/// 1. **Inter-chunk pacing** — 50 ms sleep between chunks ≈ 500 WCU/s sustained.
/// 2. **Exponential backoff** on `ThrottlingException` — up to 5 retries with
///    doubling delays starting at 1 s.
///
/// Returns the total number of items deleted from this table.
#[allow(clippy::too_many_arguments)]
async fn delete_dynamo_table_with_progress(
    dynamo: &DynamoClient,
    state: &SimulatorState,
    table: &str,
    pk_name: &str,
    sk_name: &str,
    display_name: &str,
    table_index: usize,
    total_tables: usize,
    prior_deleted: u64,
) -> Result<u64, AppError> {
    use crate::engine_state::DynamoProgress;

    const CHUNK_DELAY_MS: u64 = 50;
    const BACKOFF_BASE_MS: u64 = 1_000;
    const MAX_RETRIES: u32 = 5;

    let mut deleted: u64 = 0;
    let mut last_key: Option<HashMap<String, aws_sdk_dynamodb::types::AttributeValue>> = None;

    loop {
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
                            attempt = 0;
                        }
                        Err(e) => {
                            let is_throttle = format!("{:?}", e).contains("ThrottlingException");
                            attempt += 1;
                            if is_throttle && attempt <= MAX_RETRIES {
                                let delay_ms = BACKOFF_BASE_MS * (1 << (attempt - 1));
                                tracing::warn!(
                                    "DynamoDB throttled on '{}' (attempt {}/{}), backing off {}ms",
                                    table, attempt, MAX_RETRIES, delay_ms
                                );
                                tokio::time::sleep(tokio::time::Duration::from_millis(delay_ms)).await;
                                pending = Some(batch);
                            } else {
                                state.set_dynamo_progress(None);
                                return Err(AppError::Internal(format!(
                                    "DynamoDB batch_write_item failed on '{}' after {} attempt(s): {:?}",
                                    table, attempt, e
                                )));
                            }
                        }
                    }
                }

                deleted += chunk_len;

                // Publish progress after each chunk.
                state.set_dynamo_progress(Some(DynamoProgress {
                    operation: "Resetting DynamoDB".to_string(),
                    current_table: display_name.to_string(),
                    tables_done: table_index,
                    total_tables,
                    items_processed: prior_deleted + deleted,
                    total_items: 0,
                    is_complete: false,
                }));

                tokio::time::sleep(tokio::time::Duration::from_millis(CHUNK_DELAY_MS)).await;
            }
        }

        if last_key.is_none() {
            break;
        }
    }

    Ok(deleted)
}

// =============================================================================
// Database Initialization
// =============================================================================

/// Initialize (or re-initialize) the database schema.
///
/// Reads `migrations/init.sql` and executes it against Aurora DSQL.
/// This drops and recreates the `vital_fold` schema — all simulation data is lost.
/// The `public.users` table is created with IF NOT EXISTS (safe for re-runs).
///
/// **Destructive** — requires confirmation from the admin dashboard.
#[utoipa::path(
    post,
    path = "/admin/init-db",
    tag = "Admin",
    security(("bearer_auth" = [])),
    responses(
        (status = 200, description = "Schema initialized successfully", body = MessageResponse),
        (status = 401, description = "Unauthorized", body = String),
        (status = 500, description = "SQL execution failed", body = String)
    )
)]
pub async fn init_database(
    pool: web::Data<DbPool>,
    state: web::Data<SimulatorState>,
) -> Result<HttpResponse, AppError> {
    // Read the SQL file at compile time so it's embedded in the binary.
    let sql = include_str!("../../migrations/init.sql");

    tracing::info!("Executing database initialization (migrations/init.sql)");

    // Split on semicolons and execute each non-empty statement.
    // Strip SQL comments (lines starting with --) before checking if a statement is empty.
    let mut executed = 0usize;
    for statement in sql.split(';') {
        let cleaned: String = statement.lines()
            .filter(|line| !line.trim_start().starts_with("--"))
            .collect::<Vec<_>>()
            .join("\n");
        let trimmed = cleaned.trim();
        if trimmed.is_empty() {
            continue;
        }
        sqlx::query(trimmed)
            .execute(pool.get_ref())
            .await
            .map_err(|e| {
                tracing::error!("Init DB failed on statement: {}", &trimmed[..trimmed.len().min(120)]);
                AppError::Internal(format!("Schema init failed: {}", e))
            })?;
        executed += 1;
    }

    // Reset in-memory state since all data was dropped.
    state.set_counts(crate::engine_state::SimulationCounts::default());
    state.set_last_run(chrono::Utc::now());

    tracing::info!("Database initialized successfully ({} statements executed)", executed);

    Ok(HttpResponse::Ok().json(MessageResponse {
        message: format!("Schema initialized — {} SQL statements executed", executed),
    }))
}
