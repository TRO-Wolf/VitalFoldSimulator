/// Generate insurance companies and plans.
///
/// Insurance companies are fixed (7 companies from domain values).
/// Insurance plans are dynamically generated based on SimulationConfig.

use crate::errors::AppError;
use chrono::NaiveDate;
use fake::Fake;
use fake::faker::internet::en::SafeEmail;
use rand::Rng;
use sqlx::types::BigDecimal;

use super::SimulationContext;

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

/// Fixed insurance company names from domain values.
const INSURANCE_COMPANIES: &[&str] = &[
    "Orange Spear",
    "Care Medical",
    "Cade Medical",
    "Multiplied Health",
    "Octi Care",
    "Tatnay",
    "Caymana",
];

/// Generate the 7 fixed insurance companies and insert them into the database.
pub async fn generate_insurance_companies(ctx: &mut SimulationContext) -> Result<(), AppError> {
    use rand::Rng;

    for company_name in INSURANCE_COMPANIES {
        let (phone, tax_id) = {
            use rand::thread_rng;
            let mut rng = thread_rng();
            (gen_phone(&mut rng), rng.gen_range(100_000_000..999_999_999))
        };
        let email = SafeEmail().fake::<String>();

        let result: (uuid::Uuid,) = sqlx::query_as(
            "INSERT INTO vital_fold.insurance_company (company_name, email, phone_number, tax_id_number) VALUES ($1, $2, $3, $4) RETURNING company_id"
        )
        .bind(company_name)
        .bind(email)
        .bind(phone)
        .bind(tax_id)
        .fetch_one(&ctx.pool)
        .await?;

        ctx.company_ids.push(result.0);
        ctx.counts.insurance_companies += 1;
    }

    tracing::info!(
        "Generated {} insurance companies",
        ctx.counts.insurance_companies
    );

    Ok(())
}

/// Generate insurance plans for each company and insert them into the database.
///
/// Each company gets plans_per_company plans.
pub async fn generate_insurance_plans(ctx: &mut SimulationContext) -> Result<(), AppError> {
    use rand::Rng;
    let plans_per_company = ctx.config.plans_per_company;

    for company_id in &ctx.company_ids {
        for i in 0..plans_per_company {
            let plan_name = format!("Plan {}", i + 1);

            let (deductible, copay, prior_auth, active) = {
                use rand::thread_rng;
                let mut rng = thread_rng();
                (
                    BigDecimal::from(rng.gen_range(250..2000)),
                    BigDecimal::from(rng.gen_range(20..150)),
                    rng.gen_bool(0.5),
                    rng.gen_bool(0.8),
                )
            };

            let start_date = NaiveDate::from_ymd_opt(2024, 1, 1).unwrap();

            let result: (uuid::Uuid,) = sqlx::query_as(
                "INSERT INTO vital_fold.insurance_plan (plan_name, company_id, deductible_amount, copay_amount, prior_auth_required, active_plan, active_start_date) VALUES ($1, $2, $3, $4, $5, $6, $7) RETURNING insurance_plan_id"
            )
            .bind(plan_name)
            .bind(company_id)
            .bind(deductible)
            .bind(copay)
            .bind(prior_auth)
            .bind(active)
            .bind(start_date)
            .fetch_one(&ctx.pool)
            .await?;

            ctx.plan_ids.push(result.0);
            ctx.counts.insurance_plans += 1;
        }
    }

    tracing::info!("Generated {} insurance plans", ctx.counts.insurance_plans);

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_insurance_companies_count() {
        assert_eq!(INSURANCE_COMPANIES.len(), 7);
    }

    #[test]
    fn test_insurance_company_names() {
        // Verify exact spellings from domain values
        assert!(INSURANCE_COMPANIES.contains(&"Orange Spear"));
        assert!(INSURANCE_COMPANIES.contains(&"Care Medical"));
        assert!(INSURANCE_COMPANIES.contains(&"Cade Medical"));
    }
}
