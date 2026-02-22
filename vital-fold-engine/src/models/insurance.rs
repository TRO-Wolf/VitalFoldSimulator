use chrono::NaiveDate;
use serde::{Deserialize, Serialize};
use sqlx::types::BigDecimal;
use uuid::Uuid;

/// Insurance company (e.g., "Orange Spear", "Care Medical")
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct InsuranceCompany {
    pub company_id: Uuid,
    pub company_name: String,
    pub email: String,
    pub phone_number: String,
    pub tax_id_number: i32,
}

/// Insurance plan offered by a company.
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct InsurancePlan {
    pub insurance_plan_id: Uuid,
    pub plan_name: String,
    pub company_id: Uuid,
    pub deductible_amount: BigDecimal,
    pub copay_amount: BigDecimal,
    pub prior_auth_required: bool,
    pub active_plan: bool,
    pub active_start_date: NaiveDate,
}

/// Patient's insurance enrollment linking to a plan.
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct PatientInsurance {
    pub patient_insurance_id: Uuid,
    pub patient_id: Uuid,
    pub insurance_plan_id: Uuid,
    pub policy_number: String,
    pub coverage_start_date: NaiveDate,
    pub coverage_end_date: Option<NaiveDate>,
}
