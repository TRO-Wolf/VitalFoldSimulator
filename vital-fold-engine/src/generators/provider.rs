/// Generate healthcare providers (doctors) with realistic fake names.
///
/// Uses the `fake` crate to generate random provider names and specialties.
/// Each provider is assigned a random specialty and medical license type.

use crate::errors::AppError;
use fake::Fake;
use fake::faker::name::en::{FirstName, LastName};
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

use super::SimulationContext;

/// Cardiac-focused specialties for providers.
const SPECIALTIES: &[&str] = &[
    "Cardiologist",
    "Cardiac Surgeon",
    "Electrophysiologist",
    "Interventional Cardiologist",
];

/// Physician license types (selected when not NP).
const PHYSICIAN_TYPES: &[&str] = &["MD", "DO"];

/// Generate N providers with random names and specialties.
///
/// Providers are assigned to clinics proportionally based on `config.clinic_weights`.
/// Busier clinics (higher weight) get more providers. Each provider's primary clinic
/// index is stored in `ctx.provider_clinic_assignments` for use by schedule generation.
pub async fn generate_providers(ctx: &mut SimulationContext) -> Result<(), AppError> {
    use rand::distr::{Distribution, weighted::WeightedIndex};

    let n = ctx.config.providers;

    // Build all provider data synchronously — rng dropped before any await.
    let (first_names, last_names, specialties, license_types, phones, emails, clinic_indices) = {
        let dist = WeightedIndex::new(&ctx.config.clinic_weights)
            .or_else(|_| WeightedIndex::new(&super::DEFAULT_CLINIC_WEIGHTS))
            .map_err(|e| crate::errors::AppError::Internal(
                format!("clinic weights invalid: {}", e)))?;
        let mut rng = rand::rng();

        let mut first_names:    Vec<String> = Vec::with_capacity(n);
        let mut last_names:     Vec<String> = Vec::with_capacity(n);
        let mut specialties:    Vec<&str>   = Vec::with_capacity(n);
        let mut license_types:  Vec<&str>   = Vec::with_capacity(n);
        let mut phones:         Vec<String> = Vec::with_capacity(n);
        let mut emails:         Vec<String> = Vec::with_capacity(n);
        let mut clinic_indices: Vec<usize>  = Vec::with_capacity(n);

        for _ in 0..n {
            let first: String = loop {
                let name: String = FirstName().fake();
                if name != "Adolf" { break name; }
            };
            let last: String = LastName().fake();

            // ~30% NP, ~70% MD/DO
            let license = if rng.random_bool(0.3) {
                "NP"
            } else {
                PHYSICIAN_TYPES[rng.random_range(0..PHYSICIAN_TYPES.len())]
            };

            // Email: first initial + last name @ example.org (e.g. j.smith@example.org)
            let first_initial = first.chars().next().unwrap_or_else(|| 'x');
            let email = format!(
                "{}.{}@example.org",
                first_initial.to_lowercase(),
                last.to_lowercase().replace(' ', "")
            );

            specialties.push(SPECIALTIES[rng.random_range(0..SPECIALTIES.len())]);
            license_types.push(license);
            phones.push(gen_phone(&mut rng));
            emails.push(email);
            first_names.push(first);
            last_names.push(last);
            clinic_indices.push(dist.sample(&mut rng));
        }

        (first_names, last_names, specialties, license_types, phones, emails, clinic_indices)
    }; // rng + dist dropped here before any await

    for i in 0..n {
        let result: (i64,) = sqlx::query_as(
            "INSERT INTO vital_fold.provider (first_name, last_name, specialty, license_type, phone_number, email) VALUES ($1, $2, $3, $4, $5, $6) RETURNING provider_id"
        )
        .bind(&first_names[i])
        .bind(&last_names[i])
        .bind(specialties[i])
        .bind(license_types[i])
        .bind(&phones[i])
        .bind(&emails[i])
        .fetch_one(&ctx.pool)
        .await?;

        ctx.provider_ids.push(result.0);
        ctx.provider_clinic_assignments.push(clinic_indices[i]);
        ctx.counts.providers += 1;
    }

    tracing::info!("Generated {} providers (distributed by clinic weight)", ctx.counts.providers);

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
    fn test_physician_types() {
        assert_eq!(PHYSICIAN_TYPES.len(), 2);
        assert!(PHYSICIAN_TYPES.contains(&"MD"));
        assert!(PHYSICIAN_TYPES.contains(&"DO"));
    }
}
