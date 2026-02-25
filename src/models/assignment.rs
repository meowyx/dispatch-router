use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScoreBreakdown {
    pub distance_score: f64,
    pub load_score: f64,
    pub rating_score: f64,
    pub priority_score: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Assignment {
    pub id: Uuid,
    pub order_id: Uuid,
    pub courier_id: Uuid,
    pub score: f64,
    pub score_breakdown: ScoreBreakdown,
    pub assigned_at: DateTime<Utc>,
}
