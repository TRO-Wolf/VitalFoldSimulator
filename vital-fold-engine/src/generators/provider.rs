/// Generate healthcare providers (doctors) with realistic fake names.
///
/// Uses the `fake` crate to generate random provider names and specialties.
/// Each provider is assigned a random specialty and medical license type.

use crate::errors::AppError;
use fake::Fake;
use fake::faker::name::en::{FirstName, LastName};
use fake::faker::internet::en::SafeEmail;
use rand::Rng;

/// Generate a phone number guaranteed to fit within VARCHAR(20).
/// Format: +1-NXX-NXX-XXXX (18 chars)
fn gen_phone(rng: &mut impl Rng) -> String {
    format!(
        "+1-{}{}{}-{}{}{}-{}{}{}{}",
        rng.gen_range(2..=9),
        rng.gen_range(0..=9),
        rng.gen_range(0..=9),
        rng.gen_range(2..=9),
        rng.gen_range(0..=9),
        rng.gen_range(0..=9),
        rng.gen_range(0..=9),
        rng.gen_range(0..=9),
        rng.gen_range(0..=9),
        rng.gen_range(0..=9),
    )
}

use super::SimulationContext;

/// Cardiac-focused specialties for providers.
const SPECIALTIES: &[&str] = &[
    "Cardiologist",
    "Cardiac Surgeon",
    "Electrophysiologist",
    "Interventional Cardiologist",
];

/// Medical license types.
const LICENSE_TYPES: &[&str] = &["MD", "DO"];

/// Generate N providers with random names and specialties.
pub async fn generate_providers(ctx: &mut SimulationContext) -> Result<(), AppError> {
    use rand::Rng;

    for _ in 0..ctx.config.providers {
        let first_name: String = FirstName().fake();
        let last_name: String = LastName().fake();

        let (specialty, license_type, phone, email) = {
            use rand::thread_rng;
            let mut rng = thread_rng();
            (
                SPECIALTIES[rng.gen_range(0..SPECIALTIES.len())],
                LICENSE_TYPES[rng.gen_range(0..LICENSE_TYPES.len())],
                gen_phone(&mut rng),
                SafeEmail().fake::<String>(),
            )
        };

        let result: (uuid::Uuid,) = sqlx::query_as(
            "INSERT INTO vital_fold.provider (first_name, last_name, specialty, license_type, phone_number, email) VALUES ($1, $2, $3, $4, $5, $6) RETURNING provider_id"
        )
        .bind(&first_name)
        .bind(&last_name)
        .bind(specialty)
        .bind(license_type)
        .bind(&phone)
        .bind(&email)
        .fetch_one(&ctx.pool)
        .await?;

        ctx.provider_ids.push(result.0);
        ctx.counts.providers += 1;
    }

    tracing::info!("Generated {} providers", ctx.counts.providers);

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_specialties_count() {
        assert!(SPECIALTIES.len() > 0);
    }

    #[test]
    fn test_license_types() {
        assert_eq!(LICENSE_TYPES.len(), 2);
        assert!(LICENSE_TYPES.contains(&"MD"));
        assert!(LICENSE_TYPES.contains(&"DO"));
    }
}
