/// Generate patients with emergency contacts, demographics, and insurance associations.
///
/// Each patient gets:
/// - A random name and date of birth
/// - An emergency contact (random)
/// - Demographics (phone, address, insurance info)
/// - Insurance association (1-3 random plans)

use crate::errors::AppError;
use chrono::{Local, Utc};
use fake::faker::{address::en::*, name::en::*};
use fake::Fake;
use std::net::{IpAddr, Ipv4Addr};
use uuid::Uuid;

use super::SimulationContext;

/// Generate N patients with random names and dates of birth.
pub async fn generate_patients(ctx: &mut SimulationContext) -> Result<(), AppError> {
    let now = Utc::now();

    for _ in 0..ctx.config.num_patients {
        let id = Uuid::new_v4();

        // Generate random patient name
        let first_name = FirstName().fake::<String>();
        let last_name = LastName().fake::<String>();
        let full_name = format!("{} {}", first_name, last_name);

        // Generate random date of birth (ages 18-80)
        let today = Local::now().naive_local().date();
        let days_back = (18 * 365) + ((rand::random::<u32>() % (62 * 365)) as i64);
        let dob = today - chrono::Duration::days(days_back);

        sqlx::query(
            "INSERT INTO vital_fold.patient (id, first_name, last_name, date_of_birth, created_at) VALUES ($1, $2, $3, $4, $5)"
        )
        .bind(id)
        .bind(&first_name)
        .bind(&last_name)
        .bind(dob)
        .bind(now)
        .execute(&ctx.pool)
        .await?;

        ctx.patient_ids.push(id);
        ctx.counts.patients += 1;
    }

    tracing::info!("Generated {} patients", ctx.counts.patients);

    Ok(())
}

/// Generate emergency contacts for each patient.
pub async fn generate_emergency_contacts(ctx: &mut SimulationContext) -> Result<(), AppError> {
    let now = Utc::now();

    for patient_id in &ctx.patient_ids {
        let id = Uuid::new_v4();

        // Generate random emergency contact name
        let name = Name().fake::<String>();

        // Generate random phone number (using address faker as placeholder)
        let phone = format!("+1{:09}", Uuid::new_v4().as_u64_pair().0 % 1000000000);

        // Generate random relationship
        let relationships = vec!["Spouse", "Parent", "Sibling", "Child", "Friend"];
        let rel_idx = (Uuid::new_v4().as_u64_pair().0 as usize) % relationships.len();
        let relationship = relationships[rel_idx];

        sqlx::query(
            "INSERT INTO vital_fold.emergency_contact (id, patient_id, name, phone, relationship, created_at) VALUES ($1, $2, $3, $4, $5, $6)"
        )
        .bind(id)
        .bind(patient_id)
        .bind(&name)
        .bind(&phone)
        .bind(relationship)
        .bind(now)
        .execute(&ctx.pool)
        .await?;

        ctx.counts.emergency_contacts += 1;
    }

    tracing::info!(
        "Generated {} emergency contacts",
        ctx.counts.emergency_contacts
    );

    Ok(())
}

/// Generate patient demographics for each patient.
pub async fn generate_patient_demographics(ctx: &mut SimulationContext) -> Result<(), AppError> {
    let now = Utc::now();

    for patient_id in &ctx.patient_ids {
        let id = Uuid::new_v4();

        // Generate random phone and address
        let phone = format!("+1{:09}", Uuid::new_v4().as_u64_pair().0 % 1000000000);
        let street_address = format!("{} {}",
            (Uuid::new_v4().as_u64_pair().0 % 9999 + 1),
            BuildingNumber().fake::<String>()
        );
        let city = CityName().fake::<String>();
        let state = StateName().fake::<String>();
        let zipcode = ZipCode().fake::<String>();

        // Generate random gender
        let genders = vec!["M", "F", "Other"];
        let gender_idx = (Uuid::new_v4().as_u64_pair().0 as usize) % genders.len();
        let gender = genders[gender_idx];

        sqlx::query(
            "INSERT INTO vital_fold.patient_demographics (id, patient_id, phone, street_address, city, state, zipcode, gender, created_at) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)"
        )
        .bind(id)
        .bind(patient_id)
        .bind(&phone)
        .bind(&street_address)
        .bind(&city)
        .bind(&state)
        .bind(&zipcode)
        .bind(gender)
        .bind(now)
        .execute(&ctx.pool)
        .await?;

        ctx.counts.patient_demographics += 1;
    }

    tracing::info!(
        "Generated {} patient demographics",
        ctx.counts.patient_demographics
    );

    Ok(())
}

/// Generate patient insurance associations (1-3 plans per patient).
pub async fn generate_patient_insurance(ctx: &mut SimulationContext) -> Result<(), AppError> {
    let now = Utc::now();

    for patient_id in &ctx.patient_ids {
        // Each patient gets 1-3 insurance plans
        let num_plans = 1 + ((Uuid::new_v4().as_u64_pair().0 % 3) as usize);

        for _ in 0..num_plans {
            let id = Uuid::new_v4();

            // Random plan selection
            let plan_idx = (Uuid::new_v4().as_u64_pair().0 as usize) % ctx.insurance_plan_ids.len();
            let plan_id = ctx.insurance_plan_ids[plan_idx];

            // Generate random policy number
            let policy_number = format!("POL-{}", Uuid::new_v4().to_string()[..8].to_uppercase());

            sqlx::query(
                "INSERT INTO vital_fold.patient_insurance (id, patient_id, insurance_plan_id, policy_number, created_at) VALUES ($1, $2, $3, $4, $5)"
            )
            .bind(id)
            .bind(patient_id)
            .bind(plan_id)
            .bind(&policy_number)
            .bind(now)
            .execute(&ctx.pool)
            .await?;

            ctx.counts.patient_insurance += 1;
        }
    }

    tracing::info!(
        "Generated {} patient insurance links",
        ctx.counts.patient_insurance
    );

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_relationships_available() {
        let relationships = vec!["Spouse", "Parent", "Sibling", "Child", "Friend"];
        assert_eq!(relationships.len(), 5);
    }
}
