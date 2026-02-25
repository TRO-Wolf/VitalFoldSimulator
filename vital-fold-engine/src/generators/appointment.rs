/// Generate appointments for patients at clinics (Aurora DSQL only).
///
/// Each patient gets N appointments distributed across random clinics and providers.
/// Appointments are bulk-inserted in chunks of DSQL_BATCH_SIZE to stay under Aurora
/// DSQL's 3000-row per-transaction limit.
///
/// DynamoDB writes are NOT performed here. They are performed by `run_simulate` in mod.rs,
/// which queries for appointments where appointment_date = today and writes patient_visit
/// and patient_vitals records on the day of the visit.

use crate::errors::AppError;
use chrono::{Duration, NaiveDateTime, Utc};
use uuid::Uuid;

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
pub async fn generate_appointments(ctx: &mut SimulationContext) -> Result<(), AppError> {
    use rand::{thread_rng, Rng};

    let today = Utc::now().date_naive();
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
                let days_ahead  = rng.gen_range(0..90i64);
                let hour        = rng.gen_range(9..17u32);
                let minute      = rng.gen_range(0..60u32);
                let reason      = APPOINTMENT_REASONS[rng.gen_range(0..APPOINTMENT_REASONS.len())];

                let appt_dt = NaiveDateTime::new(
                    today + Duration::days(days_ahead),
                    chrono::NaiveTime::from_hms_opt(hour, minute, 0).unwrap(),
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

/// Write appointment data to DynamoDB patient_visit table (fire-and-forget).
/// Called by `run_simulate` in mod.rs for appointments whose date matches today.
///
/// Fields written per dynamo.json schema:
/// - creation_time: Unix epoch seconds at the moment of write
/// - record_expiration_epoch: appointment_date + 30 days as Unix epoch (DynamoDB TTL)
pub(super) async fn write_patient_visit(
    dynamo: &aws_sdk_dynamodb::Client,
    patient_id: Uuid,
    clinic_id: Uuid,
    appointment_id: Uuid,
    provider_id: Uuid,
    appointment_dt: NaiveDateTime,
) {
    use aws_sdk_dynamodb::types::AttributeValue;
    use rand::{thread_rng, Rng};

    let (checkout_offset, provider_seen_offset, ekg_usage, copay) = {
        let mut rng = thread_rng();
        (
            rng.gen_range(30..120i64),
            rng.gen_range(5..30i64),
            rng.gen_bool(0.2),
            rng.gen_range(20..150u32),
        )
    };

    let checkin_time  = appointment_dt.format("%Y-%m-%dT%H:%M:%SZ").to_string();
    let checkout_time = (appointment_dt + Duration::minutes(checkout_offset)).format("%Y-%m-%dT%H:%M:%SZ").to_string();
    let provider_seen = (appointment_dt + Duration::minutes(provider_seen_offset)).format("%Y-%m-%dT%H:%M:%SZ").to_string();

    // Epoch values for auditing and TTL.
    // creation_time: when this record is written (now).
    // record_expiration_epoch: 90 days from now.
    let now          = Utc::now();
    let now_epoch    = now.timestamp();
    let expiry_epoch = (now + Duration::days(90)).timestamp();

    // Sort key is "clinic_id#appointment_id" to ensure uniqueness —
    // a patient can have multiple appointments at the same clinic.
    let sort_key = format!("{}#{}", clinic_id, appointment_id);

    let result = dynamo
        .put_item()
        .table_name("patient_visit")
        .item("patient_id",              AttributeValue::S(patient_id.to_string()))
        .item("clinic_id",               AttributeValue::S(sort_key))
        .item("appointment_id",          AttributeValue::S(appointment_id.to_string()))
        .item("provider_id",             AttributeValue::S(provider_id.to_string()))
        .item("checkin_time",            AttributeValue::S(checkin_time))
        .item("checkout_time",           AttributeValue::S(checkout_time))
        .item("provider_seen_time",      AttributeValue::S(provider_seen))
        .item("ekg_usage",               AttributeValue::Bool(ekg_usage))
        .item("estimated_copay",         AttributeValue::N(copay.to_string()))
        .item("creation_time",           AttributeValue::N(now_epoch.to_string()))
        .item("record_expiration_epoch", AttributeValue::N(expiry_epoch.to_string()))
        .send()
        .await;

    if let Err(e) = result {
        tracing::error!("DynamoDB patient_visit write failed: {:?}", e);
    }
}

/// Write vitals data to DynamoDB patient_vitals table (fire-and-forget).
/// Called by `run_simulate` in mod.rs for appointments whose date matches today.
///
/// `appointment_dt` is required to compute `record_expiration_epoch` (appt + 30 days).
///
/// Fields written per dynamo.json schema:
/// - creation_time: Unix epoch seconds at the moment of write
/// - record_expiration_epoch: appointment_date + 30 days as Unix epoch (DynamoDB TTL)
/// - weight: correct spelling per updated dynamo.json schema
pub(super) async fn write_patient_vitals(
    dynamo: &aws_sdk_dynamodb::Client,
    patient_id: Uuid,
    clinic_id: Uuid,
    visit_id: Uuid,
    provider_id: Uuid,
) {
    use aws_sdk_dynamodb::types::AttributeValue;
    use rand::{thread_rng, Rng};

    let (heart_rate, oxygen, temp, sys, dia, height, weight, pulses) = {
        let mut rng = thread_rng();
        (
            rng.gen_range(50..120u32),
            rng.gen_range(92..101u32),
            97.0f64 + (rng.gen::<f64>() * 2.5),
            rng.gen_range(100..160u32),
            rng.gen_range(60..100u32),
            rng.gen_range(60.0f64..78.0),
            rng.gen_range(120.0f64..220.0),
            rng.gen_range(50..120u32), // pulse rate bpm, same clinical range as heart_rate
        )
    };

    // Epoch values for auditing and TTL.
    // creation_time: when this record is written (now).
    // record_expiration_epoch: 90 days from now.
    let now          = Utc::now();
    let now_epoch    = now.timestamp();
    let expiry_epoch = (now + Duration::days(90)).timestamp();

    // Sort key is "clinic_id#visit_id" to ensure uniqueness —
    // a patient can have multiple appointments at the same clinic.
    let sort_key = format!("{}#{}", clinic_id, visit_id);

    let result = dynamo
        .put_item()
        .table_name("patient_vitals")
        .item("patient_id",              AttributeValue::S(patient_id.to_string()))
        .item("clinic_id",               AttributeValue::S(sort_key))
        .item("provider_id",             AttributeValue::S(provider_id.to_string()))
        .item("visit_id",                AttributeValue::S(visit_id.to_string()))
        .item("heart_rate",              AttributeValue::N(heart_rate.to_string()))
        .item("oxygen",                  AttributeValue::N(oxygen.to_string()))
        .item("temperature",             AttributeValue::N(format!("{:.1}", temp)))
        .item("blood_pressure",          AttributeValue::S(format!("{}/{}", sys, dia)))
        .item("height",                  AttributeValue::N(format!("{:.1}", height)))
        .item("weight",                  AttributeValue::N(format!("{:.1}", weight)))
        .item("pulses",                  AttributeValue::N(pulses.to_string()))
        .item("creation_time",           AttributeValue::N(now_epoch.to_string()))
        .item("record_expiration_epoch", AttributeValue::N(expiry_epoch.to_string()))
        .send()
        .await;

    if let Err(e) = result {
        tracing::error!("DynamoDB patient_vitals write failed: {:?}", e);
    }
}
