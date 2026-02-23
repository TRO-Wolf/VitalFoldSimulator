/// Generate medical records for appointments.
///
/// Each appointment can have medical records created.
/// Records include diagnosis codes from a fixed cardiac-focused list
/// and treatment descriptions.

use crate::errors::AppError;
use chrono::Utc;
use uuid::Uuid;

use super::SimulationContext;

/// Fixed diagnosis codes (cardiac-focused clinic).
/// These are the canonical spellings from domain values.
const DIAGNOSES: &[&str] = &[
    "Atrial Fibrillation (AFib)",
    "Coronary Artery Disease (CAD)",
    "Chest Pain",
    "Hypertension",
    "Hyperlipidemia",
    "Shortness of Breath (SOB)",
    "Tachycardia",
    "Bradycardia",
];

/// Treatment options.
const TREATMENTS: &[&str] = &[
    "Prescribed medication",
    "Lifestyle modification counseling",
    "Referral to cardiology",
    "EKG ordered",
    "Stress test ordered",
    "Imaging study scheduled",
    "Follow-up appointment scheduled",
    "Patient education provided",
];

/// Generate medical records for appointments.
///
/// Most appointments get 1-2 medical records with diagnoses and treatments.
pub async fn generate_medical_records(ctx: &mut SimulationContext) -> Result<(), AppError> {
    let now = Utc::now();

    // Iterate through appointments and create medical records
    // Since we don't store appointment references, we'll create records
    // proportional to appointments
    let records_to_create = ctx.counts.appointments * ctx.config.medical_records_per_patient / 2;

    for appt_idx in 0..records_to_create {
        let id = Uuid::new_v4();

        // Select random patient and appointment (by index simulation)
        let patient_idx = appt_idx % ctx.patient_ids.len();
        let patient_id = ctx.patient_ids[patient_idx];

        // Select random diagnosis
        let diag_idx = (appt_idx * 7) % DIAGNOSES.len();
        let diagnosis = DIAGNOSES[diag_idx];

        // Select 1-2 treatments
        let num_treatments = 1 + ((appt_idx % 2) as usize);
        let mut treatment_list = Vec::new();
        for t_idx in 0..num_treatments {
            let t = (appt_idx * 13 + t_idx) % TREATMENTS.len();
            treatment_list.push(TREATMENTS[t]);
        }
        let treatment = treatment_list.join("; ");

        sqlx::query(
            "INSERT INTO vital_fold.medical_record (id, patient_id, diagnosis, treatment, created_at) VALUES ($1, $2, $3, $4, $5)"
        )
        .bind(id)
        .bind(patient_id)
        .bind(diagnosis)
        .bind(&treatment)
        .bind(now)
        .execute(&ctx.pool)
        .await?;

        ctx.counts.medical_records += 1;
    }

    tracing::info!("Generated {} medical records", ctx.counts.medical_records);

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_diagnoses_count() {
        assert_eq!(DIAGNOSES.len(), 8);
    }

    #[test]
    fn test_diagnoses_spelling() {
        // Verify exact spellings from domain values
        assert!(DIAGNOSES.contains(&"Atrial Fibrillation (AFib)"));
        assert!(DIAGNOSES.contains(&"Coronary Artery Disease (CAD)"));
        assert!(DIAGNOSES.contains(&"Hypertension"));
    }

    #[test]
    fn test_treatments_available() {
        assert!(TREATMENTS.len() > 0);
    }
}
