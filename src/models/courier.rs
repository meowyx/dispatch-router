use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeoPoint {
    pub lat: f64,
    pub lng: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum CourierStatus {
    Available,
    Busy,
    Offline,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Courier {
    pub id: Uuid,
    pub name: String,
    pub location: GeoPoint,
    pub capacity: u8,
    pub current_load: u8,
    pub status: CourierStatus,
    pub rating: f64,
    pub updated_at: DateTime<Utc>,
}
