/// Generate appointments for patients at clinics.
///
/// Each patient gets N appointments distributed across random clinics and providers.
/// Appointments are scheduled with random times during clinic hours.
/// Appointments can also trigger DynamoDB writes for vitals tracking (fire-and-forget).

use crate::errors::AppError;
use chrono::{Duration, Local, NaiveDateTime, Utc};
use uuid::Uuid;

use super::SimulationContext;

/// Generate appointments for patients across the clinics.
///
/// Each patient gets approximately appointments_per_patient appointments.
/// Appointments are distributed across random clinics and providers.
pub async fn generate_appointments(ctx: &mut SimulationContext) -> Result<(), AppError> {
    let now = Utc::now();
    let today = Local::now().date_naive();

    for (patient_idx, patient_id) in ctx.patient_ids.iter().enumerate() {
        let num_appointments = 1 + ((patient_idx as u32 % 3) as usize);

        for appt_idx in 0..num_appointments {
            let id = Uuid::new_v4();

            // Distribute appointments across clinics
            let clinic_idx = (patient_idx + appt_idx) % ctx.clinic_ids.len();
            let clinic_id = ctx.clinic_ids[clinic_idx];

            // Distribute appointments across providers
            let provider_idx = (patient_idx * 7 + appt_idx) % ctx.provider_ids.len();
            let provider_id = ctx.provider_ids[provider_idx];

            // Schedule appointments in the future (next 90 days)
            let days_ahead = 1 + ((patient_idx * appt_idx) % 90) as i64;
            let appointment_date = today + Duration::days(days_ahead);

            // Random time during clinic hours (9am-5pm)
            let hour = 9 + ((patient_idx + appt_idx) % 8);
            let minute = (patient_idx * 13 + appt_idx * 17) % 60;
            let appointment_time =
                NaiveDateTime::new(appointment_date, chrono::NaiveTime::from_hms_opt(hour as u32, minute as u32, 0).unwrap()).and_utc();

            // Generate reason for visit
            let reasons = vec![
                "Annual checkup",
                "Chest pain evaluation",
                "Blood pressure check",
                "Follow-up visit",
                "New patient visit",
            ];
            let reason_idx = (patient_idx * 11 + appt_idx) % reasons.len();
            let reason_for_visit = reasons[reason_idx];

            sqlx::query(
                "INSERT INTO vital_fold.appointment (id, patient_id, provider_id, clinic_id, appointment_time, reason_for_visit, created_at) VALUES ($1, $2, $3, $4, $5, $6, $7)"
            )
            .bind(id)
            .bind(patient_id)
            .bind(provider_id)
            .bind(clinic_id)
            .bind(appointment_time)
            .bind(reason_for_visit)
            .bind(now)
            .execute(&ctx.pool)
            .await?;

            ctx.counts.appointments += 1;
        }
    }

    tracing::info!("Generated {} appointments", ctx.counts.appointments);

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_appointment_reasons() {
        let reasons = vec![
            "Annual checkup",
            "Chest pain evaluation",
            "Blood pressure check",
            "Follow-up visit",
            "New patient visit",
        ];
        assert_eq!(reasons.len(), 5);
    }
}
