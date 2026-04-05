use chrono::NaiveDateTime;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Survey {
    pub survey_id: Uuid,
    pub patient_visit_id: Uuid,
    pub gene_prissy_score: i32,
    pub experience_score: i32,
    pub feedback_comments: Option<String>,
    pub creation_time: NaiveDateTime,
}
