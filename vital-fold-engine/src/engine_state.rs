use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;
use utoipa::ToSchema;

/// Per-clinic activity snapshot used by the timelapse heatmap.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ClinicActivity {
    pub clinic_id: String,
    pub city: String,
    pub state: String,
    pub active_patients: usize,
}

/// State of a running (or completed) timelapse simulation.
/// Updated once per hour-window by `run_timelapse` and read by `GET /simulate/heatmap`.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct TimelapseState {
    /// Current simulated date, e.g. "2026-03-15"
    pub simulation_day: String,
    /// 1-based day counter
    pub day_number: usize,
    /// Total days in the timelapse
    pub total_days: usize,
    /// Current simulated hour (9..17)
    pub sim_hour: u32,
    /// Per-clinic appointment counts for the current hour-window
    pub clinics: Vec<ClinicActivity>,
    /// True when the timelapse has finished all days
    pub is_complete: bool,
}

/// Row counts from the last completed populate or simulate run.
/// Populated by POST /populate (Aurora DSQL fields) and POST /simulate (DynamoDB fields).
///
/// Aurora DSQL fields — set by POST /populate:
///   insurance_companies, insurance_plans, clinics, providers, patients,
///   emergency_contacts, patient_demographics, patient_insurance,
///   clinic_schedules, appointments, medical_records
///
/// DynamoDB fields — set by POST /simulate (day-of visit writes):
///   dynamo_patient_visits, dynamo_patient_vitals
#[derive(Debug, Clone, Default, Serialize, Deserialize, ToSchema)]
pub struct SimulationCounts {
    pub insurance_companies: usize,
    pub insurance_plans: usize,
    pub clinics: usize,
    pub providers: usize,
    pub patients: usize,
    pub emergency_contacts: usize,
    pub patient_demographics: usize,
    pub patient_insurance: usize,
    pub clinic_schedules: usize,
    pub appointments: usize,
    pub medical_records: usize,
    /// DynamoDB patient_visit records written by the last simulate run.
    pub dynamo_patient_visits: usize,
    /// DynamoDB patient_vitals records written by the last simulate run.
    pub dynamo_patient_vitals: usize,
}

/// Global state for the data simulator.
/// Tracks whether a simulation is running and stores metrics from the last run.
pub struct SimulatorState {
    /// Flag indicating if a simulation is currently running
    pub running: AtomicBool,

    /// Timestamp of the last completed simulation run
    pub last_run: Mutex<Option<DateTime<Utc>>>,

    /// Row counts from the last completed simulation run
    pub counts: Mutex<SimulationCounts>,

    /// Timelapse heatmap state (None when no timelapse has been run)
    pub timelapse: Mutex<Option<TimelapseState>>,
}

impl SimulatorState {
    /// Create a new SimulatorState with all fields initialized to defaults.
    pub fn new() -> Self {
        SimulatorState {
            running: AtomicBool::new(false),
            last_run: Mutex::new(None),
            counts: Mutex::new(SimulationCounts::default()),
            timelapse: Mutex::new(None),
        }
    }

    /// Check if a simulation is currently running (non-blocking).
    pub fn is_running(&self) -> bool {
        self.running.load(Ordering::SeqCst)
    }

    /// Attempt to mark the simulator as running.
    /// Returns true if the simulator was idle and is now marked as running.
    /// Returns false if a simulation was already in progress.
    pub fn try_start(&self) -> bool {
        self.running
            .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
            .is_ok()
    }

    /// Mark the simulator as stopped.
    pub fn stop(&self) {
        self.running.store(false, Ordering::SeqCst);
    }

    /// Get the timestamp of the last completed run (if any).
    pub fn get_last_run(&self) -> Option<DateTime<Utc>> {
        *self.last_run.lock().unwrap()
    }

    /// Update the last run timestamp to now.
    pub fn set_last_run(&self, timestamp: DateTime<Utc>) {
        *self.last_run.lock().unwrap() = Some(timestamp);
    }

    /// Get a clone of the current simulation counts.
    pub fn get_counts(&self) -> SimulationCounts {
        self.counts.lock().unwrap().clone()
    }

    /// Update the simulation counts.
    pub fn set_counts(&self, counts: SimulationCounts) {
        *self.counts.lock().unwrap() = counts;
    }

    /// Get a clone of the current timelapse state (if any).
    pub fn get_timelapse(&self) -> Option<TimelapseState> {
        self.timelapse.lock().unwrap().clone()
    }

    /// Update the timelapse state.
    pub fn set_timelapse(&self, state: Option<TimelapseState>) {
        *self.timelapse.lock().unwrap() = state;
    }
}

impl Default for SimulatorState {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simulator_state_creation() {
        let state = SimulatorState::new();
        assert!(!state.is_running());
        assert_eq!(state.get_last_run(), None);
    }

    #[test]
    fn test_try_start_transitions() {
        let state = SimulatorState::new();

        // First attempt should succeed
        assert!(state.try_start());
        assert!(state.is_running());

        // Second attempt should fail (already running)
        assert!(!state.try_start());
        assert!(state.is_running());

        // After stop, should be able to start again
        state.stop();
        assert!(!state.is_running());
        assert!(state.try_start());
    }

    #[test]
    fn test_counts_updates() {
        let state = SimulatorState::new();
        let mut counts = SimulationCounts::default();
        counts.patients = 100;
        counts.appointments = 50;

        state.set_counts(counts.clone());
        let retrieved = state.get_counts();

        assert_eq!(retrieved.patients, 100);
        assert_eq!(retrieved.appointments, 50);
    }

    #[test]
    fn test_last_run_timestamp() {
        let state = SimulatorState::new();
        let now = Utc::now();

        state.set_last_run(now);
        let retrieved = state.get_last_run();

        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap(), now);
    }
}
