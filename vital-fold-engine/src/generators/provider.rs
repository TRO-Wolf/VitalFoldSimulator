/// Generate healthcare providers (doctors) with realistic fake names.
///
/// Uses the `fake` crate to generate random provider names and specialties.
/// Each provider is assigned a random specialty and medical license type.

use crate::errors::AppError;
use chrono::Utc;
use fake::faker::name::en::*;
use fake::Fake;
use uuid::Uuid;

use super::SimulationContext;

/// Medical specialties for providers.
const SPECIALTIES: &[&str] = &[
    "Cardiology",
    "Internal Medicine",
    "Family Medicine",
    "Emergency Medicine",
    "Neurology",
    "Pulmonology",
    "Gastroenterology",
    "Rheumatology",
];

/// Medical license types.
const LICENSE_TYPES: &[&str] = &["MD", "DO"];

/// Generate N providers with random names and specialties.
pub async fn generate_providers(ctx: &mut SimulationContext) -> Result<(), AppError> {
    let now = Utc::now();

    for _ in 0..ctx.config.num_providers {
        let id = Uuid::new_v4();

        // Generate a random provider name using the `fake` crate
        let name = Name().fake::<String>();

        // Randomly select a specialty
        let specialty_idx = (Uuid::new_v4().as_u64_pair().0 as usize) % SPECIALTIES.len();
        let specialty = SPECIALTIES[specialty_idx];

        // Randomly select a license type
        let license_idx = (Uuid::new_v4().as_u64_pair().0 as usize) % LICENSE_TYPES.len();
        let license_type = LICENSE_TYPES[license_idx];

        sqlx::query!(
            "INSERT INTO vital_fold.provider (id, name, specialty, license_type, created_at) VALUES ($1, $2, $3, $4, $5)",
            id,
            &name,
            specialty,
            license_type,
            now
        )
        .execute(&ctx.pool)
        .await?;

        ctx.provider_ids.push(id);
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
