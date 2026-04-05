/// Generate patient_visit and patient_vitals rows in Aurora DSQL.
///
/// Called during Phase 1 (POST /populate) after appointments are generated.
/// Each appointment gets one patient_visit row and one patient_vitals row.
///
/// Phase 2 (POST /simulate) later reads from these Aurora tables via JOIN
/// and writes the data to DynamoDB.

use crate::db::DbPool;
use crate::errors::AppError;
use chrono::{NaiveDateTime, TimeDelta, Utc};
use uuid::Uuid;

use super::SimulationContext;

/// Aurora DSQL maximum rows per transaction statement.
const DSQL_BATCH_SIZE: usize = 2500;

/// Row returned when querying appointments for visit generation.
#[derive(sqlx::FromRow)]
struct AppointmentRow {
    appointment_id: Uuid,
    patient_id: Uuid,
    clinic_id: i64,
    provider_id: i64,
    appointment_datetime: NaiveDateTime,
}

/// Generate one patient_visit row and one patient_vitals row per appointment.
///
/// Queries all appointments, generates visit + vitals data, and bulk-inserts into
/// vital_fold.patient_visit (with RETURNING to capture UUIDs) then vital_fold.patient_vitals.
pub async fn generate_patient_visits(ctx: &mut SimulationContext) -> Result<(), AppError> {
    use rand::{rng, Rng};

    // Query all appointments from Aurora.
    let appointments: Vec<AppointmentRow> = sqlx::query_as(
        "SELECT appointment_id, patient_id, clinic_id, provider_id, appointment_datetime \
         FROM vital_fold.appointment"
    )
    .fetch_all(&ctx.pool)
    .await?;

    let total = appointments.len();
    if total == 0 {
        tracing::warn!("No appointments found — skipping visit generation");
        return Ok(());
    }

    // Build all visit + vital data synchronously — rng dropped before any await.
    let (
        appointment_ids, patient_ids, clinic_ids, provider_ids,
        checkin_times, checkout_times, provider_seen_times,
        ekg_usages, copays, creation_times, expiry_epochs,
        heights, weights, blood_pressures, heart_rates,
        temperatures, oxygen_saturations,
    ) = {
        let mut rng = rng();
        let now = Utc::now().naive_utc();
        let expiry = (Utc::now() + TimeDelta::days(90)).timestamp();

        let mut appointment_ids:     Vec<Uuid>          = Vec::with_capacity(total);
        let mut patient_ids:         Vec<Uuid>          = Vec::with_capacity(total);
        let mut clinic_ids:          Vec<i64>           = Vec::with_capacity(total);
        let mut provider_ids:        Vec<i64>           = Vec::with_capacity(total);
        let mut checkin_times:       Vec<NaiveDateTime>  = Vec::with_capacity(total);
        let mut checkout_times:      Vec<Option<NaiveDateTime>> = Vec::with_capacity(total);
        let mut provider_seen_times: Vec<Option<NaiveDateTime>> = Vec::with_capacity(total);
        let mut ekg_usages:          Vec<bool>          = Vec::with_capacity(total);
        let mut copays:              Vec<i32>           = Vec::with_capacity(total);
        let mut creation_times:      Vec<NaiveDateTime>  = Vec::with_capacity(total);
        let mut expiry_epochs:       Vec<i64>           = Vec::with_capacity(total);
        let mut heights:             Vec<f64>           = Vec::with_capacity(total);
        let mut weights:             Vec<f64>           = Vec::with_capacity(total);
        let mut blood_pressures:     Vec<String>        = Vec::with_capacity(total);
        let mut heart_rates:         Vec<i32>           = Vec::with_capacity(total);
        let mut temperatures:        Vec<f64>           = Vec::with_capacity(total);
        let mut oxygen_saturations:  Vec<f64>           = Vec::with_capacity(total);

        for appt in &appointments {
            appointment_ids.push(appt.appointment_id);
            // Checkin: 5–15 min before scheduled appointment
            let early_arrival        = rng.random_range(5..=15i64);
            // Provider seen: 0–5 min after scheduled time
            let provider_seen_offset = rng.random_range(0..=5i64);
            // Checkout: 15–30 min after scheduled time
            let checkout_offset      = rng.random_range(15..=30i64);

            patient_ids.push(appt.patient_id);
            clinic_ids.push(appt.clinic_id);
            provider_ids.push(appt.provider_id);
            checkin_times.push(appt.appointment_datetime - TimeDelta::minutes(early_arrival));
            checkout_times.push(Some(appt.appointment_datetime + TimeDelta::minutes(checkout_offset)));
            provider_seen_times.push(Some(appt.appointment_datetime + TimeDelta::minutes(provider_seen_offset)));
            let ekg = rng.random_bool(0.2);
            let copay = if ekg {
                rng.random_range(150..350) // EKG visit: higher copay
            } else {
                rng.random_range(20..150)  // Standard visit
            };
            ekg_usages.push(ekg);
            copays.push(copay);
            creation_times.push(now);
            expiry_epochs.push(expiry);

            // Vitals
            heights.push(rng.random_range(60.0..78.0f64));
            weights.push(rng.random_range(120.0..220.0f64));
            let sys = rng.random_range(100..160u32);
            let dia = rng.random_range(60..100u32);
            blood_pressures.push(format!("{}/{}", sys, dia));
            heart_rates.push(rng.random_range(50..120i32));
            temperatures.push(97.0 + (rng.random::<f64>() * 2.5));
            oxygen_saturations.push(rng.random_range(95.0..100.0f64));
        }

        (
            appointment_ids, patient_ids, clinic_ids, provider_ids,
            checkin_times, checkout_times, provider_seen_times,
            ekg_usages, copays, creation_times, expiry_epochs,
            heights, weights, blood_pressures, heart_rates,
            temperatures, oxygen_saturations,
        )
    }; // rng dropped here

    // INSERT 1: patient_visit rows (with RETURNING to capture generated UUIDs).
    let mut visit_ids: Vec<Uuid> = Vec::with_capacity(total);

    for chunk_start in (0..total).step_by(DSQL_BATCH_SIZE) {
        let chunk_end = (chunk_start + DSQL_BATCH_SIZE).min(total);
        let r = chunk_start..chunk_end;

        let rows: Vec<(Uuid,)> = sqlx::query_as(
            "INSERT INTO vital_fold.patient_visit \
             (appointment_id, patient_id, clinic_id, provider_id, checkin_time, checkout_time, \
              provider_seen_time, ekg_usage, estimated_copay, creation_time, record_expiration_epoch) \
             SELECT * FROM UNNEST(\
                $1::uuid[], $2::uuid[], $3::bigint[], $4::bigint[], $5::timestamp[], $6::timestamp[], \
                $7::timestamp[], $8::boolean[], $9::numeric[], $10::timestamp[], $11::bigint[]) \
             RETURNING patient_visit_id"
        )
        .bind(&appointment_ids[r.clone()])
        .bind(&patient_ids[r.clone()])
        .bind(&clinic_ids[r.clone()])
        .bind(&provider_ids[r.clone()])
        .bind(&checkin_times[r.clone()])
        .bind(&checkout_times[r.clone()])
        .bind(&provider_seen_times[r.clone()])
        .bind(&ekg_usages[r.clone()])
        .bind(&copays[r.clone()])
        .bind(&creation_times[r.clone()])
        .bind(&expiry_epochs[r.clone()])
        .fetch_all(&ctx.pool)
        .await?;

        ctx.counts.patient_visits += rows.len();
        visit_ids.extend(rows.into_iter().map(|(id,)| id));
    }

    // INSERT 2: patient_vitals rows (using the returned visit UUIDs as PK).
    for chunk_start in (0..total).step_by(DSQL_BATCH_SIZE) {
        let chunk_end = (chunk_start + DSQL_BATCH_SIZE).min(total);
        let r = chunk_start..chunk_end;

        let result = sqlx::query(
            "INSERT INTO vital_fold.patient_vitals \
             (patient_visit_id, patient_id, clinic_id, provider_id, \
              height, weight, blood_pressure, heart_rate, temperature, oxygen_saturation, \
              creation_time, record_expiration_epoch) \
             SELECT * FROM UNNEST(\
                $1::uuid[], $2::uuid[], $3::bigint[], $4::bigint[], \
                $5::numeric[], $6::numeric[], $7::text[], $8::int[], $9::numeric[], $10::numeric[], \
                $11::timestamp[], $12::bigint[])"
        )
        .bind(&visit_ids[r.clone()])
        .bind(&patient_ids[r.clone()])
        .bind(&clinic_ids[r.clone()])
        .bind(&provider_ids[r.clone()])
        .bind(&heights[r.clone()])
        .bind(&weights[r.clone()])
        .bind(&blood_pressures[r.clone()])
        .bind(&heart_rates[r.clone()])
        .bind(&temperatures[r.clone()])
        .bind(&oxygen_saturations[r.clone()])
        .bind(&creation_times[r.clone()])
        .bind(&expiry_epochs[r.clone()])
        .execute(&ctx.pool)
        .await?;

        ctx.counts.patient_vitals += result.rows_affected() as usize;
    }

    tracing::info!(
        "Generated {} patient_visit + {} patient_vitals rows",
        ctx.counts.patient_visits, ctx.counts.patient_vitals
    );

    Ok(())
}

/// Generate patient_visit + patient_vitals for a set of appointments (standalone).
/// Used by `run_date_range_simulate` and `run_populate_dynamic`.
/// Returns (visit_ids, ekg_flags, vitals_count). `ekg_flags[i]` aligns 1:1
/// with `appointments[i]` so the downstream RVU generator can bill CPT 93000
/// for visits where the EKG was performed.
pub async fn generate_visits_for_appointments(
    pool: &DbPool,
    appointments: &[(Uuid, Uuid, i64, i64, NaiveDateTime)],
) -> Result<(Vec<Uuid>, Vec<bool>, usize), AppError> {
    use rand::{rng, Rng};

    let total = appointments.len();
    if total == 0 { return Ok((Vec::new(), Vec::new(), 0)); }

    let (
        appointment_ids, patient_ids, clinic_ids, provider_ids,
        checkin_times, checkout_times, provider_seen_times,
        ekg_usages, copays, creation_times, expiry_epochs,
        heights, weights, blood_pressures, heart_rates,
        temperatures, oxygen_saturations,
    ) = {
        let mut rng = rng();
        let now = Utc::now().naive_utc();
        let expiry = (Utc::now() + TimeDelta::days(90)).timestamp();

        let mut appointment_ids:     Vec<Uuid>                 = Vec::with_capacity(total);
        let mut patient_ids:         Vec<Uuid>                 = Vec::with_capacity(total);
        let mut clinic_ids:          Vec<i64>                  = Vec::with_capacity(total);
        let mut provider_ids:        Vec<i64>                  = Vec::with_capacity(total);
        let mut checkin_times:       Vec<NaiveDateTime>         = Vec::with_capacity(total);
        let mut checkout_times:      Vec<Option<NaiveDateTime>> = Vec::with_capacity(total);
        let mut provider_seen_times: Vec<Option<NaiveDateTime>> = Vec::with_capacity(total);
        let mut ekg_usages:          Vec<bool>                 = Vec::with_capacity(total);
        let mut copays:              Vec<i32>                  = Vec::with_capacity(total);
        let mut creation_times:      Vec<NaiveDateTime>         = Vec::with_capacity(total);
        let mut expiry_epochs:       Vec<i64>                  = Vec::with_capacity(total);
        let mut heights:             Vec<f64>                  = Vec::with_capacity(total);
        let mut weights:             Vec<f64>                  = Vec::with_capacity(total);
        let mut blood_pressures:     Vec<String>               = Vec::with_capacity(total);
        let mut heart_rates:         Vec<i32>                  = Vec::with_capacity(total);
        let mut temperatures:        Vec<f64>                  = Vec::with_capacity(total);
        let mut oxygen_saturations:  Vec<f64>                  = Vec::with_capacity(total);

        // appointments tuple: (appt_id, patient_id, clinic_id, provider_id, appt_dt)
        for &(appt_id, patient_id, clinic_id, provider_id, appt_dt) in appointments {
            appointment_ids.push(appt_id);
            // Checkin: 5–15 min before scheduled appointment
            let early_arrival        = rng.random_range(5..=15i64);
            // Provider seen: 0–5 min after scheduled time
            let provider_seen_offset = rng.random_range(0..=5i64);
            // Checkout: 15–30 min after scheduled time
            let checkout_offset      = rng.random_range(15..=30i64);

            patient_ids.push(patient_id);
            clinic_ids.push(clinic_id);
            provider_ids.push(provider_id);
            checkin_times.push(appt_dt - TimeDelta::minutes(early_arrival));
            checkout_times.push(Some(appt_dt + TimeDelta::minutes(checkout_offset)));
            provider_seen_times.push(Some(appt_dt + TimeDelta::minutes(provider_seen_offset)));
            let ekg = rng.random_bool(0.2);
            let copay = if ekg {
                rng.random_range(150..350) // EKG visit: higher copay
            } else {
                rng.random_range(20..150)  // Standard visit
            };
            ekg_usages.push(ekg);
            copays.push(copay);
            creation_times.push(now);
            expiry_epochs.push(expiry);

            // Vitals
            heights.push(rng.random_range(60.0..78.0f64));
            weights.push(rng.random_range(120.0..220.0f64));
            let sys = rng.random_range(100..160u32);
            let dia = rng.random_range(60..100u32);
            blood_pressures.push(format!("{}/{}", sys, dia));
            heart_rates.push(rng.random_range(50..120i32));
            temperatures.push(97.0 + (rng.random::<f64>() * 2.5));
            oxygen_saturations.push(rng.random_range(95.0..100.0f64));
        }

        (
            appointment_ids, patient_ids, clinic_ids, provider_ids,
            checkin_times, checkout_times, provider_seen_times,
            ekg_usages, copays, creation_times, expiry_epochs,
            heights, weights, blood_pressures, heart_rates,
            temperatures, oxygen_saturations,
        )
    };

    // INSERT 1: patient_visit rows (with RETURNING to capture generated UUIDs).
    let mut visit_ids: Vec<Uuid> = Vec::with_capacity(total);

    for chunk_start in (0..total).step_by(DSQL_BATCH_SIZE) {
        let chunk_end = (chunk_start + DSQL_BATCH_SIZE).min(total);
        let r = chunk_start..chunk_end;

        let rows: Vec<(Uuid,)> = sqlx::query_as(
            "INSERT INTO vital_fold.patient_visit \
             (appointment_id, patient_id, clinic_id, provider_id, checkin_time, checkout_time, \
              provider_seen_time, ekg_usage, estimated_copay, creation_time, record_expiration_epoch) \
             SELECT * FROM UNNEST(\
                $1::uuid[], $2::uuid[], $3::bigint[], $4::bigint[], $5::timestamp[], $6::timestamp[], \
                $7::timestamp[], $8::boolean[], $9::numeric[], $10::timestamp[], $11::bigint[]) \
             RETURNING patient_visit_id"
        )
        .bind(&appointment_ids[r.clone()])
        .bind(&patient_ids[r.clone()])
        .bind(&clinic_ids[r.clone()])
        .bind(&provider_ids[r.clone()])
        .bind(&checkin_times[r.clone()])
        .bind(&checkout_times[r.clone()])
        .bind(&provider_seen_times[r.clone()])
        .bind(&ekg_usages[r.clone()])
        .bind(&copays[r.clone()])
        .bind(&creation_times[r.clone()])
        .bind(&expiry_epochs[r.clone()])
        .fetch_all(pool)
        .await?;

        visit_ids.extend(rows.into_iter().map(|(id,)| id));
    }

    // INSERT 2: patient_vitals rows (using the returned visit UUIDs as PK).
    let mut vitals_count = 0usize;

    for chunk_start in (0..total).step_by(DSQL_BATCH_SIZE) {
        let chunk_end = (chunk_start + DSQL_BATCH_SIZE).min(total);
        let r = chunk_start..chunk_end;

        let result = sqlx::query(
            "INSERT INTO vital_fold.patient_vitals \
             (patient_visit_id, patient_id, clinic_id, provider_id, \
              height, weight, blood_pressure, heart_rate, temperature, oxygen_saturation, \
              creation_time, record_expiration_epoch) \
             SELECT * FROM UNNEST(\
                $1::uuid[], $2::uuid[], $3::bigint[], $4::bigint[], \
                $5::numeric[], $6::numeric[], $7::text[], $8::int[], $9::numeric[], $10::numeric[], \
                $11::timestamp[], $12::bigint[])"
        )
        .bind(&visit_ids[r.clone()])
        .bind(&patient_ids[r.clone()])
        .bind(&clinic_ids[r.clone()])
        .bind(&provider_ids[r.clone()])
        .bind(&heights[r.clone()])
        .bind(&weights[r.clone()])
        .bind(&blood_pressures[r.clone()])
        .bind(&heart_rates[r.clone()])
        .bind(&temperatures[r.clone()])
        .bind(&oxygen_saturations[r.clone()])
        .bind(&creation_times[r.clone()])
        .bind(&expiry_epochs[r.clone()])
        .execute(pool)
        .await?;

        vitals_count += result.rows_affected() as usize;
    }

    tracing::info!(
        "Generated {} patient_visit + {} patient_vitals rows for date range",
        visit_ids.len(), vitals_count
    );
    Ok((visit_ids, ekg_usages, vitals_count))
}
