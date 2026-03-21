/// Data generator modules for the simulation engine.
///
/// # Two-Phase Data Lifecycle
///
/// ## Phase 1 — POST /populate → run_populate()
/// Seeds all Aurora DSQL tables with synthetic healthcare data:
/// insurance companies, plans, clinics, providers, patients, emergency contacts,
/// demographics, insurance links, clinic schedules, appointments, and medical records.
/// Appointments are generated 1–89 days in the future. No DynamoDB writes.
///
/// ## Phase 2 — POST /simulate → run_simulate()
/// Called on the day of an appointment. Queries Aurora for all appointments where
/// appointment_date = today, then writes patient_visit and patient_vitals records
/// to DynamoDB for each one. Models the real-world scenario where vitals and
/// check-in data are recorded on the day of the visit.

pub mod insurance;
pub mod clinic;
pub mod provider;
pub mod patient;
pub mod appointment;
pub mod medical_record;

use crate::db::DbPool;
use crate::engine_state::{ClinicActivity, SimulationCounts, SimulatorState, TimelapseState};
use crate::errors::AppError;
use chrono::{NaiveDate, NaiveDateTime, TimeDelta, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;
use aws_sdk_dynamodb::Client as DynamoClient;

/// Configuration for a populate run.
/// Controls how many of each entity to generate.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SimulationConfig {
    /// Number of insurance plans per company
    pub plans_per_company: usize,

    /// Number of providers to generate
    pub providers: usize,

    /// Number of patients to generate
    pub patients: usize,

    /// Number of appointments per patient (all dated 1–89 days in the future)
    pub appointments_per_patient: usize,

    /// Number of medical records per appointment
    pub records_per_appointment: usize,
}

impl Default for SimulationConfig {
    fn default() -> Self {
        SimulationConfig {
            plans_per_company: 3,
            providers: 50,
            patients: 50000,
            appointments_per_patient: 2,
            records_per_appointment: 1,
        }
    }
}

/// Context passed through all generators containing shared data and pool references.
pub struct SimulationContext {
    /// Database connection pool
    pub pool: DbPool,

    /// DynamoDB client (held here so generators can access it if needed)
    pub dynamo_client: DynamoClient,

    /// Populate configuration
    pub config: SimulationConfig,

    /// Counts accumulated during this run
    pub counts: SimulationCounts,

    /// Insurance company IDs (populated during insurance generation)
    pub company_ids: Vec<Uuid>,

    /// Insurance plan IDs (populated during plan generation)
    pub plan_ids: Vec<Uuid>,

    /// Clinic IDs (populated during clinic generation)
    pub clinic_ids: Vec<Uuid>,

    /// Provider IDs (populated during provider generation)
    pub provider_ids: Vec<Uuid>,

    /// Patient IDs (populated during patient generation)
    pub patient_ids: Vec<Uuid>,

    /// Patient data needed for demographics generation: (patient_id, first_name, last_name, dob)
    /// Populated during patient generation, consumed during demographics generation.
    pub patient_data: Vec<(Uuid, String, String, NaiveDate)>,
}

impl SimulationContext {
    /// Create a new SimulationContext.
    pub fn new(pool: DbPool, dynamo_client: DynamoClient, config: SimulationConfig) -> Self {
        SimulationContext {
            pool,
            dynamo_client,
            config,
            counts: SimulationCounts::default(),
            company_ids: Vec::new(),
            plan_ids: Vec::new(),
            clinic_ids: Vec::new(),
            provider_ids: Vec::new(),
            patient_ids: Vec::new(),
            patient_data: Vec::new(),
        }
    }
}

/// Seed all Aurora DSQL tables with synthetic healthcare data.
///
/// This is Phase 1 of the data lifecycle (POST /populate). It generates all
/// relational data but does NOT write to DynamoDB. DynamoDB writes happen on
/// the day of each appointment via `run_simulate`.
///
/// # Execution Order
/// 1. Insurance companies (7 fixed carriers)
/// 2. Insurance plans (plans_per_company per carrier)
/// 3. Clinics (10 fixed SE US locations)
/// 4. Providers (N random cardiac specialists)
/// 5. Patients (N random with emergency contacts inline)
/// 6. Emergency contacts (no-op: done inside generate_patients)
/// 7. Patient demographics (1 per patient)
/// 8. Patient insurance links (random plan per patient)
/// 9. Clinic schedules (provider-clinic-day combinations)
/// 10. Appointments (N per patient, dated 1–89 days in the future)
/// 11. Medical records (N per appointment)
pub async fn run_populate(
    pool: DbPool,
    dynamo_client: DynamoClient,
    config: SimulationConfig,
    state: &SimulatorState,
) -> Result<(), AppError> {
    let mut ctx = SimulationContext::new(pool, dynamo_client, config);

    tracing::info!("Starting populate run");
    let start = Utc::now();

    // Step 1: Generate insurance companies
    tracing::debug!("Generating insurance companies");
    insurance::generate_insurance_companies(&mut ctx).await?;

    // Step 2: Generate insurance plans
    tracing::debug!("Generating insurance plans");
    insurance::generate_insurance_plans(&mut ctx).await?;

    // Step 3: Generate clinics
    tracing::debug!("Generating clinics");
    clinic::generate_clinics(&mut ctx).await?;

    // Step 4: Generate providers
    tracing::debug!("Generating providers");
    provider::generate_providers(&mut ctx).await?;

    // Step 5: Generate patients (also inserts emergency contacts inline)
    tracing::debug!("Generating patients");
    patient::generate_patients(&mut ctx).await?;

    // Step 6: Emergency contacts — handled inside generate_patients; no-op here
    tracing::debug!("Generating emergency contacts");
    patient::generate_emergency_contacts(&mut ctx).await?;

    // Step 7: Generate patient demographics (1 per patient)
    tracing::debug!("Generating patient demographics");
    patient::generate_patient_demographics(&mut ctx).await?;

    // Step 8: Generate patient insurance links
    tracing::debug!("Generating patient insurance links");
    patient::generate_patient_insurance(&mut ctx).await?;

    // Step 9: Generate clinic schedules
    tracing::debug!("Generating clinic schedules");
    clinic::generate_clinic_schedules(&mut ctx).await?;

    // Step 10: Generate appointments (Aurora DSQL only — no DynamoDB)
    tracing::debug!("Generating appointments");
    appointment::generate_appointments(&mut ctx).await?;

    // Step 11: Generate medical records
    tracing::debug!("Generating medical records");
    medical_record::generate_medical_records(&mut ctx).await?;

    let duration = Utc::now().signed_duration_since(start);
    tracing::info!(
        "Populate complete in {:.2}s — {} patients, {} appointments, {} medical records",
        duration.num_milliseconds() as f64 / 1000.0,
        ctx.counts.patients,
        ctx.counts.appointments,
        ctx.counts.medical_records,
    );

    // Update global state with final counts and timestamp
    state.set_last_run(Utc::now());
    state.set_counts(ctx.counts);

    Ok(())
}


/// Row returned when querying today's appointments for DynamoDB simulation.
#[derive(sqlx::FromRow)]
struct TodayAppointment {
    appointment_id:   Uuid,
    patient_id:       Uuid,
    clinic_id:        Uuid,
    provider_id:      Uuid,
    appointment_date: NaiveDateTime,
}

/// Write DynamoDB records for all appointments scheduled for today.
///
/// This is Phase 2 of the data lifecycle (POST /simulate). It models what happens
/// on the day of an appointment: the clinic records check-in times, vitals, and
/// visit data into DynamoDB. Run this once per day to simulate real-time data capture.
///
/// # DynamoDB Rate Limit
/// DynamoDB allows 30,000 writes/sec per table. Each appointment triggers 2 writes
/// (patient_visit + patient_vitals), so the effective throughput cap is 15,000
/// appointments/sec.
///
/// The semaphore caps concurrency at DYNAMO_CONCURRENCY = 128 in-flight requests.
/// At ~5ms average DynamoDB round-trip: 128 / 0.005 ≈ 25,600 writes/sec per table —
/// safely below the 30,000/sec limit with headroom for latency variance.
///
/// If today's appointment count exceeds ~75,000, the semaphore naturally throttles
/// the write rate via backpressure. No additional rate limiting is needed.
///
/// # Appointment Date Note
/// Populate generates appointments 1–89 days in the future. This function queries
/// for `appointment_date::date = CURRENT_DATE`. In normal operation, call it once
/// per calendar day to process that day's scheduled visits.
pub async fn run_simulate(
    pool: DbPool,
    dynamo_client: DynamoClient,
    state: &SimulatorState,
) -> Result<(), AppError> {
    tracing::info!("Starting simulate run — querying today's appointments");
    let start = Utc::now();

    // Fetch all appointments scheduled for today.
    // appointment_date is stored as TIMESTAMP; cast to DATE for date-only comparison.
    let rows: Vec<TodayAppointment> = sqlx::query_as(
        "SELECT appointment_id, patient_id, clinic_id, provider_id, appointment_date \
         FROM vital_fold.appointment \
         WHERE appointment_date::date = CURRENT_DATE"
    )
    .fetch_all(&pool)
    .await?;

    let total = rows.len();

    if total == 0 {
        tracing::warn!(
            "Simulate run found 0 appointments for today. \
             Run POST /populate first, then call POST /simulate on a day when appointments exist."
        );
        state.set_last_run(Utc::now());
        return Ok(());
    }

    tracing::info!("Found {} appointments for today — writing to DynamoDB", total);

    // Diagnostic: confirm the DynamoDB tables are reachable and log the region.
    // This runs synchronously before any writes so errors are immediately visible.
    for table_name in &["patient_visit", "patient_vitals"] {
        match dynamo_client.describe_table().table_name(*table_name).send().await {
            Ok(resp) => {
                let status = resp.table()
                    .and_then(|t| t.table_status())
                    .map(|s| s.as_str())
                    .unwrap_or("unknown");
                tracing::info!("DynamoDB table '{}' is reachable, status={}", table_name, status);
            }
            Err(e) => {
                tracing::error!("DynamoDB table '{}' NOT reachable: {:?}", table_name, e);
            }
        }
    }

    // Bounded concurrency: cap simultaneous DynamoDB requests to stay well under
    // the 30,000 writes/sec per-table limit (see function doc for rate math).
    const DYNAMO_CONCURRENCY: usize = 128;
    let sem = std::sync::Arc::new(tokio::sync::Semaphore::new(DYNAMO_CONCURRENCY));
    let mut handles: Vec<tokio::task::JoinHandle<bool>> = Vec::with_capacity(total * 2);

    for row in rows {
        // patient_visit write
        let permit     = sem.clone().acquire_owned().await.unwrap();
        let client     = dynamo_client.clone();
        let pt_id      = row.patient_id;
        let cl_id      = row.clinic_id;
        let appt_id    = row.appointment_id;
        let pr_id      = row.provider_id;
        let appt_dt    = row.appointment_date;
        handles.push(tokio::spawn(async move {
            appointment::write_patient_visit(&client, pt_id, cl_id, appt_id, pr_id, appt_dt).await;
            drop(permit);
            true // signals this was a visit write
        }));

        // patient_vitals write
        let permit     = sem.clone().acquire_owned().await.unwrap();
        let client     = dynamo_client.clone();
        let pt_id      = row.patient_id;
        let cl_id      = row.clinic_id;
        let visit_id   = row.appointment_id;
        let pr_id      = row.provider_id;
        handles.push(tokio::spawn(async move {
            appointment::write_patient_vitals(&client, pt_id, cl_id, visit_id, pr_id).await;
            drop(permit);
            false // signals this was a vitals write
        }));
    }

    // Drain all handles and tally completed writes.
    // Task panics are ignored — the write functions never panic.
    let mut visits_written  = 0usize;
    let mut vitals_written  = 0usize;
    for h in handles {
        if let Ok(is_visit) = h.await {
            if is_visit { visits_written  += 1; }
            else        { vitals_written  += 1; }
        }
    }

    let duration = Utc::now().signed_duration_since(start);
    tracing::info!(
        "Simulate complete in {:.2}s — {} patient_visit + {} patient_vitals written to DynamoDB",
        duration.num_milliseconds() as f64 / 1000.0,
        visits_written,
        vitals_written,
    );

    // Merge DynamoDB counts into whatever counts are already stored in state
    // (populate counts remain; only the dynamo fields are updated).
    let mut counts = state.get_counts();
    counts.dynamo_patient_visits = visits_written;
    counts.dynamo_patient_vitals = vitals_written;
    state.set_last_run(Utc::now());
    state.set_counts(counts);

    Ok(())
}

/// Row returned when querying clinic metadata for the timelapse.
#[derive(sqlx::FromRow)]
struct ClinicMeta {
    clinic_id: Uuid,
    city: String,
    state: String,
}

/// Row returned when counting appointments per clinic for a given date+hour.
#[derive(sqlx::FromRow)]
struct ClinicCount {
    clinic_id: Uuid,
    cnt: i64,
}

/// Run a timelapse visualization across multiple days.
///
/// Queries Aurora for appointment counts per clinic per hour-window, updating
/// `TimelapseState` so the frontend heatmap can poll and render activity.
/// Does NOT write to DynamoDB — this is a read-only visualization.
///
/// Each simulated day is subdivided into 8 hour-windows (9am–5pm). The real-time
/// interval between windows is `day_interval_secs / 8`.
pub async fn run_timelapse(
    pool: DbPool,
    state: &SimulatorState,
    total_days: usize,
    day_interval_secs: u64,
) -> Result<(), AppError> {
    tracing::info!("Starting timelapse — {} days, {}s per day", total_days, day_interval_secs);

    // 1. Fetch clinic metadata for display labels.
    let clinics: Vec<ClinicMeta> = sqlx::query_as(
        "SELECT clinic_id, city, state FROM vital_fold.clinic"
    )
    .fetch_all(&pool)
    .await?;

    let clinic_map: HashMap<Uuid, (String, String)> = clinics
        .iter()
        .map(|c| (c.clinic_id, (c.city.clone(), c.state.clone())))
        .collect();

    if clinic_map.is_empty() {
        tracing::warn!("Timelapse: no clinics found. Run POST /populate first.");
        return Ok(());
    }

    // 2. Find the date range of existing appointments.
    let range: Option<(NaiveDate, NaiveDate)> = sqlx::query_as(
        "SELECT MIN(appointment_date::date), MAX(appointment_date::date) \
         FROM vital_fold.appointment"
    )
    .fetch_optional(&pool)
    .await?;

    let (min_date, max_date) = match range {
        Some((min, max)) => (min, max),
        None => {
            tracing::warn!("Timelapse: no appointments found. Run POST /populate first.");
            return Ok(());
        }
    };

    tracing::info!("Timelapse date range: {} to {}", min_date, max_date);

    let window_sleep = tokio::time::Duration::from_secs(day_interval_secs / 8);
    let mut current_date = min_date;
    let mut day_number: usize = 0;
    let actual_total = total_days.min(
        (max_date - min_date).num_days() as usize + 1
    );

    // 3. Day loop.
    while current_date <= max_date && day_number < total_days {
        if !state.is_running() {
            tracing::info!("Timelapse stopped by user at day {}", day_number);
            break;
        }

        day_number += 1;

        // 4. Hour-window loop (9..17 = 8 windows).
        for hour in 9u32..17u32 {
            if !state.is_running() {
                break;
            }

            // Count appointments per clinic for this date+hour.
            let counts: Vec<ClinicCount> = sqlx::query_as(
                "SELECT clinic_id, COUNT(*) as cnt \
                 FROM vital_fold.appointment \
                 WHERE appointment_date::date = $1 \
                   AND EXTRACT(HOUR FROM appointment_date) = $2 \
                 GROUP BY clinic_id"
            )
            .bind(current_date)
            .bind(hour as i32)
            .fetch_all(&pool)
            .await?;

            let count_map: HashMap<Uuid, i64> = counts
                .into_iter()
                .map(|c| (c.clinic_id, c.cnt))
                .collect();

            // Build ClinicActivity for every clinic (0 if no appointments this window).
            let clinic_activity: Vec<ClinicActivity> = clinics
                .iter()
                .map(|c| {
                    let active = count_map.get(&c.clinic_id).copied().unwrap_or(0) as usize;
                    ClinicActivity {
                        clinic_id: c.clinic_id.to_string(),
                        city: c.city.clone(),
                        state: c.state.clone(),
                        active_patients: active,
                    }
                })
                .collect();

            state.set_timelapse(Some(TimelapseState {
                simulation_day: current_date.format("%Y-%m-%d").to_string(),
                day_number,
                total_days: actual_total,
                sim_hour: hour,
                clinics: clinic_activity,
                is_complete: false,
            }));

            tokio::time::sleep(window_sleep).await;
        }

        current_date += TimeDelta::days(1);
    }

    // 5. Mark complete.
    if let Some(mut final_state) = state.get_timelapse() {
        final_state.is_complete = true;
        state.set_timelapse(Some(final_state));
    }

    tracing::info!("Timelapse complete — processed {} days", day_number);

    Ok(())
}

/// Animate hour-by-hour appointment counts for a single day.
///
/// Shared by `run_today_heatmap` and `run_heatmap_replay`. Queries Aurora only —
/// no DynamoDB interaction. Returns early if the user stops the run.
async fn animate_single_day(
    pool: &DbPool,
    state: &SimulatorState,
    date: NaiveDate,
    clinics: &[ClinicMeta],
    window_interval_secs: u64,
) -> Result<(), AppError> {
    let window_sleep = tokio::time::Duration::from_secs(window_interval_secs);

    for hour in 9u32..17u32 {
        if !state.is_running() {
            tracing::info!("Heatmap stopped by user at hour {}", hour);
            break;
        }

        let counts: Vec<ClinicCount> = sqlx::query_as(
            "SELECT clinic_id, COUNT(*) as cnt \
             FROM vital_fold.appointment \
             WHERE appointment_date::date = $1 \
               AND EXTRACT(HOUR FROM appointment_date) = $2 \
             GROUP BY clinic_id"
        )
        .bind(date)
        .bind(hour as i32)
        .fetch_all(pool)
        .await?;

        let count_map: HashMap<Uuid, i64> = counts
            .into_iter()
            .map(|c| (c.clinic_id, c.cnt))
            .collect();

        let clinic_activity: Vec<ClinicActivity> = clinics
            .iter()
            .map(|c| {
                let active = count_map.get(&c.clinic_id).copied().unwrap_or(0) as usize;
                ClinicActivity {
                    clinic_id: c.clinic_id.to_string(),
                    city: c.city.clone(),
                    state: c.state.clone(),
                    active_patients: active,
                }
            })
            .collect();

        state.set_timelapse(Some(TimelapseState {
            simulation_day: date.format("%Y-%m-%d").to_string(),
            day_number: 1,
            total_days: 1,
            sim_hour: hour,
            clinics: clinic_activity,
            is_complete: false,
        }));

        tokio::time::sleep(window_sleep).await;
    }

    if let Some(mut final_state) = state.get_timelapse() {
        final_state.is_complete = true;
        state.set_timelapse(Some(final_state));
    }

    Ok(())
}

/// Run a single-day heatmap for today's appointments.
///
/// If DynamoDB hasn't been populated yet (dynamo_patient_visits == 0), auto-triggers
/// `run_simulate` first to write patient_visit + patient_vitals records, then animates
/// hour-by-hour (9am–5pm) appointment counts per clinic.
pub async fn run_today_heatmap(
    pool: DbPool,
    dynamo_client: DynamoClient,
    state: &SimulatorState,
    window_interval_secs: u64,
) -> Result<(), AppError> {
    if state.get_counts().dynamo_patient_visits == 0 {
        tracing::info!("Heatmap: DynamoDB not populated — running simulate first");
        run_simulate(pool.clone(), dynamo_client, state).await?;
        if !state.is_running() {
            return Ok(());
        }
    }

    let clinics: Vec<ClinicMeta> = sqlx::query_as(
        "SELECT clinic_id, city, state FROM vital_fold.clinic"
    )
    .fetch_all(&pool)
    .await?;

    if clinics.is_empty() {
        tracing::warn!("Heatmap: no clinics found. Run POST /populate first.");
        return Ok(());
    }

    let today = Utc::now().date_naive();
    tracing::info!("Heatmap: animating {} with {}s per window", today, window_interval_secs);
    animate_single_day(&pool, state, today, &clinics, window_interval_secs).await?;
    tracing::info!("Heatmap complete for {}", today);

    Ok(())
}

/// Replay heatmap animation using existing Aurora appointment data (read-only).
///
/// Unlike `run_today_heatmap`, this does **not** auto-populate DynamoDB.
/// It queries only Aurora for appointment counts per clinic per hour,
/// making it safe for non-admin users.
pub async fn run_heatmap_replay(
    pool: DbPool,
    state: &SimulatorState,
    window_interval_secs: u64,
) -> Result<(), AppError> {
    let clinics: Vec<ClinicMeta> = sqlx::query_as(
        "SELECT clinic_id, city, state FROM vital_fold.clinic"
    )
    .fetch_all(&pool)
    .await?;

    if clinics.is_empty() {
        tracing::warn!("Replay: no clinics found. Run POST /populate first.");
        return Ok(());
    }

    let today = Utc::now().date_naive();
    tracing::info!("Replay: animating {} with {}s per window", today, window_interval_secs);
    animate_single_day(&pool, state, today, &clinics, window_interval_secs).await?;
    tracing::info!("Replay complete for {}", today);

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simulation_config_default() {
        let config = SimulationConfig::default();
        assert!(config.providers > 0);
        assert!(config.patients > 0);
    }
}
