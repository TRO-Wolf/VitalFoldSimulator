/// Generate medical records for appointments.
///
/// Records are bulk-inserted in chunks of DSQL_BATCH_SIZE to stay under Aurora
/// DSQL's 3000-row per-transaction limit.

use crate::errors::AppError;
use uuid::Uuid;

use super::SimulationContext;

/// Aurora DSQL maximum rows per transaction statement.
const DSQL_BATCH_SIZE: usize = 2500;

/// Fixed diagnosis codes (cardiac-focused clinic).
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

/// Treatment options mapped by diagnosis for realistic data.
fn get_treatment_for_diagnosis(diagnosis: &str) -> &'static str {
    match diagnosis {
        "Atrial Fibrillation (AFib)"   => "Anticoagulation therapy",
        "Coronary Artery Disease (CAD)" => "Statin therapy",
        "Chest Pain"                   => "Stress test ordered",
        "Hypertension"                 => "ACE inhibitor",
        "Hyperlipidemia"               => "Statin initiated",
        "Shortness of Breath (SOB)"    => "Pulmonary function test",
        "Tachycardia"                  => "Beta blocker",
        "Bradycardia"                  => "Pacemaker evaluation",
        _                              => "Follow-up appointment scheduled",
    }
}

/// Generate medical records for appointments in chunked bulk inserts.
pub async fn generate_medical_records(ctx: &mut SimulationContext) -> Result<(), AppError> {
    // Fetch all appointments once to get the FK data needed for records.
    let appointments: Vec<(Uuid, Uuid, Uuid, Uuid, chrono::NaiveDateTime)> = sqlx::query_as(
        "SELECT appointment_id, patient_id, provider_id, clinic_id, appointment_date \
         FROM vital_fold.appointment"
    )
    .fetch_all(&ctx.pool)
    .await?;

    let total = appointments.len() * ctx.config.records_per_appointment;

    // Build all record data synchronously — rng dropped before any await.
    let (pt_ids, provider_ids, clinic_ids, record_dates, diagnoses, treatments) = {
        use rand::{thread_rng, Rng};
        let mut rng = thread_rng();

        let mut pt_ids:       Vec<Uuid>              = Vec::with_capacity(total);
        let mut provider_ids: Vec<Uuid>              = Vec::with_capacity(total);
        let mut clinic_ids:   Vec<Uuid>              = Vec::with_capacity(total);
        let mut record_dates: Vec<chrono::NaiveDateTime> = Vec::with_capacity(total);
        let mut diagnoses:    Vec<String>            = Vec::with_capacity(total);
        let mut treatments:   Vec<String>            = Vec::with_capacity(total);

        for &(_, patient_id, provider_id, clinic_id, appointment_date) in &appointments {
            for _ in 0..ctx.config.records_per_appointment {
                let diagnosis = DIAGNOSES[rng.gen_range(0..DIAGNOSES.len())];
                let treatment = get_treatment_for_diagnosis(diagnosis);
                let offset    = rng.gen_range(15..120i64);

                pt_ids.push(patient_id);
                provider_ids.push(provider_id);
                clinic_ids.push(clinic_id);
                record_dates.push(appointment_date + chrono::Duration::minutes(offset));
                diagnoses.push(diagnosis.to_string());
                treatments.push(treatment.to_string());
            }
        }

        (pt_ids, provider_ids, clinic_ids, record_dates, diagnoses, treatments)
    }; // rng dropped here

    for chunk_start in (0..total).step_by(DSQL_BATCH_SIZE) {
        let chunk_end = (chunk_start + DSQL_BATCH_SIZE).min(total);
        let r = chunk_start..chunk_end;

        sqlx::query(
            "INSERT INTO vital_fold.medical_record \
             (patient_id, provider_id, clinic_id, record_date, diagnosis, treatment) \
             SELECT * FROM UNNEST($1::uuid[], $2::uuid[], $3::uuid[], $4::timestamp[], $5::text[], $6::text[])"
        )
        .bind(&pt_ids[r.clone()])
        .bind(&provider_ids[r.clone()])
        .bind(&clinic_ids[r.clone()])
        .bind(&record_dates[r.clone()])
        .bind(&diagnoses[r.clone()])
        .bind(&treatments[r.clone()])
        .execute(&ctx.pool)
        .await?;

        ctx.counts.medical_records += chunk_end - chunk_start;
    }

    tracing::info!("Generated {} medical records", ctx.counts.medical_records);

    Ok(())
}
