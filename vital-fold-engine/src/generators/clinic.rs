/// Generate clinics with fixed geographic distribution.
///
/// The company operates 10 clinics across SE US with specific city/state distribution.
/// Clinic schedules are generated for provider-clinic pairs.

use chrono::NaiveTime;
use fake::Fake;
use fake::faker::address::en::StreetName;
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

const STREET_SUFFIXES: &[&str] = &[
    "Blvd", "Ave", "Dr", "Pkwy", "Way", "Ln", "Ct", "Rd", "St", "Pl",
];

/// Generate the 10 fixed clinics and insert them into the database.
pub async fn generate_clinics(ctx: &mut SimulationContext) -> Result<(), AppError> {
    // Track city occurrence count for duplicate cities (Atlanta×2, Miami×2, Jacksonville×2)
    let mut city_count: std::collections::HashMap<&str, usize> = std::collections::HashMap::new();

    for (idx, (city, state, region)) in CLINIC_DISTRIBUTION.iter().enumerate() {
        let count = city_count.entry(city).or_insert(0);
        *count += 1;
        let occurrence = *count;

        // Clinic name: append number only for cities with multiple clinics
        let clinic_name = if CLINIC_DISTRIBUTION.iter().filter(|(c, _, _)| c == city).count() > 1 {
            format!("VitalFold Heart Center - {} {}", city, occurrence)
        } else {
            format!("VitalFold Heart Center - {}", city)
        };

        // Realistic street address: "1234 Elm Blvd, Suite 200"
        let street_address = {
            let mut rng = rand::rng();
            let street_name: String = StreetName().fake();
            let suffix = STREET_SUFFIXES[rng.random_range(0..STREET_SUFFIXES.len())];
            let number = rng.random_range(100..9999u32);
            let suite = rng.random_range(100..500u32);
            format!("{} {} {}, Suite {}", number, street_name, suffix, suite)
        };

        let zip_prefix = super::patient::METRO_AREAS[idx].2;
        let zip_code = {
            let mut rng = rand::rng();
            format!("{}{:02}", zip_prefix, rng.random_range(0..100u32))
        };

        // Email matches clinic identity: vfhc_miami1@vitalfold.org
        let city_slug = city.to_lowercase().replace(' ', "");
        let email = if CLINIC_DISTRIBUTION.iter().filter(|(c, _, _)| c == city).count() > 1 {
            format!("vfhc_{}{}@vitalfold.org", city_slug, occurrence)
        } else {
            format!("vfhc_{}@vitalfold.org", city_slug)
        };

        let phone = {
            let mut rng = rand::rng();
            gen_phone(&mut rng)
        };

        let result: (i64,) = sqlx::query_as(
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
///
/// Each provider is scheduled at their primary clinic (from `provider_clinic_assignments`).
/// ~30% of providers also work at a second random clinic. Each works 3-5 days per week.
/// If `provider_clinic_assignments` is empty (e.g. dynamic populate path), falls back
/// to the old behavior of 1-2 random clinics.
pub async fn generate_clinic_schedules(ctx: &mut SimulationContext) -> Result<(), AppError> {
    use rand::Rng;
    let open_time = NaiveTime::from_hms_opt(8, 0, 0).unwrap_or_default();
    let close_time = NaiveTime::from_hms_opt(17, 0, 0).unwrap_or_default();

    let has_assignments = !ctx.provider_clinic_assignments.is_empty();

    for (prov_idx, provider_id) in ctx.provider_ids.iter().enumerate() {
        let selected_clinics = {
            use rand::rng;
            let mut rng = rng();
            if has_assignments {
                // Primary clinic from proportional assignment
                let primary = ctx.clinic_ids[ctx.provider_clinic_assignments[prov_idx] % ctx.clinic_ids.len()];
                let mut clinics = vec![primary];
                // 30% chance of a second clinic (any random one)
                if rng.random_bool(0.3) {
                    let second = ctx.clinic_ids[rng.random_range(0..ctx.clinic_ids.len())];
                    if second != primary {
                        clinics.push(second);
                    }
                }
                clinics
            } else {
                // Fallback: 1-2 random clinics (dynamic populate path)
                let n = rng.random_range(1..=2);
                (0..n)
                    .map(|_| ctx.clinic_ids[rng.random_range(0..ctx.clinic_ids.len())])
                    .collect()
            }
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
