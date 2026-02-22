/// Generate clinics with fixed geographic distribution.
///
/// The company operates 10 clinics across SE US with specific city/state distribution.
/// Clinic schedules are fixed (Monday-Friday, 9am-5pm) for each clinic.

use chrono::Utc;
use chrono::naive::NaiveTime;
use uuid::Uuid;

use crate::errors::AppError;
use super::SimulationContext;

/// Fixed clinic distribution: (city, state, count)
const CLINIC_DISTRIBUTION: &[(&str, &str)] = &[
    ("Charlotte", "NC"),
    ("Asheville", "NC"),
    ("Atlanta", "GA"),
    ("Atlanta", "GA"),
    ("Tallahassee", "FL"),
    ("Miami", "FL"),
    ("Miami", "FL"),
    ("Orlando", "FL"),
    ("Jacksonville", "FL"),
    ("Jacksonville", "FL"),
];

/// Generate the 10 fixed clinics and insert them into the database.
pub async fn generate_clinics(ctx: &mut SimulationContext) -> Result<(), AppError> {
    let now = Utc::now();

    for (idx, (city, state)) in CLINIC_DISTRIBUTION.iter().enumerate() {
        let id = Uuid::new_v4();
        let clinic_name = format!("VF {} {} (Clinic {})", city, state, idx + 1);

        sqlx::query!(
            "INSERT INTO vital_fold.clinic (id, name, city, state, created_at) VALUES ($1, $2, $3, $4, $5)",
            id,
            &clinic_name,
            city,
            state,
            now
        )
        .execute(&ctx.pool)
        .await?;

        ctx.clinic_ids.push(id);
        ctx.counts.clinics += 1;
    }

    tracing::info!("Generated {} clinics", ctx.counts.clinics);

    Ok(())
}

/// Generate clinic schedules (Monday-Friday, 9am-5pm) for each clinic.
pub async fn generate_clinic_schedules(ctx: &mut SimulationContext) -> Result<(), AppError> {
    let now = Utc::now();
    let open_time = NaiveTime::from_hms_opt(9, 0, 0).unwrap(); // 9:00 AM
    let close_time = NaiveTime::from_hms_opt(17, 0, 0).unwrap(); // 5:00 PM

    // Generate schedules for Monday (1) through Friday (5)
    for clinic_id in &ctx.clinic_ids {
        for day_of_week in 1..=5 {
            let id = Uuid::new_v4();

            sqlx::query!(
                "INSERT INTO vital_fold.clinic_schedule (id, clinic_id, day_of_week, open_time, close_time, created_at) VALUES ($1, $2, $3, $4, $5, $6)",
                id,
                clinic_id,
                day_of_week,
                open_time,
                close_time,
                now
            )
            .execute(&ctx.pool)
            .await?;

            ctx.clinic_schedule_ids.push(id);
            ctx.counts.clinic_schedules += 1;
        }
    }

    tracing::info!(
        "Generated {} clinic schedules",
        ctx.counts.clinic_schedules
    );

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_clinic_distribution_count() {
        assert_eq!(CLINIC_DISTRIBUTION.len(), 10);
    }

    #[test]
    fn test_clinic_distribution_florida() {
        // Florida should have 5 clinics (largest cluster)
        let florida_count = CLINIC_DISTRIBUTION
            .iter()
            .filter(|(_, state)| *state == "FL")
            .count();
        assert_eq!(florida_count, 5);
    }

    #[test]
    fn test_clinic_distribution_carolina() {
        // Carolinas should have 3 clinics
        let carolina_count = CLINIC_DISTRIBUTION
            .iter()
            .filter(|(_, state)| *state == "NC")
            .count();
        assert_eq!(carolina_count, 2);
    }
}
