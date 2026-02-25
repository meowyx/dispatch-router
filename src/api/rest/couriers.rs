use std::sync::Arc;

use axum::extract::{Path, State};
use axum::routing::{patch, post};
use axum::Json;
use axum::Router;
use chrono::Utc;
use serde::Deserialize;
use uuid::Uuid;

use crate::error::AppError;
use crate::models::courier::{Courier, CourierStatus, GeoPoint};
use crate::state::AppState;

pub fn router() -> Router<Arc<AppState>> {
    Router::new()
        .route("/couriers", post(create_courier).get(list_couriers))
        .route("/couriers/:id/status", patch(update_courier_status))
        .route("/couriers/:id/location", patch(update_courier_location))
}

#[derive(Deserialize)]
pub struct CreateCourierRequest {
    pub name: String,
    pub location: GeoPoint,
    pub capacity: u8,
    pub rating: f64,
}

#[derive(Deserialize)]
pub struct UpdateStatusRequest {
    pub status: CourierStatus,
}

#[derive(Deserialize)]
pub struct UpdateLocationRequest {
    pub location: GeoPoint,
}

async fn create_courier(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<CreateCourierRequest>,
) -> Result<Json<Courier>, AppError> {
    if payload.name.trim().is_empty() {
        return Err(AppError::BadRequest("name cannot be empty".to_string()));
    }

    if payload.capacity == 0 {
        return Err(AppError::BadRequest("capacity must be > 0".to_string()));
    }

    let courier = Courier {
        id: Uuid::new_v4(),
        name: payload.name,
        location: payload.location,
        capacity: payload.capacity,
        current_load: 0,
        status: CourierStatus::Available,
        rating: payload.rating.clamp(0.0, 5.0),
        updated_at: Utc::now(),
    };

    state.couriers.insert(courier.id, courier.clone());
    Ok(Json(courier))
}

async fn list_couriers(State(state): State<Arc<AppState>>) -> Json<Vec<Courier>> {
    let couriers = state
        .couriers
        .iter()
        .map(|entry| entry.value().clone())
        .collect();
    Json(couriers)
}

async fn update_courier_status(
    State(state): State<Arc<AppState>>,
    Path(id): Path<Uuid>,
    Json(payload): Json<UpdateStatusRequest>,
) -> Result<Json<Courier>, AppError> {
    let mut courier = state
        .couriers
        .get_mut(&id)
        .ok_or_else(|| AppError::NotFound(format!("courier {} not found", id)))?;

    courier.status = payload.status;
    courier.updated_at = Utc::now();

    Ok(Json(courier.clone()))
}

async fn update_courier_location(
    State(state): State<Arc<AppState>>,
    Path(id): Path<Uuid>,
    Json(payload): Json<UpdateLocationRequest>,
) -> Result<Json<Courier>, AppError> {
    let mut courier = state
        .couriers
        .get_mut(&id)
        .ok_or_else(|| AppError::NotFound(format!("courier {} not found", id)))?;

    courier.location = payload.location;
    courier.updated_at = Utc::now();

    Ok(Json(courier.clone()))
}
