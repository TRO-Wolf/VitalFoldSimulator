/// Generate survey rows in Aurora DSQL.
///
/// Called during Phase 2 (POST /populate/dynamic) after patient_visit rows
/// are created. Only ~30% of visits fill out a survey — the real-world patient
/// survey response rate. The resulting rows feed gold-layer provider-quality
/// metrics such as `AVG(gene_prissy_score) GROUP BY provider_id`.

use crate::db::DbPool;
use crate::errors::AppError;
use chrono::{NaiveDateTime, Utc};
use uuid::Uuid;

/// Aurora DSQL maximum rows per transaction statement.
const DSQL_BATCH_SIZE: usize = 2500;

/// Probability that any given visit produces a survey row.
const SURVEY_RESPONSE_RATE: f64 = 0.30;

/// Probability that a survey row includes free-text feedback_comments.
const COMMENT_RATE: f64 = 0.40;

/// Canned free-text feedback comments (chosen at random when present).
const FEEDBACK_COMMENTS: &[&str] = &[
    "Great visit, staff was friendly",
    "Short wait time, very efficient",
    "Doctor explained everything clearly",
    "Long wait but good care",
    "Clean facility, professional team",
    "Felt rushed during the appointment",
    "Excellent bedside manner",
    "Will recommend to family and friends",
    "Appointment was on time",
    "Front desk was very helpful",
];

/// Generate survey rows for a random subset (~30%) of patient visits.
///
/// Returns the number of survey rows inserted.
pub async fn generate_surveys_for_visits(
    pool: &DbPool,
    visit_ids: &[Uuid],
) -> Result<usize, AppError> {
    use rand::{rng, Rng};

    if visit_ids.is_empty() {
        return Ok(0);
    }

    // Build all data synchronously — rng dropped before any await.
    let (patient_visit_ids, gene_prissy_scores, experience_scores, feedback_comments, creation_times) = {
        let mut rng = rng();
        let now: NaiveDateTime = Utc::now().naive_utc();

        let capacity = (visit_ids.len() as f64 * SURVEY_RESPONSE_RATE) as usize + 1;
        let mut patient_visit_ids:  Vec<Uuid>           = Vec::with_capacity(capacity);
        let mut gene_prissy_scores: Vec<i32>            = Vec::with_capacity(capacity);
        let mut experience_scores:  Vec<i32>            = Vec::with_capacity(capacity);
        let mut feedback_comments:  Vec<Option<String>> = Vec::with_capacity(capacity);
        let mut creation_times:     Vec<NaiveDateTime>  = Vec::with_capacity(capacity);

        for &visit_id in visit_ids {
            if !rng.random_bool(SURVEY_RESPONSE_RATE) {
                continue;
            }

            patient_visit_ids.push(visit_id);
            gene_prissy_scores.push(rng.random_range(1..=10i32));
            experience_scores.push(rng.random_range(1..=10i32));

            let comment = if rng.random_bool(COMMENT_RATE) {
                Some(FEEDBACK_COMMENTS[rng.random_range(0..FEEDBACK_COMMENTS.len())].to_string())
            } else {
                None
            };
            feedback_comments.push(comment);
            creation_times.push(now);
        }

        (patient_visit_ids, gene_prissy_scores, experience_scores, feedback_comments, creation_times)
    }; // rng dropped here

    let total = patient_visit_ids.len();
    if total == 0 {
        return Ok(0);
    }

    let mut inserted = 0usize;
    for chunk_start in (0..total).step_by(DSQL_BATCH_SIZE) {
        let chunk_end = (chunk_start + DSQL_BATCH_SIZE).min(total);
        let r = chunk_start..chunk_end;

        let result = sqlx::query(
            "INSERT INTO vital_fold.survey \
             (patient_visit_id, gene_prissy_score, experience_score, feedback_comments, creation_time) \
             SELECT * FROM UNNEST($1::uuid[], $2::int[], $3::int[], $4::text[], $5::timestamp[])"
        )
        .bind(&patient_visit_ids[r.clone()])
        .bind(&gene_prissy_scores[r.clone()])
        .bind(&experience_scores[r.clone()])
        .bind(&feedback_comments[r.clone()])
        .bind(&creation_times[r.clone()])
        .execute(pool)
        .await?;

        inserted += result.rows_affected() as usize;
    }

    tracing::info!(
        "Generated {} survey rows from {} visits ({:.0}% response rate)",
        inserted, visit_ids.len(), (inserted as f64 / visit_ids.len() as f64) * 100.0
    );

    Ok(inserted)
}
