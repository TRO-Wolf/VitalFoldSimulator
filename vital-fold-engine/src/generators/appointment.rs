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

/// Distribute `total` items across buckets proportional to `weights`.
/// Uses largest-remainder method to ensure the sum equals `total` exactly.
fn distribute_by_weight(total: usize, weights: &[u32], weight_sum: u32) -> Vec<usize> {
    if weight_sum == 0 || weights.is_empty() {
        return vec![0; weights.len()];
    }
    let mut result: Vec<usize> = weights.iter()
        .map(|&w| ((total as u64 * w as u64) / weight_sum as u64) as usize)
        .collect();
    let assigned: usize = result.iter().sum();
    let mut remainder: usize = total.saturating_sub(assigned);

    // Distribute leftover 1-by-1 to the buckets with the largest fractional part.
    if remainder > 0 {
        let mut fractionals: Vec<(usize, f64)> = weights.iter().enumerate()
            .map(|(i, &w)| {
                let exact = total as f64 * w as f64 / weight_sum as f64;
                (i, exact - result[i] as f64)
            })
            .collect();
        fractionals.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        for (idx, _) in fractionals {
            if remainder == 0 { break; }
            result[idx] += 1;
            remainder -= 1;
        }
    }
    result
}

/// Aurora DSQL maximum rows per transaction statement.
const DSQL_BATCH_SIZE: usize = 2500;

/// Pre-build a lookup: clinic_index → Vec<patient_ids> for patients assigned to that metro.
/// Used to bias patient selection toward the clinic's geographic area.
fn build_clinic_patient_map(
    patient_ids: &[Uuid],
    patient_home_clinics: &[usize],
) -> std::collections::HashMap<usize, Vec<Uuid>> {
    let mut map: std::collections::HashMap<usize, Vec<Uuid>> = std::collections::HashMap::new();
    for (i, &clinic_idx) in patient_home_clinics.iter().enumerate() {
        map.entry(clinic_idx).or_default().push(patient_ids[i]);
    }
    map
}

const APPOINTMENT_REASONS: &[&str] = &[
    "Annual checkup",
    "Chest pain evaluation",
    "Blood pressure check",
    "Follow-up visit",
    "New patient visit",
];

/// Generate appointments by filling every provider's schedule.
///
/// Each provider gets 36 appointments per day (one per 15-minute slot from 8:00–16:45)
/// at their assigned clinic. Patients are drawn randomly (biased 70% toward home clinic).
/// Appointments span `config.start_date .. config.end_date` (both inclusive).
pub async fn generate_appointments(ctx: &mut SimulationContext) -> Result<(), AppError> {
    use rand::{rng, Rng};
    use super::SLOTS_PER_PROVIDER;

    let span = (ctx.config.end_date - ctx.config.start_date).num_days() + 1;
    let num_providers = ctx.provider_ids.len();
    let total = num_providers * SLOTS_PER_PROVIDER * span as usize;

    // Pre-build clinic→patients lookup for geographic bias (O(n) once, not per appointment).
    let clinic_patients = build_clinic_patient_map(&ctx.patient_ids, &ctx.patient_home_clinics);

    // Build all appointment data synchronously — rng dropped before any await.
    let (
        pt_ids, provider_ids, clinic_ids,
        appt_dts, reasons,
    ) = {
        let mut rng = rng();

        let mut pt_ids:       Vec<Uuid>            = Vec::with_capacity(total);
        let mut provider_ids: Vec<i64>             = Vec::with_capacity(total);
        let mut clinic_ids:   Vec<i64>             = Vec::with_capacity(total);
        let mut appt_dts:     Vec<NaiveDateTime>   = Vec::with_capacity(total);
        let mut reasons:      Vec<String>          = Vec::with_capacity(total);

        for day_offset in 0..span {
            let date = ctx.config.start_date + TimeDelta::days(day_offset);

            for (prov_idx, &provider_id) in ctx.provider_ids.iter().enumerate() {
                // Provider's primary clinic from proportional assignment
                let clinic_idx = if !ctx.provider_clinic_assignments.is_empty() {
                    ctx.provider_clinic_assignments[prov_idx]
                } else {
                    rng.random_range(0..ctx.clinic_ids.len())
                };
                let clinic_id = ctx.clinic_ids[clinic_idx % ctx.clinic_ids.len()];

                // Fill all 36 time slots for this provider on this day
                for hour in 8u32..17u32 {
                    for quarter in 0u32..4u32 {
                        let minute = quarter * 15;
                        // 70% local patient from same metro, 30% any patient
                        let patient_id = if rng.random_bool(0.7) {
                            clinic_patients.get(&clinic_idx)
                                .filter(|pts| !pts.is_empty())
                                .map(|pts| pts[rng.random_range(0..pts.len())])
                                .unwrap_or_else(|| ctx.patient_ids[rng.random_range(0..ctx.patient_ids.len())])
                        } else {
                            ctx.patient_ids[rng.random_range(0..ctx.patient_ids.len())]
                        };
                        let reason = APPOINTMENT_REASONS[rng.random_range(0..APPOINTMENT_REASONS.len())];

                        let appt_dt = NaiveDateTime::new(
                            date,
                            chrono::NaiveTime::from_hms_opt(hour, minute, 0)
                                .expect("hour 8..16 and minute 0/15/30/45 are always valid"),
                        );

                        pt_ids.push(patient_id);
                        provider_ids.push(provider_id);
                        clinic_ids.push(clinic_id);
                        appt_dts.push(appt_dt);
                        reasons.push(reason.to_string());
                    }
                }
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
             (patient_id, provider_id, clinic_id, appointment_datetime, reason_for_visit) \
             SELECT * FROM UNNEST($1::uuid[], $2::bigint[], $3::bigint[], $4::timestamp[], $5::text[])"
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

/// Generate appointments for a date range by filling every provider's schedule.
///
/// Each provider gets 36 appointments per day (one per 15-minute slot from 8:00–16:45)
/// at clinics distributed by `clinic_weights`. Patients are drawn randomly.
///
/// Uses `INSERT ... RETURNING` to capture generated UUIDs for immediate downstream use.
pub async fn generate_appointments_by_day(
    pool: &DbPool,
    patient_ids: &[Uuid],
    provider_ids: &[i64],
    clinic_ids: &[i64],
    start_date: NaiveDate,
    end_date: NaiveDate,
    clinic_weights: &[u32],
) -> Result<Vec<(Uuid, Uuid, i64, i64, NaiveDateTime)>, AppError> {
    use rand::{rng, Rng};
    use rand::distr::{Distribution, weighted::WeightedIndex};
    use super::SLOTS_PER_PROVIDER;

    let span = (end_date - start_date).num_days() + 1;

    // Distribute providers across clinics proportionally by weight.
    let weight_sum: u32 = clinic_weights.iter().sum();
    let providers_per_clinic: Vec<usize> = distribute_by_weight(provider_ids.len(), clinic_weights, weight_sum);

    // Build a flat list of (provider_id, clinic_id) assignments.
    let mut provider_assignments: Vec<(i64, i64)> = Vec::with_capacity(provider_ids.len());
    let mut prov_offset = 0usize;
    for (clinic_idx, &num_provs) in providers_per_clinic.iter().enumerate() {
        let clinic_id = clinic_ids[clinic_idx % clinic_ids.len()];
        for _ in 0..num_provs {
            if prov_offset < provider_ids.len() {
                provider_assignments.push((provider_ids[prov_offset], clinic_id));
                prov_offset += 1;
            }
        }
    }

    let total = provider_assignments.len() * SLOTS_PER_PROVIDER * span as usize;
    let mut all_results: Vec<(Uuid, Uuid, i64, i64, NaiveDateTime)> = Vec::with_capacity(total);

    // Build all appointment data synchronously — rng dropped before any await.
    let (pt_ids, prov_ids, cl_ids, appt_dts, reasons) = {
        let mut rng = rng();

        let mut pt_ids:   Vec<Uuid>          = Vec::with_capacity(total);
        let mut prov_ids: Vec<i64>           = Vec::with_capacity(total);
        let mut cl_ids:   Vec<i64>           = Vec::with_capacity(total);
        let mut appt_dts: Vec<NaiveDateTime> = Vec::with_capacity(total);
        let mut reasons:  Vec<String>        = Vec::with_capacity(total);

        for day_offset in 0..span {
            let date = start_date + TimeDelta::days(day_offset);
            for &(provider_id, clinic_id) in &provider_assignments {
                // Fill all 36 time slots for this provider
                for hour in 8u32..17u32 {
                    for quarter in 0u32..4u32 {
                        let minute = quarter * 15;
                        let patient_id = patient_ids[rng.random_range(0..patient_ids.len())];
                        let reason = APPOINTMENT_REASONS[rng.random_range(0..APPOINTMENT_REASONS.len())];

                        let appt_dt = NaiveDateTime::new(
                            date,
                            chrono::NaiveTime::from_hms_opt(hour, minute, 0)
                                .expect("hour 8..16 and minute 0/15/30/45 are always valid"),
                        );

                        pt_ids.push(patient_id);
                        prov_ids.push(provider_id);
                        cl_ids.push(clinic_id);
                        appt_dts.push(appt_dt);
                        reasons.push(reason.to_string());
                    }
                }
            }
        }

        (pt_ids, prov_ids, cl_ids, appt_dts, reasons)
    }; // rng dropped here

    // Bulk-insert in DSQL_BATCH_SIZE chunks, capturing generated IDs via RETURNING.
    for chunk_start in (0..total).step_by(DSQL_BATCH_SIZE) {
        let chunk_end = (chunk_start + DSQL_BATCH_SIZE).min(total);
        let r = chunk_start..chunk_end;

        let rows: Vec<(Uuid, Uuid, i64, i64, NaiveDateTime)> = sqlx::query_as(
            "INSERT INTO vital_fold.appointment \
             (patient_id, provider_id, clinic_id, appointment_datetime, reason_for_visit) \
             SELECT * FROM UNNEST($1::uuid[], $2::bigint[], $3::bigint[], $4::timestamp[], $5::text[]) \
             RETURNING appointment_id, patient_id, clinic_id, provider_id, appointment_datetime"
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

    let days = (end_date - start_date).num_days() + 1;
    tracing::info!(
        "Generated {} appointments ({} providers × 36 slots × {} days) for {} to {}",
        all_results.len(), provider_assignments.len(), days, start_date, end_date
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
