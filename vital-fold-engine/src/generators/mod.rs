/// Data generator modules for the simulation engine.
///
/// # Two-Phase Data Lifecycle
///
/// ## Phase 1 — POST /populate → run_populate()
/// Seeds all Aurora DSQL tables with synthetic healthcare data:
/// insurance companies, plans, clinics, providers, patients, emergency contacts,
/// demographics, insurance links, clinic schedules, appointments, and medical records.
/// Appointments are distributed across a configurable date range. No DynamoDB writes.
///
/// ## Phase 2 — POST /simulate → run_simulate()
/// Called on the day of an appointment. JOINs patient_visit + patient_vitals from
/// Aurora where checkin_time matches today, then writes to both DynamoDB tables
/// (patient_visit and patient_vitals) for each. Models the real-world scenario
/// where vitals and check-in data are recorded on the day of the visit.

pub mod insurance;
pub mod clinic;
pub mod provider;
pub mod patient;
pub mod appointment;
pub mod medical_record;
pub mod visit;
pub mod survey;
pub mod rvu;

use crate::db::DbPool;
use crate::engine_state::{ClinicActivity, PopulateProgress, SimulationCounts, SimulatorState, TimelapseState};
use crate::errors::AppError;
use chrono::{NaiveDate, NaiveDateTime, TimeDelta, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;
use aws_sdk_dynamodb::Client as DynamoClient;

/// Number of clinics in the fixed distribution.
pub const NUM_CLINICS: usize = 10;

/// Default per-clinic weights based on approximate metro population.
/// Index order matches CLINIC_DISTRIBUTION in clinic.rs:
///   0: Charlotte, 1: Asheville, 2-3: Atlanta, 4: Tallahassee,
///   5-6: Miami, 7: Orlando, 8-9: Jacksonville
pub const DEFAULT_CLINIC_WEIGHTS: [u32; NUM_CLINICS] = [
    12,  // Charlotte, NC
     3,  // Asheville, NC
    14,  // Atlanta, GA (clinic 1)
    14,  // Atlanta, GA (clinic 2)
     2,  // Tallahassee, FL
    14,  // Miami, FL (clinic 1)
    14,  // Miami, FL (clinic 2)
    12,  // Orlando, FL
     8,  // Jacksonville, FL (clinic 1)
     8,  // Jacksonville, FL (clinic 2)
];

/// Time slots per provider per day: 9 hours (8 AM–4:45 PM) × 4 quarter-hour windows.
pub const SLOTS_PER_PROVIDER: usize = 36;

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

    /// Number of medical records per appointment
    pub records_per_appointment: usize,

    /// Start date for appointment generation (inclusive)
    pub start_date: NaiveDate,

    /// End date for appointment generation (inclusive)
    pub end_date: NaiveDate,

    /// Per-clinic weights (10 entries). Controls distribution of patients,
    /// providers, and appointments across clinics. Higher weight = more volume.
    /// Defaults to DEFAULT_CLINIC_WEIGHTS if not provided.
    pub clinic_weights: Vec<u32>,
}

impl Default for SimulationConfig {
    fn default() -> Self {
        let tomorrow = Utc::now().date_naive() + TimeDelta::days(1);
        SimulationConfig {
            plans_per_company: 3,
            providers: 50,
            patients: 50000,
            records_per_appointment: 1,
            start_date: tomorrow,
            end_date: tomorrow + TimeDelta::days(89),
            clinic_weights: DEFAULT_CLINIC_WEIGHTS.to_vec(),
        }
    }
}

/// Sum all Aurora row counts for populate progress tracking.
fn count_aurora_rows(c: &SimulationCounts) -> u64 {
    (c.insurance_companies + c.insurance_plans + c.clinics + c.providers
        + c.patients + c.emergency_contacts + c.patient_demographics
        + c.patient_insurance + c.clinic_schedules + c.appointments
        + c.medical_records + c.patient_visits + c.patient_vitals) as u64
}

/// Sum Aurora row counts for static (reference) data only.
fn count_static_rows(c: &SimulationCounts) -> u64 {
    (c.insurance_companies + c.insurance_plans + c.clinics + c.providers
        + c.patients + c.emergency_contacts + c.patient_demographics
        + c.patient_insurance) as u64
}

/// Sum Aurora row counts for dynamic (date-dependent) data only.
fn count_dynamic_rows(c: &SimulationCounts) -> u64 {
    (c.clinic_schedules + c.appointments + c.medical_records
        + c.patient_visits + c.patient_vitals + c.surveys
        + c.appointment_cpt) as u64
}

/// Context passed through all generators containing shared data and pool references.
pub struct SimulationContext {
    /// Database connection pool
    pub pool: DbPool,

    /// DynamoDB client (held here so generators can access it if needed)
    #[allow(dead_code)]
    pub dynamo_client: DynamoClient,

    /// Populate configuration
    pub config: SimulationConfig,

    /// Counts accumulated during this run
    pub counts: SimulationCounts,

    /// Insurance company IDs (populated during insurance generation)
    pub company_ids: Vec<Uuid>,

    /// Insurance plan IDs (populated during plan generation)
    pub plan_ids: Vec<Uuid>,

    /// Clinic IDs (populated during clinic generation) — BIGINT identity
    pub clinic_ids: Vec<i64>,

    /// Provider IDs (populated during provider generation) — BIGINT identity
    pub provider_ids: Vec<i64>,

    /// Patient IDs (populated during patient generation)
    pub patient_ids: Vec<Uuid>,

    /// Patient data needed for demographics generation: (patient_id, first_name, last_name, dob)
    /// Populated during patient generation, consumed during demographics generation.
    pub patient_data: Vec<(Uuid, String, String, NaiveDate)>,

    /// Maps each patient (by index into patient_ids) to their home clinic index
    /// into clinic_ids. Used by appointment generation to bias clinic assignment
    /// toward the patient's geographic area.
    pub patient_home_clinics: Vec<usize>,

    /// Maps each provider (by index into provider_ids) to their primary clinic index.
    /// Populated during provider generation to distribute providers proportionally.
    pub provider_clinic_assignments: Vec<usize>,
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
            patient_home_clinics: Vec::new(),
            provider_clinic_assignments: Vec::new(),
        }
    }
}

/// Total number of steps in the populate pipeline.
const POPULATE_TOTAL_STEPS: usize = 13;

/// Populate step display names (index matches step-1).
const POPULATE_STEP_NAMES: [&str; POPULATE_TOTAL_STEPS] = [
    "Insurance Companies",
    "Insurance Plans",
    "Clinics",
    "Providers",
    "Patients",
    "Emergency Contacts",
    "Patient Demographics",
    "Patient Insurance",
    "Clinic Schedules",
    "Appointments",
    "Medical Records",
    "Patient Visits",
    "Patient Vitals",
];

/// Total number of steps in the static populate pipeline (steps 1-8).
const STATIC_TOTAL_STEPS: usize = 8;

/// Static populate step display names.
const STATIC_STEP_NAMES: [&str; STATIC_TOTAL_STEPS] = [
    "Insurance Companies",
    "Insurance Plans",
    "Clinics",
    "Providers",
    "Patients",
    "Emergency Contacts",
    "Patient Demographics",
    "Patient Insurance",
];

/// Total number of steps in the dynamic populate pipeline.
const DYNAMIC_TOTAL_STEPS: usize = 7;

/// Dynamic populate step display names.
const DYNAMIC_STEP_NAMES: [&str; DYNAMIC_TOTAL_STEPS] = [
    "Clinic Schedules",
    "Appointments",
    "Medical Records",
    "Patient Visits",
    "Patient Vitals",
    "Surveys",
    "Billing (CPT / RVU)",
];

/// Publish populate progress to the global state.
fn set_populate_step(state: &SimulatorState, step: usize, counts: &SimulationCounts) {
    state.set_populate_progress(Some(PopulateProgress {
        current_step: POPULATE_STEP_NAMES[step].to_string(),
        steps_done: step,
        total_steps: POPULATE_TOTAL_STEPS,
        rows_written: count_aurora_rows(counts),
        is_complete: false,
    }));
}

/// Publish populate progress with custom step names and totals.
fn publish_progress(
    state: &SimulatorState,
    step: usize,
    step_names: &[&str],
    total_steps: usize,
    rows_written: u64,
) {
    state.set_populate_progress(Some(PopulateProgress {
        current_step: step_names[step].to_string(),
        steps_done: step,
        total_steps,
        rows_written,
        is_complete: false,
    }));
}

/// Seed all Aurora DSQL tables with synthetic healthcare data.
///
/// This is Phase 1 of the data lifecycle (POST /populate). It generates all
/// relational data but does NOT write to DynamoDB. DynamoDB writes happen on
/// the day of each appointment via `run_simulate`.
///
/// Progress is published to `SimulatorState::populate_progress` at each step
/// so the UI can render a live progress bar.
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
/// 10. Appointments (N per patient, within configured date range)
/// 11. Medical records (N per appointment)
/// 12. Patient visits (one per appointment)
/// 13. Patient vitals (one per visit)
pub async fn run_populate(
    pool: DbPool,
    dynamo_client: DynamoClient,
    config: SimulationConfig,
    state: &SimulatorState,
) -> Result<(), AppError> {
    let mut ctx = SimulationContext::new(pool, dynamo_client, config);

    tracing::info!(
        "Starting populate run (appointments {} to {})",
        ctx.config.start_date, ctx.config.end_date
    );
    let start = Utc::now();

    // Step 1: Generate insurance companies
    set_populate_step(state, 0, &ctx.counts);
    insurance::generate_insurance_companies(&mut ctx).await?;

    // Step 2: Generate insurance plans
    set_populate_step(state, 1, &ctx.counts);
    insurance::generate_insurance_plans(&mut ctx).await?;

    // Step 3: Generate clinics
    set_populate_step(state, 2, &ctx.counts);
    clinic::generate_clinics(&mut ctx).await?;

    // Step 4: Generate providers
    set_populate_step(state, 3, &ctx.counts);
    provider::generate_providers(&mut ctx).await?;

    // Step 5: Generate patients (also inserts emergency contacts inline)
    set_populate_step(state, 4, &ctx.counts);
    patient::generate_patients(&mut ctx).await?;

    // Step 6: Emergency contacts — handled inside generate_patients; no-op here
    set_populate_step(state, 5, &ctx.counts);
    patient::generate_emergency_contacts(&mut ctx).await?;

    // Step 7: Generate patient demographics (1 per patient)
    set_populate_step(state, 6, &ctx.counts);
    patient::generate_patient_demographics(&mut ctx).await?;

    // Step 8: Generate patient insurance links
    set_populate_step(state, 7, &ctx.counts);
    patient::generate_patient_insurance(&mut ctx).await?;

    // Step 9: Generate clinic schedules
    set_populate_step(state, 8, &ctx.counts);
    clinic::generate_clinic_schedules(&mut ctx).await?;

    // Step 10: Generate appointments (Aurora DSQL only — no DynamoDB)
    set_populate_step(state, 9, &ctx.counts);
    appointment::generate_appointments(&mut ctx).await?;

    // Step 11: Generate medical records
    set_populate_step(state, 10, &ctx.counts);
    medical_record::generate_medical_records(&mut ctx).await?;

    // Step 12-13: Generate patient visits and vitals (one each per appointment)
    set_populate_step(state, 11, &ctx.counts);
    visit::generate_patient_visits(&mut ctx).await?;
    set_populate_step(state, 12, &ctx.counts);

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
    state.set_counts(ctx.counts.clone());

    // Mark populate as complete so the UI shows the final state.
    state.set_populate_progress(Some(PopulateProgress {
        current_step: String::new(),
        steps_done: POPULATE_TOTAL_STEPS,
        total_steps: POPULATE_TOTAL_STEPS,
        rows_written: count_aurora_rows(&ctx.counts),
        is_complete: true,
    }));

    Ok(())
}

/// Seed Aurora DSQL with static reference data only (steps 1-8).
///
/// Generates insurance companies, plans, clinics, providers, patients,
/// emergency contacts, demographics, and patient insurance links.
/// Does NOT generate date-dependent data (appointments, schedules, etc.).
///
/// Progress is published with 8 total steps.
pub async fn run_populate_static(
    pool: DbPool,
    dynamo_client: DynamoClient,
    config: SimulationConfig,
    state: &SimulatorState,
) -> Result<(), AppError> {
    let mut ctx = SimulationContext::new(pool, dynamo_client, config);

    tracing::info!("Starting static populate (steps 1-8)");
    let start = Utc::now();

    // Step 1: Insurance Companies
    publish_progress(state, 0, &STATIC_STEP_NAMES, STATIC_TOTAL_STEPS, count_static_rows(&ctx.counts));
    insurance::generate_insurance_companies(&mut ctx).await?;

    // Step 2: Insurance Plans
    publish_progress(state, 1, &STATIC_STEP_NAMES, STATIC_TOTAL_STEPS, count_static_rows(&ctx.counts));
    insurance::generate_insurance_plans(&mut ctx).await?;

    // Step 3: Clinics
    publish_progress(state, 2, &STATIC_STEP_NAMES, STATIC_TOTAL_STEPS, count_static_rows(&ctx.counts));
    clinic::generate_clinics(&mut ctx).await?;

    // Step 4: Providers
    publish_progress(state, 3, &STATIC_STEP_NAMES, STATIC_TOTAL_STEPS, count_static_rows(&ctx.counts));
    provider::generate_providers(&mut ctx).await?;

    // Step 5: Patients (also inserts emergency contacts inline)
    publish_progress(state, 4, &STATIC_STEP_NAMES, STATIC_TOTAL_STEPS, count_static_rows(&ctx.counts));
    patient::generate_patients(&mut ctx).await?;

    // Step 6: Emergency contacts — handled inside generate_patients; no-op here
    publish_progress(state, 5, &STATIC_STEP_NAMES, STATIC_TOTAL_STEPS, count_static_rows(&ctx.counts));
    patient::generate_emergency_contacts(&mut ctx).await?;

    // Step 7: Patient Demographics
    publish_progress(state, 6, &STATIC_STEP_NAMES, STATIC_TOTAL_STEPS, count_static_rows(&ctx.counts));
    patient::generate_patient_demographics(&mut ctx).await?;

    // Step 8: Patient Insurance
    publish_progress(state, 7, &STATIC_STEP_NAMES, STATIC_TOTAL_STEPS, count_static_rows(&ctx.counts));
    patient::generate_patient_insurance(&mut ctx).await?;

    let duration = Utc::now().signed_duration_since(start);
    tracing::info!(
        "Static populate complete in {:.2}s — {} patients, {} providers, {} clinics",
        duration.num_milliseconds() as f64 / 1000.0,
        ctx.counts.patients,
        ctx.counts.providers,
        ctx.counts.clinics,
    );

    state.set_last_run(Utc::now());
    state.set_counts(ctx.counts.clone());

    state.set_populate_progress(Some(PopulateProgress {
        current_step: String::new(),
        steps_done: STATIC_TOTAL_STEPS,
        total_steps: STATIC_TOTAL_STEPS,
        rows_written: count_static_rows(&ctx.counts),
        is_complete: true,
    }));

    Ok(())
}

/// Generate date-dependent data for a specific date range (dynamic populate).
///
/// Queries existing reference data (patients, providers, clinics) from Aurora,
/// then generates clinic schedules (first run only), appointments, medical records,
/// and patient visits within the specified date range.
///
/// Does NOT write to DynamoDB — that's the responsibility of POST /simulate.
/// Counts are updated additively so this can be called multiple times for different ranges.
///
/// Requires a prior run of `run_populate_static` to have seeded reference data.
pub async fn run_populate_dynamic(
    pool: DbPool,
    dynamo_client: DynamoClient,
    state: &SimulatorState,
    start_date: NaiveDate,
    end_date: NaiveDate,
    records_per_appointment: usize,
    clinic_weights: Vec<u32>,
) -> Result<(), AppError> {
    tracing::info!(
        "Starting dynamic populate ({} to {}, {} records/appt, provider-based scheduling)",
        start_date, end_date, records_per_appointment
    );
    let start = Utc::now();

    // Phase A: Query existing reference data from Aurora.
    let patient_rows: Vec<(Uuid,)> = sqlx::query_as(
        "SELECT patient_id FROM vital_fold.patient"
    )
    .fetch_all(&pool)
    .await?;

    let provider_rows: Vec<(i64,)> = sqlx::query_as(
        "SELECT provider_id FROM vital_fold.provider"
    )
    .fetch_all(&pool)
    .await?;

    let clinic_rows: Vec<(i64,)> = sqlx::query_as(
        "SELECT clinic_id FROM vital_fold.clinic"
    )
    .fetch_all(&pool)
    .await?;

    if patient_rows.is_empty() || provider_rows.is_empty() || clinic_rows.is_empty() {
        return Err(AppError::BadRequest(
            "No reference data found. Run POST /populate/static first.".to_string()
        ));
    }

    let pt_ids: Vec<Uuid> = patient_rows.into_iter().map(|(id,)| id).collect();
    let prov_ids: Vec<i64> = provider_rows.into_iter().map(|(id,)| id).collect();
    let cl_ids: Vec<i64>   = clinic_rows.into_iter().map(|(id,)| id).collect();

    tracing::info!(
        "Reference data loaded: {} patients, {} providers, {} clinics",
        pt_ids.len(), prov_ids.len(), cl_ids.len()
    );

    let mut counts = state.get_counts();

    // Step 1: Clinic Schedules — only on first dynamic run
    publish_progress(state, 0, &DYNAMIC_STEP_NAMES, DYNAMIC_TOTAL_STEPS, count_dynamic_rows(&counts));
    if counts.clinic_schedules == 0 {
        let mut ctx = SimulationContext::new(
            pool.clone(), dynamo_client.clone(), SimulationConfig::default(),
        );
        ctx.provider_ids = prov_ids.clone();
        ctx.clinic_ids = cl_ids.clone();
        clinic::generate_clinic_schedules(&mut ctx).await?;
        counts.clinic_schedules = ctx.counts.clinic_schedules;
        tracing::info!("Generated {} clinic schedules", counts.clinic_schedules);
    } else {
        tracing::info!("Clinic schedules already exist ({}), skipping", counts.clinic_schedules);
    }

    // Step 2: Appointments (each provider fills 36 slots/day at their clinic)
    // Returns all appointments including no-shows and cancellations.
    publish_progress(state, 1, &DYNAMIC_STEP_NAMES, DYNAMIC_TOTAL_STEPS, count_dynamic_rows(&counts));
    let appointments = appointment::generate_appointments_by_day(
        &pool, &pt_ids, &prov_ids, &cl_ids,
        start_date, end_date,
        &clinic_weights,
    ).await?;
    let new_appointments = appointments.len();
    counts.appointments += new_appointments;

    // Split: only completed appointments produce downstream clinical records.
    // No-shows and cancellations stay in the appointment table but get nothing else.
    let completed: Vec<(Uuid, Uuid, i64, i64, NaiveDateTime)> = appointments.iter()
        .filter(|(_, _, _, _, _, status)| status == "completed")
        .map(|(id, pt, cl, pv, dt, _)| (*id, *pt, *cl, *pv, *dt))
        .collect();
    let no_shows = appointments.iter().filter(|a| a.5 == "no_show").count();
    let cancellations = appointments.iter().filter(|a| a.5 == "cancelled").count();
    counts.no_shows += no_shows;
    counts.cancellations += cancellations;
    tracing::info!(
        "Appointment status split: {} completed, {} no-show, {} cancelled",
        completed.len(), no_shows, cancellations
    );

    // Step 3: Medical Records (completed appointments only)
    publish_progress(state, 2, &DYNAMIC_STEP_NAMES, DYNAMIC_TOTAL_STEPS, count_dynamic_rows(&counts));
    let new_medical_records = medical_record::generate_medical_records_for_range(
        &pool, &completed, records_per_appointment,
    ).await?;
    counts.medical_records += new_medical_records;

    // Step 4: Patient Visits (completed appointments only)
    publish_progress(state, 3, &DYNAMIC_STEP_NAMES, DYNAMIC_TOTAL_STEPS, count_dynamic_rows(&counts));
    let (visit_ids, ekg_flags, new_vitals) = visit::generate_visits_for_appointments(
        &pool, &completed,
    ).await?;
    let new_visits = visit_ids.len();
    counts.patient_visits += new_visits;

    // Step 5: Patient Vitals (already generated above, just track count)
    publish_progress(state, 4, &DYNAMIC_STEP_NAMES, DYNAMIC_TOTAL_STEPS, count_dynamic_rows(&counts));
    counts.patient_vitals += new_vitals;

    // Step 6: Surveys (~30% of visits fill one out)
    publish_progress(state, 5, &DYNAMIC_STEP_NAMES, DYNAMIC_TOTAL_STEPS, count_dynamic_rows(&counts));
    let new_surveys = survey::generate_surveys_for_visits(&pool, &visit_ids).await?;
    counts.surveys += new_surveys;

    // Step 7: Billing line-items (completed appointments only — ekg_flags aligns 1:1)
    publish_progress(state, 6, &DYNAMIC_STEP_NAMES, DYNAMIC_TOTAL_STEPS, count_dynamic_rows(&counts));
    let new_cpt = rvu::generate_appointment_cpt(&pool, &completed, &ekg_flags).await?;
    counts.appointment_cpt += new_cpt;

    let duration = Utc::now().signed_duration_since(start);
    tracing::info!(
        "Dynamic populate complete in {:.2}s — {} appointments ({} completed, {} no-show, {} cancelled), \
         {} medical records, {} visits, {} vitals, {} surveys, {} cpt line-items",
        duration.num_milliseconds() as f64 / 1000.0,
        new_appointments, completed.len(), no_shows, cancellations,
        new_medical_records, new_visits, new_vitals, new_surveys, new_cpt,
    );

    state.set_last_run(Utc::now());
    state.set_counts(counts.clone());

    state.set_populate_progress(Some(PopulateProgress {
        current_step: String::new(),
        steps_done: DYNAMIC_TOTAL_STEPS,
        total_steps: DYNAMIC_TOTAL_STEPS,
        rows_written: count_dynamic_rows(&counts),
        is_complete: true,
    }));

    Ok(())
}

/// Query distinct dates that have appointments in Aurora DSQL.
///
/// Returns dates sorted ascending. Used by the frontend calendar to show
/// which dates are already populated.
pub async fn get_populated_dates(pool: &DbPool) -> Result<Vec<NaiveDate>, AppError> {
    let rows: Vec<(NaiveDate,)> = sqlx::query_as(
        "SELECT DISTINCT appointment_datetime::date as d FROM vital_fold.appointment ORDER BY d"
    )
    .fetch_all(pool)
    .await?;

    Ok(rows.into_iter().map(|(d,)| d).collect())
}


/// Write DynamoDB records for all visits scheduled for today by reading from Aurora.
///
/// This is Phase 2 of the data lifecycle (POST /simulate). It JOINs patient_visit
/// and patient_vitals from Aurora DSQL where checkin_time matches today, then writes
/// each to both DynamoDB tables. No random data is generated — all values come from Aurora.
///
/// The semaphore caps concurrency at 40 in-flight DynamoDB requests to stay
/// within DynamoDB's 4,000 WCU on-demand throughput limit per table.
pub async fn run_simulate(
    pool: DbPool,
    dynamo_client: DynamoClient,
    state: &SimulatorState,
) -> Result<(), AppError> {
    use crate::models::PatientVisitWithVitals;

    tracing::info!("Starting simulate run — querying today's visits from Aurora");
    let start = Utc::now();

    // JOIN patient_visit + patient_vitals for today's visits.
    let visits: Vec<PatientVisitWithVitals> = sqlx::query_as(
        "SELECT v.patient_visit_id, v.patient_id, v.clinic_id, v.provider_id, \
                v.checkin_time, v.checkout_time, v.provider_seen_time, \
                v.ekg_usage, v.estimated_copay, v.creation_time, v.record_expiration_epoch, \
                vt.height, vt.weight, vt.blood_pressure, vt.heart_rate, \
                vt.temperature, vt.oxygen_saturation \
         FROM vital_fold.patient_visit v \
         JOIN vital_fold.patient_vitals vt ON v.patient_visit_id = vt.patient_visit_id \
         WHERE v.checkin_time::date = CURRENT_DATE"
    )
    .fetch_all(&pool)
    .await?;

    let total = visits.len();

    if total == 0 {
        tracing::warn!(
            "Simulate run found 0 visits for today. \
             Run POST /populate first, then call POST /simulate on a day when appointments exist."
        );
        state.set_last_run(Utc::now());
        return Ok(());
    }

    tracing::info!("Found {} visits for today — writing to DynamoDB", total);

    // Diagnostic: confirm the DynamoDB tables are reachable.
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

    // Bounded concurrency for DynamoDB writes.
    // Each visit spawns 2 writes (patient_visit + patient_vitals).
    const DYNAMO_CONCURRENCY: usize = 40;
    let sem = std::sync::Arc::new(tokio::sync::Semaphore::new(DYNAMO_CONCURRENCY));
    let mut visit_handles: Vec<tokio::task::JoinHandle<bool>> = Vec::with_capacity(total);
    let mut vitals_handles: Vec<tokio::task::JoinHandle<bool>> = Vec::with_capacity(total);

    for visit in &visits {
        // Write to patient_visit table
        let permit = sem.clone().acquire_owned().await
            .map_err(|_| AppError::Internal("DynamoDB semaphore closed unexpectedly".into()))?;
        let client = dynamo_client.clone();
        let v = visit.clone();
        visit_handles.push(tokio::spawn(async move {
            let ok = appointment::write_patient_visit(&client, &v).await;
            drop(permit);
            ok
        }));

        // Write to patient_vitals table
        let permit = sem.clone().acquire_owned().await
            .map_err(|_| AppError::Internal("DynamoDB semaphore closed unexpectedly".into()))?;
        let client = dynamo_client.clone();
        let v = visit.clone();
        vitals_handles.push(tokio::spawn(async move {
            let ok = appointment::write_patient_vitals(&client, &v).await;
            drop(permit);
            ok
        }));
    }

    // Drain handles and tally successful writes per table.
    let mut visit_writes_ok = 0usize;
    for h in visit_handles {
        match h.await {
            Ok(true) => visit_writes_ok += 1,
            Ok(false) => {} // DynamoDB error already logged
            Err(e) => tracing::error!("DynamoDB visit write task panicked: {:?}", e),
        }
    }
    let mut vitals_writes_ok = 0usize;
    for h in vitals_handles {
        match h.await {
            Ok(true) => vitals_writes_ok += 1,
            Ok(false) => {} // DynamoDB error already logged
            Err(e) => tracing::error!("DynamoDB vitals write task panicked: {:?}", e),
        }
    }

    let duration = Utc::now().signed_duration_since(start);
    tracing::info!(
        "Simulate complete in {:.2}s — {}/{} visit writes, {}/{} vitals writes",
        duration.num_milliseconds() as f64 / 1000.0,
        visit_writes_ok, total, vitals_writes_ok, total,
    );

    // Use actual success counts so the dashboard reflects reality.
    let mut counts = state.get_counts();
    counts.dynamo_patient_visits = visit_writes_ok;
    counts.dynamo_patient_vitals = vitals_writes_ok;
    state.set_last_run(Utc::now());
    state.set_counts(counts);

    Ok(())
}

/// Row returned when querying clinic metadata for the timelapse.
#[derive(sqlx::FromRow)]
struct ClinicMeta {
    clinic_id: i64,
    city: String,
    state: String,
}

/// Row returned when counting appointments per clinic for a given date+hour.
#[derive(sqlx::FromRow)]
struct ClinicCount {
    clinic_id: i64,
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
#[allow(dead_code)]
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

    let clinic_map: HashMap<i64, (String, String)> = clinics
        .iter()
        .map(|c| (c.clinic_id, (c.city.clone(), c.state.clone())))
        .collect();

    if clinic_map.is_empty() {
        tracing::warn!("Timelapse: no clinics found. Run POST /populate first.");
        return Ok(());
    }

    // 2. Find the date range of existing appointments.
    let range: Option<(NaiveDate, NaiveDate)> = sqlx::query_as(
        "SELECT MIN(appointment_datetime::date), MAX(appointment_datetime::date) \
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

        // 4. Hour-window loop (8..17 = 9 windows).
        for hour in 8u32..17u32 {
            if !state.is_running() {
                break;
            }

            // Count appointments per clinic for this date+hour.
            let counts: Vec<ClinicCount> = sqlx::query_as(
                "SELECT clinic_id, COUNT(*) as cnt \
                 FROM vital_fold.appointment \
                 WHERE appointment_datetime::date = $1 \
                   AND EXTRACT(HOUR FROM appointment_datetime) = $2 \
                   AND status = 'completed' \
                 GROUP BY clinic_id"
            )
            .bind(current_date)
            .bind(hour as i32)
            .fetch_all(&pool)
            .await?;

            let count_map: HashMap<i64, i64> = counts
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

    for hour in 8u32..17u32 {
        if !state.is_running() {
            tracing::info!("Heatmap stopped by user at hour {}", hour);
            break;
        }

        let counts: Vec<ClinicCount> = sqlx::query_as(
            "SELECT clinic_id, COUNT(*) as cnt \
             FROM vital_fold.appointment \
             WHERE appointment_datetime::date = $1 \
               AND EXTRACT(HOUR FROM appointment_datetime) = $2 \
               AND status = 'completed' \
             GROUP BY clinic_id"
        )
        .bind(date)
        .bind(hour as i32)
        .fetch_all(pool)
        .await?;

        let count_map: HashMap<i64, i64> = counts
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
/// `run_simulate` first to write patient_visit records, then animates
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

/// Sync existing Aurora visit + vitals data to DynamoDB for a specific date range.
///
/// Reads from patient_visit JOIN patient_vitals in Aurora and writes to both
/// DynamoDB tables (patient_visit + patient_vitals). No Aurora data is generated.
///
/// Progress is published to `DynamoProgress` so the UI can show a live progress bar.
/// Requires a prior Dynamic Populate run to have created visits for the target dates.
pub async fn run_date_range_simulate(
    pool: DbPool,
    dynamo_client: DynamoClient,
    state: &SimulatorState,
    start_date: NaiveDate,
    end_date: NaiveDate,
) -> Result<(), AppError> {
    use crate::engine_state::DynamoProgress;

    tracing::info!(
        "Starting date-range DynamoDB sync: {} to {}",
        start_date, end_date
    );
    let start = Utc::now();

    // Publish initial progress while querying Aurora.
    state.set_dynamo_progress(Some(DynamoProgress {
        operation: "Syncing to DynamoDB".to_string(),
        current_table: "Querying Aurora".to_string(),
        tables_done: 0,
        total_tables: 2,
        items_processed: 0,
        total_items: 0,
        is_complete: false,
    }));

    // Read visits from Aurora (JOIN) and write to both DynamoDB tables.
    use crate::models::PatientVisitWithVitals;

    let visits: Vec<PatientVisitWithVitals> = sqlx::query_as(
        "SELECT v.patient_visit_id, v.patient_id, v.clinic_id, v.provider_id, \
                v.checkin_time, v.checkout_time, v.provider_seen_time, \
                v.ekg_usage, v.estimated_copay, v.creation_time, v.record_expiration_epoch, \
                vt.height, vt.weight, vt.blood_pressure, vt.heart_rate, \
                vt.temperature, vt.oxygen_saturation \
         FROM vital_fold.patient_visit v \
         JOIN vital_fold.patient_vitals vt ON v.patient_visit_id = vt.patient_visit_id \
         WHERE v.checkin_time::date >= $1 AND v.checkin_time::date <= $2"
    )
    .bind(start_date)
    .bind(end_date)
    .fetch_all(&pool)
    .await?;

    if visits.is_empty() {
        tracing::warn!(
            "Date-range DynamoDB sync: 0 visits found for {} to {}. Nothing to sync.",
            start_date, end_date
        );
        state.set_last_run(Utc::now());
        state.set_dynamo_progress(None);
        return Ok(());
    }

    let total = visits.len() as u64;
    // Total items = visits written to patient_visit + visits written to patient_vitals
    let total_items = total * 2;

    tracing::info!(
        "Date-range DynamoDB sync: found {} visits to write for {} to {}",
        visits.len(), start_date, end_date
    );

    state.set_dynamo_progress(Some(DynamoProgress {
        operation: "Syncing to DynamoDB".to_string(),
        current_table: "Patient Visits".to_string(),
        tables_done: 0,
        total_tables: 2,
        items_processed: 0,
        total_items,
        is_complete: false,
    }));

    const DYNAMO_CONCURRENCY: usize = 40;
    let sem = std::sync::Arc::new(tokio::sync::Semaphore::new(DYNAMO_CONCURRENCY));
    let mut visit_handles: Vec<tokio::task::JoinHandle<bool>> = Vec::with_capacity(visits.len());
    let mut vitals_handles: Vec<tokio::task::JoinHandle<bool>> = Vec::with_capacity(visits.len());

    for visit in &visits {
        if !state.is_running() {
            tracing::info!("Date-range DynamoDB sync stopped by user during writes");
            break;
        }

        // Write to patient_visit table
        let permit = sem.clone().acquire_owned().await
            .map_err(|_| AppError::Internal("DynamoDB semaphore closed unexpectedly".into()))?;
        let client = dynamo_client.clone();
        let v = visit.clone();
        visit_handles.push(tokio::spawn(async move {
            let ok = appointment::write_patient_visit(&client, &v).await;
            drop(permit);
            ok
        }));

        // Write to patient_vitals table
        let permit = sem.clone().acquire_owned().await
            .map_err(|_| AppError::Internal("DynamoDB semaphore closed unexpectedly".into()))?;
        let client = dynamo_client.clone();
        let v = visit.clone();
        vitals_handles.push(tokio::spawn(async move {
            let ok = appointment::write_patient_vitals(&client, &v).await;
            drop(permit);
            ok
        }));
    }

    // Drain visit handles (table 1 of 2), publishing progress after each completion.
    let mut visit_writes_ok = 0usize;
    for (i, h) in visit_handles.into_iter().enumerate() {
        match h.await {
            Ok(true) => visit_writes_ok += 1,
            Ok(false) => {}
            Err(e) => tracing::error!("DynamoDB visit write task panicked: {:?}", e),
        }
        // Publish progress every 50 completions or on the last one.
        if (i + 1) % 50 == 0 || i + 1 == total as usize {
            state.set_dynamo_progress(Some(DynamoProgress {
                operation: "Syncing to DynamoDB".to_string(),
                current_table: "Patient Visits".to_string(),
                tables_done: 0,
                total_tables: 2,
                items_processed: (i + 1) as u64,
                total_items,
                is_complete: false,
            }));
        }
    }

    // Drain vitals handles (table 2 of 2).
    let mut vitals_writes_ok = 0usize;
    for (i, h) in vitals_handles.into_iter().enumerate() {
        match h.await {
            Ok(true) => vitals_writes_ok += 1,
            Ok(false) => {}
            Err(e) => tracing::error!("DynamoDB vitals write task panicked: {:?}", e),
        }
        if (i + 1) % 50 == 0 || i + 1 == total as usize {
            state.set_dynamo_progress(Some(DynamoProgress {
                operation: "Syncing to DynamoDB".to_string(),
                current_table: "Patient Vitals".to_string(),
                tables_done: 1,
                total_tables: 2,
                items_processed: total + (i + 1) as u64,
                total_items,
                is_complete: false,
            }));
        }
    }

    let duration = Utc::now().signed_duration_since(start);
    tracing::info!(
        "Date-range DynamoDB sync complete in {:.2}s — \
         {}/{} visit writes, {}/{} vitals writes",
        duration.num_milliseconds() as f64 / 1000.0,
        visit_writes_ok, visits.len(), vitals_writes_ok, visits.len(),
    );

    // Mark complete.
    state.set_dynamo_progress(Some(DynamoProgress {
        operation: "Syncing to DynamoDB".to_string(),
        current_table: String::new(),
        tables_done: 2,
        total_tables: 2,
        items_processed: (visit_writes_ok + vitals_writes_ok) as u64,
        total_items,
        is_complete: true,
    }));

    // Update state with DynamoDB write counts only.
    let mut counts = state.get_counts();
    counts.dynamo_patient_visits  += visit_writes_ok;
    counts.dynamo_patient_vitals  += vitals_writes_ok;
    state.set_last_run(Utc::now());
    state.set_counts(counts);

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
