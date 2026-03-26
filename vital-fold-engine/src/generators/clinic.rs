/// Generate clinics with fixed geographic distribution.
///
/// The company operates 10 clinics across SE US with specific city/state distribution.
/// Clinic schedules are generated for provider-clinic pairs.

use chrono::NaiveTime;
use fake::Fake;
use fake::faker::internet::en::SafeEmail;
use fake::faker::address::en::{StreetName, ZipCode};
use rand::Rng;

/// Generate a phone number guaranteed to fit within VARCHAR(20).
/// Format: +1-NXX-NXX-XXXX (18 chars)
fn gen_phone(rng: &mut impl Rng) -> String {
    format!(
        "+1-{}{}{}-{}{}{}-{}{}{}{}",
        rng.random_range(2..=9),
        rng.random_range(0..=9),
        rng.random_range(0..=9),
        rng.random_range(2..=9),
        rng.random_range(0..=9),
        rng.random_range(0..=9),
        rng.random_range(0..=9),
        rng.random_range(0..=9),
        rng.random_range(0..=9),
        rng.random_range(0..=9),
    )
}

use crate::errors::AppError;
use super::SimulationContext;

/// Fixed clinic distribution: (city, state)
const CLINIC_DISTRIBUTION: &[(&str, &str, &str)] = &[
    ("Charlotte", "NC", "NC"),
    ("Asheville", "NC", "NC"),
    ("Atlanta", "GA", "GA"),
    ("Atlanta", "GA", "GA"),
    ("Tallahassee", "FL", "FL"),
    ("Miami", "FL", "FL"),
    ("Miami", "FL", "FL"),
    ("Orlando", "FL", "FL"),
    ("Jacksonville", "FL", "FL"),
    ("Jacksonville", "FL", "FL"),
];

const WEEKDAYS: &[&str] = &["Monday", "Tuesday", "Wednesday", "Thursday", "Friday"];

/// Generate the 10 fixed clinics and insert them into the database.
pub async fn generate_clinics(ctx: &mut SimulationContext) -> Result<(), AppError> {
    for (idx, (city, state, region)) in CLINIC_DISTRIBUTION.iter().enumerate() {
        let clinic_name = format!("VitalFold Heart Center - {}", city);
        let street_address: String = StreetName().fake();
        let zip_code: String = ZipCode().fake();
        let (phone, email) = {
            use rand::rng;
            let mut rng = rng();
            (gen_phone(&mut rng), SafeEmail().fake::<String>())
        };

        let result: (uuid::Uuid,) = sqlx::query_as(
            "INSERT INTO vital_fold.clinic (clinic_name, region, street_address, city, state, zip_code, phone_number, email) VALUES ($1, $2, $3, $4, $5, $6, $7, $8) RETURNING clinic_id"
        )
        .bind(&clinic_name)
        .bind(region)
        .bind(&street_address)
        .bind(city)
        .bind(state)
        .bind(&zip_code)
        .bind(&phone)
        .bind(&email)
        .fetch_one(&ctx.pool)
        .await?;

        ctx.clinic_ids.push(result.0);
        ctx.counts.clinics += 1;
    }

    tracing::info!("Generated {} clinics", ctx.counts.clinics);

    Ok(())
}

/// Generate clinic schedules for provider-clinic pairs.
/// Each provider is assigned to 1-2 clinics, working 3-5 days per week.
pub async fn generate_clinic_schedules(ctx: &mut SimulationContext) -> Result<(), AppError> {
    use rand::Rng;
    let open_time = NaiveTime::from_hms_opt(9, 0, 0).expect("09:00 is a valid time"); // 9:00 AM
    let close_time = NaiveTime::from_hms_opt(17, 0, 0).expect("17:00 is a valid time"); // 5:00 PM

    for provider_id in &ctx.provider_ids {
        let (num_clinics, selected_clinics) = {
            use rand::rng;
            let mut rng = rng();
            let n = rng.random_range(1..=2);
            let clinics: Vec<_> = (0..n)
                .map(|_| ctx.clinic_ids[rng.random_range(0..ctx.clinic_ids.len())])
                .collect();
            (n, clinics)
        };

        for clinic_id in selected_clinics {
            let selected_days = {
                use rand::rng;
                let mut rng = rng();
                let num_days = rng.random_range(3..=5);
                (0..num_days)
                    .map(|_| WEEKDAYS[rng.random_range(0..WEEKDAYS.len())])
                    .collect::<Vec<_>>()
            };

            for day_of_week in selected_days {
                let _result: (uuid::Uuid,) = sqlx::query_as(
                    "INSERT INTO vital_fold.clinic_schedule (clinic_id, provider_id, day_of_week, start_time, end_time) VALUES ($1, $2, $3, $4, $5) RETURNING schedule_id"
                )
                .bind(&clinic_id)
                .bind(provider_id)
                .bind(day_of_week)
                .bind(open_time)
                .bind(close_time)
                .fetch_one(&ctx.pool)
                .await?;

                ctx.counts.clinic_schedules += 1;
            }
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
        // Florida should have 6 clinics: Tallahassee, Miami×2, Orlando, Jacksonville×2
        let florida_count = CLINIC_DISTRIBUTION
            .iter()
            .filter(|(_, state, _)| *state == "FL")
            .count();
        assert_eq!(florida_count, 6);
    }

    #[test]
    fn test_clinic_distribution_carolina() {
        // North Carolina should have 2 clinics
        let carolina_count = CLINIC_DISTRIBUTION
            .iter()
            .filter(|(_, state, _)| *state == "NC")
            .count();
        assert_eq!(carolina_count, 2);
    }
}
