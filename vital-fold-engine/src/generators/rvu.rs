/// Generate appointment_cpt (billing line-item) rows in Aurora DSQL.
///
/// Called during Phase 2 (POST /populate/dynamic) after patient visits.
/// Every appointment produces one E/M code line-item drawn from a
/// cardiology-clinic-weighted distribution; when `ekg_usage == true` on the
/// paired patient_visit, a second line-item for CPT 93000 is added.
///
/// Each row snapshots the work/PE/MP RVU values at time of service along
/// with the Medicare conversion factor, so a gold-layer pipeline can roll
/// these up into provider productivity metrics (wRVUs per month, etc.)
/// without worrying about CMS annual updates.

use crate::db::DbPool;
use crate::errors::AppError;
use chrono::{NaiveDate, NaiveDateTime, Utc};
use rand::distr::weighted::WeightedIndex;
use rand::distr::Distribution;
use sqlx::types::BigDecimal;
use sqlx::Row;
use std::collections::HashMap;
use std::str::FromStr;
use uuid::Uuid;

/// Aurora DSQL maximum rows per transaction statement.
const DSQL_BATCH_SIZE: usize = 2500;

/// Medicare Physician Fee Schedule conversion factor — CY2024 final rule.
/// 1 total RVU ≈ $32.74 before GPCI locality adjustments.
const CONVERSION_FACTOR_STR: &str = "32.7442";

/// Cardiology follow-up clinic E/M code distribution.
/// Weights sum to 1000 for readability as per-mille. Heavy on 99213/99214
/// (established, low/moderate complexity) because cardiology clinics are
/// predominantly follow-up visits.
const EM_CODES: &[(&str, u32)] = &[
    ("99213", 400),  // Established, low complexity — most common
    ("99214", 350),  // Established, moderate complexity
    ("99212", 100),  // Established, straightforward
    ("99215", 100),  // Established, high complexity
    ("99211",  20),  // Nurse visit
    ("99204",  20),  // New patient, moderate
    ("99203",  10),  // New patient, low
];

/// CPT code used when ekg_usage = true on a patient_visit.
/// 93000 = global EKG with interpretation & report (covers technical + pro).
const EKG_CPT_CODE: &str = "93000";

/// Cached snapshot of a CPT code row. BigDecimal values are preserved from
/// the database (no f64 round-trip) so snapshot columns stay bit-exact to
/// the reference data — critical for matching on downstream gold-layer joins.
#[derive(Clone)]
struct CptLookup {
    id: i64,
    work_rvu: BigDecimal,
    pe_rvu_nonfacility: BigDecimal,
    mp_rvu: BigDecimal,
}

/// Parallel columnar buffers for bulk UNNEST insert. One `push` call appends
/// a single line-item row to every buffer in lockstep.
#[derive(Default)]
struct CptColumns {
    appointment_ids: Vec<Uuid>,
    cpt_code_ids: Vec<i64>,
    provider_ids: Vec<i64>,
    clinic_ids: Vec<i64>,
    service_dates: Vec<NaiveDate>,
    units: Vec<i16>,
    modifier_1s: Vec<Option<String>>,
    modifier_2s: Vec<Option<String>>,
    work_rvus: Vec<BigDecimal>,
    pe_rvus: Vec<BigDecimal>,
    mp_rvus: Vec<BigDecimal>,
    total_rvus: Vec<BigDecimal>,
    conversion_factors: Vec<BigDecimal>,
    expected_amounts: Vec<Option<BigDecimal>>,
    creation_times: Vec<NaiveDateTime>,
}

impl CptColumns {
    fn with_capacity(n: usize) -> Self {
        Self {
            appointment_ids: Vec::with_capacity(n),
            cpt_code_ids: Vec::with_capacity(n),
            provider_ids: Vec::with_capacity(n),
            clinic_ids: Vec::with_capacity(n),
            service_dates: Vec::with_capacity(n),
            units: Vec::with_capacity(n),
            modifier_1s: Vec::with_capacity(n),
            modifier_2s: Vec::with_capacity(n),
            work_rvus: Vec::with_capacity(n),
            pe_rvus: Vec::with_capacity(n),
            mp_rvus: Vec::with_capacity(n),
            total_rvus: Vec::with_capacity(n),
            conversion_factors: Vec::with_capacity(n),
            expected_amounts: Vec::with_capacity(n),
            creation_times: Vec::with_capacity(n),
        }
    }

    fn len(&self) -> usize {
        self.appointment_ids.len()
    }

    fn push(
        &mut self,
        lookup: &CptLookup,
        appt_id: Uuid,
        provider_id: i64,
        clinic_id: i64,
        service_date: NaiveDate,
        cf: &BigDecimal,
        now: NaiveDateTime,
    ) {
        // Exact decimal arithmetic — no f64 intermediates, no rounding until
        // the final expected_amount step.
        let total_rvu = &lookup.work_rvu + &lookup.pe_rvu_nonfacility + &lookup.mp_rvu;
        let expected = (&total_rvu * cf).round(2);

        self.appointment_ids.push(appt_id);
        self.cpt_code_ids.push(lookup.id);
        self.provider_ids.push(provider_id);
        self.clinic_ids.push(clinic_id);
        self.service_dates.push(service_date);
        self.units.push(1i16);
        self.modifier_1s.push(None);
        self.modifier_2s.push(None);
        self.work_rvus.push(lookup.work_rvu.clone());
        self.pe_rvus.push(lookup.pe_rvu_nonfacility.clone());
        self.mp_rvus.push(lookup.mp_rvu.clone());
        self.total_rvus.push(total_rvu);
        self.conversion_factors.push(cf.clone());
        self.expected_amounts.push(Some(expected));
        self.creation_times.push(now);
    }
}

/// Load all active CPT codes into a `code → CptLookup` map.
/// Verifies the E/M distribution and EKG code are all present; returns a
/// clear BadRequest error if the reference table is empty or missing codes.
async fn load_cpt_codes(pool: &DbPool) -> Result<HashMap<String, CptLookup>, AppError> {
    let rows = sqlx::query(
        "SELECT cpt_code_id, code, work_rvu, pe_rvu_nonfacility, mp_rvu \
         FROM vital_fold.cpt_code WHERE is_active = TRUE"
    )
    .fetch_all(pool)
    .await?;

    if rows.is_empty() {
        return Err(AppError::BadRequest(
            "No cpt_code rows found. Run POST /admin/init-db first.".to_string(),
        ));
    }

    let mut map: HashMap<String, CptLookup> = HashMap::with_capacity(rows.len());
    for row in rows {
        let id: i64 = row.try_get("cpt_code_id")?;
        let code: String = row.try_get("code")?;
        let work_rvu: BigDecimal = row.try_get("work_rvu")?;
        let pe_rvu_nonfacility: BigDecimal = row.try_get("pe_rvu_nonfacility")?;
        let mp_rvu: BigDecimal = row.try_get("mp_rvu")?;

        map.insert(code, CptLookup { id, work_rvu, pe_rvu_nonfacility, mp_rvu });
    }

    // Verify every code referenced by the generator is present.
    let mut missing: Vec<&str> = EM_CODES.iter()
        .map(|(code, _)| *code)
        .filter(|code| !map.contains_key(*code))
        .collect();
    if !map.contains_key(EKG_CPT_CODE) {
        missing.push(EKG_CPT_CODE);
    }
    if !missing.is_empty() {
        return Err(AppError::BadRequest(format!(
            "cpt_code table missing required codes: {}. Re-run POST /admin/init-db.",
            missing.join(", ")
        )));
    }

    Ok(map)
}

/// Generate appointment_cpt rows for the given appointments.
///
/// Returns the number of line-items inserted. Expect ~1.2 × appointments.len()
/// since ~20% of visits have EKG and therefore produce a second line-item.
pub async fn generate_appointment_cpt(
    pool: &DbPool,
    appointments: &[(Uuid, Uuid, i64, i64, NaiveDateTime)],
    ekg_flags: &[bool],
) -> Result<usize, AppError> {
    use rand::rng;

    if appointments.is_empty() {
        return Ok(0);
    }
    if appointments.len() != ekg_flags.len() {
        return Err(AppError::Internal(format!(
            "appointments.len() ({}) != ekg_flags.len() ({})",
            appointments.len(), ekg_flags.len()
        )));
    }

    // Load the CPT reference table once (fails fast if empty/incomplete).
    let cpt_map = load_cpt_codes(pool).await?;

    // Resolve lookups up-front so the hot loop never needs to handle a missing
    // key. load_cpt_codes already verified every code, so these errors are
    // unreachable in practice — but we propagate instead of panicking per
    // the project's never-unwrap rule.
    let em_lookups: Vec<&CptLookup> = EM_CODES.iter()
        .map(|(code, _)| cpt_map.get(*code).ok_or_else(|| AppError::Internal(
            format!("cpt_code '{}' disappeared after load verification", code))))
        .collect::<Result<Vec<_>, _>>()?;
    let ekg_lookup = cpt_map.get(EKG_CPT_CODE).ok_or_else(|| AppError::Internal(
        format!("cpt_code '{}' disappeared after load verification", EKG_CPT_CODE)))?;

    // Build the E/M WeightedIndex. EM_CODES is a compile-time constant, so
    // this cannot fail unless the const is edited incorrectly — propagate
    // rather than panic.
    let em_weights: Vec<u32> = EM_CODES.iter().map(|(_, w)| *w).collect();
    let em_dist = WeightedIndex::new(&em_weights).map_err(|e| AppError::Internal(
        format!("EM_CODES weights are invalid: {}", e)))?;

    // Exact conversion factor via string parse — no float round-trip.
    let cf_big = BigDecimal::from_str(CONVERSION_FACTOR_STR).map_err(|e| AppError::Internal(
        format!("CONVERSION_FACTOR_STR is not a valid BigDecimal: {}", e)))?;

    let ekg_count = ekg_flags.iter().filter(|f| **f).count();

    // Sync RNG block — rng dropped before any await.
    let columns = {
        let mut rng = rng();
        let now: NaiveDateTime = Utc::now().naive_utc();
        let mut cols = CptColumns::with_capacity(appointments.len() + ekg_count);

        for ((appt_id, _patient_id, clinic_id, provider_id, appt_dt), &has_ekg) in
            appointments.iter().zip(ekg_flags.iter())
        {
            let service_date = appt_dt.date();

            // E/M line-item (always present).
            let em_idx = em_dist.sample(&mut rng);
            let em_lookup = em_lookups[em_idx]; // bounds: em_idx < em_weights.len() == em_lookups.len()
            cols.push(em_lookup, *appt_id, *provider_id, *clinic_id, service_date, &cf_big, now);

            // Optional EKG line-item.
            if has_ekg {
                cols.push(ekg_lookup, *appt_id, *provider_id, *clinic_id, service_date, &cf_big, now);
            }
        }

        cols
    }; // rng dropped here

    let total = columns.len();
    if total == 0 {
        return Ok(0);
    }

    let mut inserted = 0usize;
    for chunk_start in (0..total).step_by(DSQL_BATCH_SIZE) {
        let chunk_end = (chunk_start + DSQL_BATCH_SIZE).min(total);
        let r = chunk_start..chunk_end;

        let result = sqlx::query(
            "INSERT INTO vital_fold.appointment_cpt \
             (appointment_id, cpt_code_id, provider_id, clinic_id, service_date, \
              units, modifier_1, modifier_2, \
              work_rvu_snapshot, pe_rvu_snapshot, mp_rvu_snapshot, total_rvu_snapshot, \
              conversion_factor, expected_amount, creation_time) \
             SELECT * FROM UNNEST(\
                $1::uuid[], $2::bigint[], $3::bigint[], $4::bigint[], $5::date[], \
                $6::smallint[], $7::text[], $8::text[], \
                $9::numeric[], $10::numeric[], $11::numeric[], $12::numeric[], \
                $13::numeric[], $14::numeric[], $15::timestamp[])"
        )
        .bind(&columns.appointment_ids[r.clone()])
        .bind(&columns.cpt_code_ids[r.clone()])
        .bind(&columns.provider_ids[r.clone()])
        .bind(&columns.clinic_ids[r.clone()])
        .bind(&columns.service_dates[r.clone()])
        .bind(&columns.units[r.clone()])
        .bind(&columns.modifier_1s[r.clone()])
        .bind(&columns.modifier_2s[r.clone()])
        .bind(&columns.work_rvus[r.clone()])
        .bind(&columns.pe_rvus[r.clone()])
        .bind(&columns.mp_rvus[r.clone()])
        .bind(&columns.total_rvus[r.clone()])
        .bind(&columns.conversion_factors[r.clone()])
        .bind(&columns.expected_amounts[r.clone()])
        .bind(&columns.creation_times[r.clone()])
        .execute(pool)
        .await?;

        inserted += result.rows_affected() as usize;
    }

    tracing::info!(
        "Generated {} appointment_cpt rows from {} appointments ({} with EKG second line-item)",
        inserted, appointments.len(), ekg_count
    );

    Ok(inserted)
}
