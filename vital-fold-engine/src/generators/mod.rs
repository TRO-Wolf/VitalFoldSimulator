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
use crate::engine_state::{SimulationCounts, SimulatorState};
use crate::errors::AppError;
use chrono::{NaiveDate, NaiveDateTime, Utc};
use serde::{Deserialize, Serialize};
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
        let pr_id      = row.provider_id;
        let appt_dt    = row.appointment_date;
        handles.push(tokio::spawn(async move {
            appointment::write_patient_visit(&client, pt_id, cl_id, pr_id, appt_dt).await;
            drop(permit);
            true // signals this was a visit write
        }));

        // patient_vitals write — appointment_dt needed for record_expiration_epoch
        let permit     = sem.clone().acquire_owned().await.unwrap();
        let client     = dynamo_client.clone();
        let pt_id      = row.patient_id;
        let cl_id      = row.clinic_id;
        let pr_id      = row.provider_id;
        let visit_id   = row.appointment_id;
        let appt_dt    = row.appointment_date;
        handles.push(tokio::spawn(async move {
            appointment::write_patient_vitals(&client, pt_id, cl_id, pr_id, visit_id, appt_dt).await;
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
