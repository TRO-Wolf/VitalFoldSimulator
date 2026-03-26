/// Generate appointments for patients at clinics (Aurora DSQL only).
///
/// Each patient gets N appointments distributed across random clinics and providers.
/// Appointments are bulk-inserted in chunks of DSQL_BATCH_SIZE to stay under Aurora
/// DSQL's 3000-row per-transaction limit.
///
/// DynamoDB writes are NOT performed here. They are performed by `run_simulate` in mod.rs,
/// which queries today's visits via a JOIN on patient_visit + patient_vitals and writes
/// to both DynamoDB tables.

use crate::db::DbPool;
use crate::errors::AppError;
use chrono::{NaiveDate, TimeDelta, NaiveDateTime};
use uuid::Uuid;

/// Max retries for DynamoDB throttling (exponential backoff: 50ms, 100ms, 200ms, 400ms, 800ms).
const DYNAMO_MAX_RETRIES: u32 = 5;
const DYNAMO_RETRY_BASE_MS: u64 = 50;

/// Returns `true` if the DynamoDB error is a throttling/throughput error worth retrying.
fn is_throttle_error<E: std::fmt::Debug>(err: &E) -> bool {
    let s = format!("{err:?}");
    s.contains("ThrottlingException") || s.contains("ProvisionedThroughputExceeded")
}

use super::SimulationContext;

/// Aurora DSQL maximum rows per transaction statement.
const DSQL_BATCH_SIZE: usize = 2500;

const APPOINTMENT_REASONS: &[&str] = &[
    "Annual checkup",
    "Chest pain evaluation",
    "Blood pressure check",
    "Follow-up visit",
    "New patient visit",
];

/// Generate all appointments for all patients in chunked bulk inserts.
///
/// Appointments are distributed randomly across `config.start_date .. config.end_date`
/// (both inclusive). Each appointment gets a random hour between 9 AM and 4:59 PM.
pub async fn generate_appointments(ctx: &mut SimulationContext) -> Result<(), AppError> {
    use rand::{thread_rng, Rng};

    let span = (ctx.config.end_date - ctx.config.start_date).num_days() + 1;
    let total = ctx.patient_ids.len() * ctx.config.appointments_per_patient;

    // Build all appointment data synchronously — rng dropped before any await.
    let (
        pt_ids, provider_ids, clinic_ids,
        appt_dts, reasons,
    ) = {
        let mut rng = thread_rng();

        let mut pt_ids:       Vec<Uuid>            = Vec::with_capacity(total);
        let mut provider_ids: Vec<Uuid>            = Vec::with_capacity(total);
        let mut clinic_ids:   Vec<Uuid>            = Vec::with_capacity(total);
        let mut appt_dts:     Vec<NaiveDateTime>   = Vec::with_capacity(total);
        let mut reasons:      Vec<String>          = Vec::with_capacity(total);

        for &patient_id in &ctx.patient_ids {
            for _ in 0..ctx.config.appointments_per_patient {
                let clinic_id   = ctx.clinic_ids[rng.gen_range(0..ctx.clinic_ids.len())];
                let provider_id = ctx.provider_ids[rng.gen_range(0..ctx.provider_ids.len())];
                let day_offset  = rng.gen_range(0..span);
                let hour        = rng.gen_range(9..17u32);
                let minute      = rng.gen_range(0..60u32);
                let reason      = APPOINTMENT_REASONS[rng.gen_range(0..APPOINTMENT_REASONS.len())];

                let appt_dt = NaiveDateTime::new(
                    ctx.config.start_date + TimeDelta::days(day_offset),
                    chrono::NaiveTime::from_hms_opt(hour, minute, 0)
                        .expect("hour 9..16 and minute 0..59 are always valid"),
                );

                pt_ids.push(patient_id);
                provider_ids.push(provider_id);
                clinic_ids.push(clinic_id);
                appt_dts.push(appt_dt);
                reasons.push(reason.to_string());
            }
        }

        (pt_ids, provider_ids, clinic_ids, appt_dts, reasons)
    }; // rng dropped here

    // Bulk-insert appointments in DSQL_BATCH_SIZE chunks.
    // RETURNING is not needed here — appointment_ids for DynamoDB writes are
    // fetched on-demand by run_simulate when querying today's appointments.
    for chunk_start in (0..total).step_by(DSQL_BATCH_SIZE) {
        let chunk_end = (chunk_start + DSQL_BATCH_SIZE).min(total);
        let r = chunk_start..chunk_end;

        let result = sqlx::query(
            "INSERT INTO vital_fold.appointment \
             (patient_id, provider_id, clinic_id, appointment_date, reason_for_visit) \
             SELECT * FROM UNNEST($1::uuid[], $2::uuid[], $3::uuid[], $4::timestamp[], $5::text[])"
        )
        .bind(&pt_ids[r.clone()])
        .bind(&provider_ids[r.clone()])
        .bind(&clinic_ids[r.clone()])
        .bind(&appt_dts[r.clone()])
        .bind(&reasons[r.clone()])
        .execute(&ctx.pool)
        .await?;

        ctx.counts.appointments += result.rows_affected() as usize;
    }

    tracing::info!("Generated {} appointments", ctx.counts.appointments);

    Ok(())
}

/// Generate appointments for a specific date range with a fixed number per day.
///
/// Unlike `generate_appointments_for_range` which distributes N appointments per patient
/// randomly across the range, this function generates exactly `appointments_per_day`
/// appointments on each day — each randomly assigned to a patient, clinic, and provider.
///
/// Uses `INSERT ... RETURNING` to capture generated UUIDs for immediate downstream use.
pub async fn generate_appointments_by_day(
    pool: &DbPool,
    patient_ids: &[Uuid],
    provider_ids: &[Uuid],
    clinic_ids: &[Uuid],
    start_date: NaiveDate,
    end_date: NaiveDate,
    appointments_per_day: usize,
) -> Result<Vec<(Uuid, Uuid, Uuid, Uuid, NaiveDateTime)>, AppError> {
    use rand::{thread_rng, Rng};

    let span = (end_date - start_date).num_days() + 1;
    let total = appointments_per_day * span as usize;
    let mut all_results: Vec<(Uuid, Uuid, Uuid, Uuid, NaiveDateTime)> = Vec::with_capacity(total);

    // Build all appointment data synchronously — rng dropped before any await.
    let (pt_ids, prov_ids, cl_ids, appt_dts, reasons) = {
        let mut rng = thread_rng();

        let mut pt_ids:   Vec<Uuid>          = Vec::with_capacity(total);
        let mut prov_ids: Vec<Uuid>          = Vec::with_capacity(total);
        let mut cl_ids:   Vec<Uuid>          = Vec::with_capacity(total);
        let mut appt_dts: Vec<NaiveDateTime> = Vec::with_capacity(total);
        let mut reasons:  Vec<String>        = Vec::with_capacity(total);

        for day_offset in 0..span {
            let date = start_date + TimeDelta::days(day_offset);
            for _ in 0..appointments_per_day {
                let patient_id  = patient_ids[rng.gen_range(0..patient_ids.len())];
                let clinic_id   = clinic_ids[rng.gen_range(0..clinic_ids.len())];
                let provider_id = provider_ids[rng.gen_range(0..provider_ids.len())];
                let hour        = rng.gen_range(9..17u32);
                let minute      = rng.gen_range(0..60u32);
                let reason      = APPOINTMENT_REASONS[rng.gen_range(0..APPOINTMENT_REASONS.len())];

                let appt_dt = NaiveDateTime::new(
                    date,
                    chrono::NaiveTime::from_hms_opt(hour, minute, 0)
                        .expect("hour 9..16 and minute 0..59 are always valid"),
                );

                pt_ids.push(patient_id);
                prov_ids.push(provider_id);
                cl_ids.push(clinic_id);
                appt_dts.push(appt_dt);
                reasons.push(reason.to_string());
            }
        }

        (pt_ids, prov_ids, cl_ids, appt_dts, reasons)
    }; // rng dropped here

    // Bulk-insert in DSQL_BATCH_SIZE chunks, capturing generated IDs via RETURNING.
    for chunk_start in (0..total).step_by(DSQL_BATCH_SIZE) {
        let chunk_end = (chunk_start + DSQL_BATCH_SIZE).min(total);
        let r = chunk_start..chunk_end;

        let rows: Vec<(Uuid, Uuid, Uuid, Uuid, NaiveDateTime)> = sqlx::query_as(
            "INSERT INTO vital_fold.appointment \
             (patient_id, provider_id, clinic_id, appointment_date, reason_for_visit) \
             SELECT * FROM UNNEST($1::uuid[], $2::uuid[], $3::uuid[], $4::timestamp[], $5::text[]) \
             RETURNING appointment_id, patient_id, clinic_id, provider_id, appointment_date"
        )
        .bind(&pt_ids[r.clone()])
        .bind(&prov_ids[r.clone()])
        .bind(&cl_ids[r.clone()])
        .bind(&appt_dts[r.clone()])
        .bind(&reasons[r.clone()])
        .fetch_all(pool)
        .await?;

        all_results.extend(rows);
    }

    tracing::info!(
        "Generated {} appointments ({}/day) for date range {} to {}",
        all_results.len(), appointments_per_day, start_date, end_date
    );

    Ok(all_results)
}

/// Write visit metadata to DynamoDB patient_visit table.
/// Called by `run_simulate` in mod.rs for visits whose checkin_time matches today.
///
/// Sort key is "clinic_id#visit_id" to ensure uniqueness per dynamo.json schema.
/// Retries up to `DYNAMO_MAX_RETRIES` times on throttling errors with exponential
/// backoff + jitter. Returns `true` on success, `false` on permanent error.
pub(super) async fn write_patient_visit(
    dynamo: &aws_sdk_dynamodb::Client,
    visit: &crate::models::PatientVisitWithVitals,
) -> bool {
    use aws_sdk_dynamodb::types::AttributeValue;

    let checkin_time  = visit.checkin_time.format("%Y-%m-%dT%H:%M:%SZ").to_string();
    let checkout_time = visit.checkout_time.map(|t| t.format("%Y-%m-%dT%H:%M:%SZ").to_string()).unwrap_or_default();
    let provider_seen = visit.provider_seen_time.map(|t| t.format("%Y-%m-%dT%H:%M:%SZ").to_string()).unwrap_or_default();

    let sort_key = format!("{}#{}", visit.clinic_id, visit.patient_visit_id);

    for attempt in 0..=DYNAMO_MAX_RETRIES {
        let result = dynamo
            .put_item()
            .table_name("patient_visit")
            .item("patient_id",              AttributeValue::S(visit.patient_id.to_string()))
            .item("clinic_id",               AttributeValue::S(sort_key.clone()))
            .item("provider_id",             AttributeValue::S(visit.provider_id.to_string()))
            .item("checkin_time",            AttributeValue::S(checkin_time.clone()))
            .item("checkout_time",           AttributeValue::S(checkout_time.clone()))
            .item("provider_seen_time",      AttributeValue::S(provider_seen.clone()))
            .item("ekg_usage",               AttributeValue::Bool(visit.ekg_usage))
            .item("estimated_copay",         AttributeValue::N(visit.estimated_copay.to_string()))
            .item("creation_time",           AttributeValue::N(visit.creation_time.and_utc().timestamp().to_string()))
            .item("record_expiration_epoch", AttributeValue::N(visit.record_expiration_epoch.to_string()))
            .send()
            .await;

        match result {
            Ok(_) => return true,
            Err(e) if attempt < DYNAMO_MAX_RETRIES && is_throttle_error(&e) => {
                let delay_ms = {
                    use rand::Rng;
                    let base = DYNAMO_RETRY_BASE_MS * 2u64.pow(attempt);
                    base / 2 + rand::rng().random_range(0..=base / 2)
                };
                tracing::debug!(
                    "DynamoDB patient_visit throttled, retry {}/{} in {}ms",
                    attempt + 1, DYNAMO_MAX_RETRIES, delay_ms
                );
                tokio::time::sleep(std::time::Duration::from_millis(delay_ms)).await;
            }
            Err(e) => {
                tracing::error!("DynamoDB patient_visit write failed: {:?}", e);
                return false;
            }
        }
    }
    false
}

/// Write vitals data to DynamoDB patient_vitals table.
/// Called by `run_simulate` in mod.rs alongside `write_patient_visit`.
///
/// Sort key is "clinic_id#visit_id" to match the patient_visit table pattern.
/// Retries up to `DYNAMO_MAX_RETRIES` times on throttling errors with exponential
/// backoff + jitter. Returns `true` on success, `false` on permanent error.
pub(super) async fn write_patient_vitals(
    dynamo: &aws_sdk_dynamodb::Client,
    visit: &crate::models::PatientVisitWithVitals,
) -> bool {
    use aws_sdk_dynamodb::types::AttributeValue;

    let sort_key = format!("{}#{}", visit.clinic_id, visit.patient_visit_id);

    for attempt in 0..=DYNAMO_MAX_RETRIES {
        let result = dynamo
            .put_item()
            .table_name("patient_vitals")
            .item("patient_id",              AttributeValue::S(visit.patient_id.to_string()))
            .item("clinic_id",               AttributeValue::S(sort_key.clone()))
            .item("provider_id",             AttributeValue::S(visit.provider_id.to_string()))
            .item("visit_id",                AttributeValue::S(visit.patient_visit_id.to_string()))
            .item("height",                  AttributeValue::N(visit.height.to_string()))
            .item("weight",                  AttributeValue::N(visit.weight.to_string()))
            .item("blood_pressure",          AttributeValue::S(visit.blood_pressure.clone()))
            .item("heart_rate",              AttributeValue::N(visit.heart_rate.to_string()))
            .item("temperature",             AttributeValue::N(visit.temperature.to_string()))
            .item("oxygen",                  AttributeValue::N(visit.oxygen_saturation.to_string()))
            .item("creation_time",           AttributeValue::N(visit.creation_time.and_utc().timestamp().to_string()))
            .item("record_expiration_epoch", AttributeValue::N(visit.record_expiration_epoch.to_string()))
            .send()
            .await;

        match result {
            Ok(_) => return true,
            Err(e) if attempt < DYNAMO_MAX_RETRIES && is_throttle_error(&e) => {
                let delay_ms = {
                    use rand::Rng;
                    let base = DYNAMO_RETRY_BASE_MS * 2u64.pow(attempt);
                    base / 2 + rand::rng().random_range(0..=base / 2)
                };
                tracing::debug!(
                    "DynamoDB patient_vitals throttled, retry {}/{} in {}ms",
                    attempt + 1, DYNAMO_MAX_RETRIES, delay_ms
                );
                tokio::time::sleep(std::time::Duration::from_millis(delay_ms)).await;
            }
            Err(e) => {
                tracing::error!("DynamoDB patient_vitals write failed: {:?}", e);
                return false;
            }
        }
    }
    false
}
