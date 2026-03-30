use crate::errors::AppError;
use rand::Rng;
use rand::distr::weighted::WeightedIndex;
use rand::distr::Distribution;
use super::SimulationContext;

/// Aurora DSQL maximum rows per transaction statement.
/// Keep well under the 3000-row hard limit.
const DSQL_BATCH_SIZE: usize = 2500;

/// Metro area definitions matching the 10 clinics in CLINIC_DISTRIBUTION (clinic.rs).
/// Each entry: (city, state_abbr, zip_prefix, population_weight).
/// Weights approximate relative metro population so larger cities get more patients.
/// Index order matches CLINIC_DISTRIBUTION exactly.
const METRO_AREAS: &[(&str, &str, &str, u32)] = &[
    // idx 0: Charlotte, NC — metro pop ~2.7M
    ("Charlotte",    "NC", "282", 12),
    // idx 1: Asheville, NC — metro pop ~0.5M
    ("Asheville",    "NC", "287",  3),
    // idx 2: Atlanta, GA (clinic 1) — metro pop ~6.1M
    ("Atlanta",      "GA", "303", 14),
    // idx 3: Atlanta, GA (clinic 2)
    ("Atlanta",      "GA", "303", 14),
    // idx 4: Tallahassee, FL — metro pop ~0.4M
    ("Tallahassee",  "FL", "323",  2),
    // idx 5: Miami, FL (clinic 1) — metro pop ~6.2M
    ("Miami",        "FL", "331", 14),
    // idx 6: Miami, FL (clinic 2)
    ("Miami",        "FL", "331", 14),
    // idx 7: Orlando, FL — metro pop ~2.7M
    ("Orlando",      "FL", "328", 12),
    // idx 8: Jacksonville, FL (clinic 1) — metro pop ~1.7M
    ("Jacksonville", "FL", "322",  8),
    // idx 9: Jacksonville, FL (clinic 2)
    ("Jacksonville", "FL", "322",  8),
];

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

struct PatientBatch {
    ec_ids:           Vec<uuid::Uuid>,
    ec_first_names:   Vec<String>,
    ec_last_names:    Vec<String>,
    ec_relationships: Vec<String>,
    ec_phones:        Vec<String>,
    ec_emails:        Vec<String>,

    pt_first_names:   Vec<String>,
    pt_last_names:    Vec<String>,
    pt_dobs:          Vec<chrono::NaiveDate>,
    pt_streets:       Vec<String>,
    pt_cities:        Vec<String>,
    pt_states:        Vec<String>,
    pt_zips:          Vec<String>,
    pt_phones:        Vec<String>,
    pt_emails:        Vec<String>,
    pt_reg_dates:     Vec<chrono::NaiveDate>,

    /// Index into METRO_AREAS / clinic_ids for each patient's "home" clinic.
    home_clinic_indices: Vec<usize>,
}

/// Build all patient + emergency contact row data in memory (no awaits).
/// Returns the batch so the caller can drop all RNG state before awaiting.
fn build_patient_batch(n: usize) -> PatientBatch {
    use fake::Fake;
    use fake::faker::name::en::{FirstName, LastName};
    use fake::faker::address::en::StreetName;
    use fake::faker::internet::en::SafeEmail;
    use chrono::Local;
    use rand::rng;
    use uuid::Uuid;

    let today = Local::now().naive_local().date();
    let mut rng = rng();
    let relationships = ["Spouse", "Parent", "Sibling", "Child", "Friend"];

    // Build weighted distribution for home clinic assignment.
    let weights: Vec<u32> = METRO_AREAS.iter().map(|m| m.3).collect();
    let dist = WeightedIndex::new(&weights).expect("METRO_AREAS weights are all positive");

    let mut batch = PatientBatch {
        ec_ids:           Vec::with_capacity(n),
        ec_first_names:   Vec::with_capacity(n),
        ec_last_names:    Vec::with_capacity(n),
        ec_relationships: Vec::with_capacity(n),
        ec_phones:        Vec::with_capacity(n),
        ec_emails:        Vec::with_capacity(n),

        pt_first_names:   Vec::with_capacity(n),
        pt_last_names:    Vec::with_capacity(n),
        pt_dobs:          Vec::with_capacity(n),
        pt_streets:       Vec::with_capacity(n),
        pt_cities:        Vec::with_capacity(n),
        pt_states:        Vec::with_capacity(n),
        pt_zips:          Vec::with_capacity(n),
        pt_phones:        Vec::with_capacity(n),
        pt_emails:        Vec::with_capacity(n),
        pt_reg_dates:     Vec::with_capacity(n),
        home_clinic_indices: Vec::with_capacity(n),
    };

    for _ in 0..n {
        batch.ec_ids.push(Uuid::new_v4());
        batch.ec_first_names.push(loop {
            let name: String = FirstName().fake();
            if name != "Adolf" { break name; }
        });
        batch.ec_last_names.push(LastName().fake());
        batch.ec_relationships.push(relationships[rng.random_range(0..relationships.len())].to_string());
        batch.ec_phones.push(gen_phone(&mut rng));
        batch.ec_emails.push(SafeEmail().fake());

        batch.pt_first_names.push(loop {
            let name: String = FirstName().fake();
            if name != "Adolf" { break name; }
        });
        batch.pt_last_names.push(LastName().fake());
        let days_back = (18 * 365) + rng.random_range(0..(62 * 365)) as i64;
        batch.pt_dobs.push(today - chrono::TimeDelta::days(days_back));

        // Assign patient to a home metro area proportional to population weight.
        let metro_idx = dist.sample(&mut rng);
        let (city, state, zip_prefix, _) = METRO_AREAS[metro_idx];
        batch.pt_streets.push(StreetName().fake());
        batch.pt_cities.push(city.to_string());
        batch.pt_states.push(state.to_string());
        batch.pt_zips.push(format!("{}{:02}", zip_prefix, rng.random_range(0..100u32)));
        batch.home_clinic_indices.push(metro_idx);

        batch.pt_phones.push(gen_phone(&mut rng));
        batch.pt_emails.push(SafeEmail().fake());
        batch.pt_reg_dates.push(today);
    }

    batch
}

/// Generate N patients and their emergency contacts in chunked bulk INSERT passes.
///
/// Each chunk is ≤ DSQL_BATCH_SIZE rows to stay under Aurora DSQL's 3000-row
/// per-transaction limit. Emergency contact UUIDs are pre-generated client-side
/// so patients can reference them without a per-row UPDATE.
pub async fn generate_patients(ctx: &mut SimulationContext) -> Result<(), AppError> {
    let n: usize = ctx.config.patients;

    // Generate all row data synchronously — rng is dropped before any await.
    let batch: PatientBatch = build_patient_batch(n);

    // Process in chunks to respect the DSQL 3000-row per-transaction limit.
    for chunk_start in (0..n).step_by(DSQL_BATCH_SIZE) {
        let chunk_end: usize = (chunk_start + DSQL_BATCH_SIZE).min(n);
        let chunk_size = chunk_end - chunk_start;
        let r = chunk_start..chunk_end;

        // Bulk INSERT emergency contacts for this chunk — one round-trip.
        let ec_ids_chunk: &[uuid::Uuid] = &batch.ec_ids[r.clone()];
        let ec_placeholder: &[uuid::Uuid]  = ec_ids_chunk; // temp patient_id, fixed in UPDATE below

        sqlx::query(
            "INSERT INTO vital_fold.emergency_contact \
             (emergency_contact_id, patient_id, first_name, last_name, relationship, phone_number, email) \
             SELECT * FROM UNNEST($1::uuid[], $2::uuid[], $3::text[], $4::text[], $5::text[], $6::text[], $7::text[])"
        )
        .bind(ec_ids_chunk)
        .bind(ec_placeholder)
        .bind(&batch.ec_first_names[r.clone()])
        .bind(&batch.ec_last_names[r.clone()])
        .bind(&batch.ec_relationships[r.clone()])
        .bind(&batch.ec_phones[r.clone()])
        .bind(&batch.ec_emails[r.clone()])
        .execute(&ctx.pool)
        .await?;

        ctx.counts.emergency_contacts += chunk_size;

        // Bulk INSERT patients for this chunk — one round-trip.
        let ec_id_strs: Vec<String> = ec_ids_chunk
            .iter()
            .map(|id| id.to_string())
            .collect();

        let patient_rows: Vec<(uuid::Uuid,)> = sqlx::query_as(
            "INSERT INTO vital_fold.patient \
             (first_name, last_name, date_of_birth, street_address, city, state, zip_code, \
              phone_number, email, registration_date, emergency_contact_id) \
             SELECT * FROM UNNEST($1::text[], $2::text[], $3::date[], $4::text[], $5::text[], \
                                  $6::text[], $7::text[], $8::text[], $9::text[], $10::date[], $11::text[]) \
             RETURNING patient_id"
        )
        .bind(&batch.pt_first_names[r.clone()])
        .bind(&batch.pt_last_names[r.clone()])
        .bind(&batch.pt_dobs[r.clone()])
        .bind(&batch.pt_streets[r.clone()])
        .bind(&batch.pt_cities[r.clone()])
        .bind(&batch.pt_states[r.clone()])
        .bind(&batch.pt_zips[r.clone()])
        .bind(&batch.pt_phones[r.clone()])
        .bind(&batch.pt_emails[r.clone()])
        .bind(&batch.pt_reg_dates[r.clone()])
        .bind(&ec_id_strs)
        .fetch_all(&ctx.pool)
        .await?;

        let real_patient_ids: Vec<uuid::Uuid> = patient_rows.iter().map(|row| row.0).collect();

        // Bulk UPDATE emergency contacts with the correct patient_ids — one round-trip.
        sqlx::query(
            "UPDATE vital_fold.emergency_contact ec \
             SET patient_id = u.patient_id \
             FROM UNNEST($1::uuid[], $2::uuid[]) AS u(ec_id, patient_id) \
             WHERE ec.emergency_contact_id = u.ec_id"
        )
        .bind(ec_ids_chunk)
        .bind(&real_patient_ids)
        .execute(&ctx.pool)
        .await?;

        // Populate context for downstream generators.
        ctx.patient_ids.extend_from_slice(&real_patient_ids);
        ctx.patient_home_clinics.extend_from_slice(&batch.home_clinic_indices[r.clone()]);
        ctx.patient_data.extend(
            real_patient_ids.into_iter()
                .zip(batch.pt_first_names[r.clone()].iter().cloned())
                .zip(batch.pt_last_names[r.clone()].iter().cloned())
                .zip(batch.pt_dobs[r.clone()].iter().copied())
                .map(|(((id, first), last), dob)| (id, first, last, dob))
        );
        ctx.counts.patients += chunk_size;
    }

    tracing::info!("Generated {} patients and {} emergency contacts", ctx.counts.patients, ctx.counts.emergency_contacts);

    Ok(())
}

/// No-op: emergency contact generation is now performed inside generate_patients.
pub async fn generate_emergency_contacts(_ctx: &mut SimulationContext) -> Result<(), AppError> {
    Ok(())
}

/// Generate patient demographics — chunked bulk INSERT via UNNEST.
///
/// Uses data cached in ctx.patient_data from generate_patients, avoiding a DB round-trip.
pub async fn generate_patient_demographics(ctx: &mut SimulationContext) -> Result<(), AppError> {
    use chrono::Local;

    let today = Local::now().naive_local().date();
    let n = ctx.patient_data.len();

    // Build all column vecs synchronously so rng is dropped before any await.
    let (pt_ids, first_names, last_names, dobs, ages, ssns, ethnicities_v, genders_v) = {
        use rand::thread_rng;

        let ethnicities = ["Caucasian", "African American", "Hispanic", "Asian", "Other"];
        let genders     = ["Male", "Female", "Other"];
        let mut rng     = thread_rng();

        let mut pt_ids:        Vec<uuid::Uuid>       = Vec::with_capacity(n);
        let mut first_names:   Vec<String>            = Vec::with_capacity(n);
        let mut last_names:    Vec<String>            = Vec::with_capacity(n);
        let mut dobs:          Vec<chrono::NaiveDate> = Vec::with_capacity(n);
        let mut ages:          Vec<i64>               = Vec::with_capacity(n);
        let mut ssns:          Vec<String>            = Vec::with_capacity(n);
        let mut ethnicities_v: Vec<String>            = Vec::with_capacity(n);
        let mut genders_v:     Vec<String>            = Vec::with_capacity(n);

        for (patient_id, first_name, last_name, dob) in &ctx.patient_data {
            let age = (today - *dob).num_days() / 365;
            let ssn = format!("{:03}-{:02}-{:04}", rng.gen_range(0..1000), rng.gen_range(0..100), rng.gen_range(0..10000));

            pt_ids.push(*patient_id);
            first_names.push(first_name.clone());
            last_names.push(last_name.clone());
            dobs.push(*dob);
            ages.push(age);
            ssns.push(ssn);
            ethnicities_v.push(ethnicities[rng.gen_range(0..ethnicities.len())].to_string());
            genders_v.push(genders[rng.gen_range(0..genders.len())].to_string());
        }

        (pt_ids, first_names, last_names, dobs, ages, ssns, ethnicities_v, genders_v)
    }; // rng dropped here before any await

    for chunk_start in (0..n).step_by(DSQL_BATCH_SIZE) {
        let chunk_end = (chunk_start + DSQL_BATCH_SIZE).min(n);
        let r = chunk_start..chunk_end;

        sqlx::query(
            "INSERT INTO vital_fold.patient_demographics \
             (patient_id, first_name, last_name, date_of_birth, age, ssn, ethnicity, birth_gender) \
             SELECT * FROM UNNEST($1::uuid[], $2::text[], $3::text[], $4::date[], $5::bigint[], $6::text[], $7::text[], $8::text[])"
        )
        .bind(&pt_ids[r.clone()])
        .bind(&first_names[r.clone()])
        .bind(&last_names[r.clone()])
        .bind(&dobs[r.clone()])
        .bind(&ages[r.clone()])
        .bind(&ssns[r.clone()])
        .bind(&ethnicities_v[r.clone()])
        .bind(&genders_v[r.clone()])
        .execute(&ctx.pool)
        .await?;

        ctx.counts.patient_demographics += chunk_end - chunk_start;
    }

    tracing::info!("Generated {} patient demographics", ctx.counts.patient_demographics);

    Ok(())
}

/// Generate patient insurance associations — chunked bulk INSERT via UNNEST.
pub async fn generate_patient_insurance(ctx: &mut SimulationContext) -> Result<(), AppError> {
    use chrono::Local;

    let today = Local::now().naive_local().date();
    let n     = ctx.patient_ids.len();

    // Build column vecs synchronously so rng is dropped before any await.
    let (pt_ids, plan_ids, policy_nums, starts, ends) = {
        use rand::thread_rng;

        let mut rng = thread_rng();

        let mut pt_ids:      Vec<uuid::Uuid>                = Vec::with_capacity(n);
        let mut plan_ids:    Vec<uuid::Uuid>                = Vec::with_capacity(n);
        let mut policy_nums: Vec<String>                    = Vec::with_capacity(n);
        let mut starts:      Vec<chrono::NaiveDate>         = Vec::with_capacity(n);
        let mut ends:        Vec<Option<chrono::NaiveDate>> = Vec::with_capacity(n);

        for &patient_id in &ctx.patient_ids {
            let plan_id = ctx.plan_ids[rng.gen_range(0..ctx.plan_ids.len())];
            let policy  = format!("POL-{:08X}", rng.gen::<u32>());
            let end     = if rng.gen_bool(0.2) {
                Some(today - chrono::TimeDelta::days(rng.gen_range(30..365)))
            } else {
                None
            };

            pt_ids.push(patient_id);
            plan_ids.push(plan_id);
            policy_nums.push(policy);
            starts.push(today);
            ends.push(end);
        }

        (pt_ids, plan_ids, policy_nums, starts, ends)
    }; // rng dropped here before any await

    for chunk_start in (0..n).step_by(DSQL_BATCH_SIZE) {
        let chunk_end = (chunk_start + DSQL_BATCH_SIZE).min(n);
        let r = chunk_start..chunk_end;

        sqlx::query(
            "INSERT INTO vital_fold.patient_insurance \
             (patient_id, insurance_plan_id, policy_number, coverage_start_date, coverage_end_date) \
             SELECT * FROM UNNEST($1::uuid[], $2::uuid[], $3::text[], $4::date[], $5::date[])"
        )
        .bind(&pt_ids[r.clone()])
        .bind(&plan_ids[r.clone()])
        .bind(&policy_nums[r.clone()])
        .bind(&starts[r.clone()])
        .bind(&ends[r.clone()])
        .execute(&ctx.pool)
        .await?;

        ctx.counts.patient_insurance += chunk_end - chunk_start;
    }

    tracing::info!("Generated {} patient insurance links", ctx.counts.patient_insurance);

    Ok(())
}
