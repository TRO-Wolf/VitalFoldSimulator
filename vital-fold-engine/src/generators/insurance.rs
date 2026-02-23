/// Generate insurance companies and plans.
///
/// Insurance companies are fixed (7 companies from domain values).
/// Insurance plans are fixed with specific company associations.

use crate::errors::AppError;
use bigdecimal::BigDecimal;
use chrono::Utc;
use uuid::Uuid;

use super::SimulationContext;

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
    let now = Utc::now();

    for company_name in INSURANCE_COMPANIES {
        let id = Uuid::new_v4();

        sqlx::query(
            "INSERT INTO vital_fold.insurance_company (id, name, created_at) VALUES ($1, $2, $3)"
        )
        .bind(id)
        .bind(company_name)
        .bind(now)
        .execute(&ctx.pool)
        .await?;

        ctx.insurance_company_ids.push(id);
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
/// Each company gets 2-3 plans with different premium tiers.
pub async fn generate_insurance_plans(ctx: &mut SimulationContext) -> Result<(), AppError> {
    let now = Utc::now();

    // Define plans for each company: (company_index, plan_name, deductible, monthly_premium)
    let plans = vec![
        // Orange Spear
        (0, "Basic", BigDecimal::from(1000), BigDecimal::from(150)),
        (0, "Standard", BigDecimal::from(500), BigDecimal::from(250)),
        (0, "Premium", BigDecimal::from(0), BigDecimal::from(400)),
        // Care Medical
        (1, "Essential", BigDecimal::from(1500), BigDecimal::from(140)),
        (1, "Preferred", BigDecimal::from(750), BigDecimal::from(270)),
        // Cade Medical
        (2, "Bronze", BigDecimal::from(2000), BigDecimal::from(130)),
        (2, "Silver", BigDecimal::from(1000), BigDecimal::from(240)),
        (2, "Gold", BigDecimal::from(500), BigDecimal::from(350)),
        // Multiplied Health
        (3, "Classic", BigDecimal::from(1200), BigDecimal::from(160)),
        (3, "Advanced", BigDecimal::from(600), BigDecimal::from(280)),
        // Octi Care
        (4, "Basic", BigDecimal::from(1800), BigDecimal::from(120)),
        (4, "Plus", BigDecimal::from(900), BigDecimal::from(220)),
        // Tatnay
        (5, "Standard", BigDecimal::from(1500), BigDecimal::from(145)),
        (5, "Premium", BigDecimal::from(750), BigDecimal::from(290)),
        // Caymana
        (6, "Plan A", BigDecimal::from(2000), BigDecimal::from(125)),
        (6, "Plan B", BigDecimal::from(1000), BigDecimal::from(235)),
        (6, "Plan C", BigDecimal::from(250), BigDecimal::from(380)),
    ];

    for (company_idx, plan_name, deductible, monthly_premium) in plans {
        let company_id = ctx.insurance_company_ids[company_idx];
        let id = Uuid::new_v4();

        sqlx::query(
            "INSERT INTO vital_fold.insurance_plan (id, insurance_company_id, name, deductible, monthly_premium, created_at) VALUES ($1, $2, $3, $4, $5, $6)"
        )
        .bind(id)
        .bind(company_id)
        .bind(plan_name)
        .bind(deductible)
        .bind(monthly_premium)
        .bind(now)
        .execute(&ctx.pool)
        .await?;

        ctx.insurance_plan_ids.push(id);
        ctx.counts.insurance_plans += 1;
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
