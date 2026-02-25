use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::models::courier::GeoPoint;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Priority {
    Low,
    Normal,
    High,
    Urgent,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum OrderStatus {
    Pending,
    Assigned,
    InTransit,
    Delivered,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeliveryOrder {
    pub id: Uuid,
    pub pickup: GeoPoint,
    pub dropoff: GeoPoint,
    pub priority: Priority,
    pub status: OrderStatus,
    pub assigned_courier: Option<Uuid>,
    pub created_at: DateTime<Utc>,
}
