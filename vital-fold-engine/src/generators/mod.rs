/// Data generator modules for the simulation engine.
///
/// This module contains generators for each table in the vital_fold schema,
/// orchestrated by a SimulationConfig that controls what and how much data to generate.

pub mod insurance;
pub mod clinic;
pub mod provider;
pub mod patient;
pub mod appointment;
pub mod medical_record;

use crate::db::DbPool;
use crate::engine_state::{SimulationCounts, SimulatorState};
use crate::errors::AppError;
use chrono::Utc;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Configuration for a simulation run.
/// Controls how many of each entity to generate.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SimulationConfig {
    /// Number of providers to generate
    pub num_providers: usize,

    /// Number of patients to generate
    pub num_patients: usize,

    /// Number of appointments per patient (approximate)
    pub appointments_per_patient: usize,

    /// Number of medical records per patient (approximate)
    pub medical_records_per_patient: usize,
}

impl Default for SimulationConfig {
    fn default() -> Self {
        SimulationConfig {
            num_providers: 50,
            num_patients: 100,
            appointments_per_patient: 3,
            medical_records_per_patient: 2,
        }
    }
}

/// Context passed through all generators containing shared data and pool references.
pub struct SimulationContext {
    /// Database connection pool
    pub pool: DbPool,

    /// Simulation configuration
    pub config: SimulationConfig,

    /// Counts accumulated during this simulation run
    pub counts: SimulationCounts,

    /// Insurance company IDs (will be populated during insurance generation)
    pub insurance_company_ids: Vec<Uuid>,

    /// Insurance plan IDs (will be populated during plan generation)
    pub insurance_plan_ids: Vec<Uuid>,

    /// Clinic IDs (will be populated during clinic generation)
    pub clinic_ids: Vec<Uuid>,

    /// Provider IDs (will be populated during provider generation)
    pub provider_ids: Vec<Uuid>,

    /// Patient IDs (will be populated during patient generation)
    pub patient_ids: Vec<Uuid>,

    /// Clinic schedule IDs (will be populated during schedule generation)
    pub clinic_schedule_ids: Vec<Uuid>,
}

impl SimulationContext {
    /// Create a new SimulationContext.
    pub fn new(pool: DbPool, config: SimulationConfig) -> Self {
        SimulationContext {
            pool,
            config,
            counts: SimulationCounts::default(),
            insurance_company_ids: Vec::new(),
            insurance_plan_ids: Vec::new(),
            clinic_ids: Vec::new(),
            provider_ids: Vec::new(),
            patient_ids: Vec::new(),
            clinic_schedule_ids: Vec::new(),
        }
    }
}

/// Run a complete simulation: generate and insert all data.
///
/// # Execution Order
/// 1. Insurance companies (7 fixed)
/// 2. Insurance plans (fixed)
/// 3. Clinics (10 fixed distribution)
/// 4. Providers (N random)
/// 5. Patients (N random)
/// 6. Emergency contacts (1 per patient)
/// 7. Patient demographics (1 per patient)
/// 8. Patient insurance links (random per patient)
/// 9. Clinic schedules (fixed per clinic)
/// 10. Appointments (random per patient/clinic)
/// 11. Medical records (random per appointment)
///
/// # Arguments
/// * `pool` - Database connection pool
/// * `config` - Simulation configuration
/// * `state` - Global simulator state to update with counts and timestamp
///
/// # Returns
/// * `Result<(), AppError>` - Success or database error
pub async fn run_simulation(
    pool: DbPool,
    config: SimulationConfig,
    state: &SimulatorState,
) -> Result<(), AppError> {
    let mut ctx = SimulationContext::new(pool, config);

    tracing::info!("Starting simulation run");
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

    // Step 5: Generate patients
    tracing::debug!("Generating patients");
    patient::generate_patients(&mut ctx).await?;

    // Step 6: Generate emergency contacts (1 per patient)
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

    // Step 10: Generate appointments
    tracing::debug!("Generating appointments");
    appointment::generate_appointments(&mut ctx).await?;

    // Step 11: Generate medical records
    tracing::debug!("Generating medical records");
    medical_record::generate_medical_records(&mut ctx).await?;

    let duration = Utc::now().signed_duration_since(start);
    tracing::info!(
        "Simulation complete in {:.2}s - inserted {} entities",
        duration.num_milliseconds() as f64 / 1000.0,
        ctx.counts.insurance_companies
            + ctx.counts.clinics
            + ctx.counts.providers
            + ctx.counts.patients
            + ctx.counts.appointments
    );

    // Update global state with final counts and timestamp
    state.set_last_run(Utc::now());
    state.set_counts(ctx.counts);

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simulation_config_default() {
        let config = SimulationConfig::default();
        assert!(config.num_providers > 0);
        assert!(config.num_patients > 0);
    }
}
