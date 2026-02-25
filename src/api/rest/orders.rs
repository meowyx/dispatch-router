use std::sync::Arc;

use axum::extract::{Path, State};
use axum::routing::{get, post};
use axum::Json;
use axum::Router;
use chrono::Utc;
use serde::Deserialize;
use uuid::Uuid;

use crate::engine::queue::enqueue_order;
use crate::error::AppError;
use crate::models::assignment::Assignment;
use crate::models::courier::GeoPoint;
use crate::models::order::{DeliveryOrder, OrderStatus, Priority};
use crate::state::AppState;

pub fn router() -> Router<Arc<AppState>> {
    Router::new()
        .route("/orders", post(create_order))
        .route("/orders/:id", get(get_order))
        .route("/assignments", get(list_assignments))
}

#[derive(Deserialize)]
pub struct CreateOrderRequest {
    pub pickup: GeoPoint,
    pub dropoff: GeoPoint,
    pub priority: Priority,
}

async fn create_order(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<CreateOrderRequest>,
) -> Result<Json<DeliveryOrder>, AppError> {
    let order = DeliveryOrder {
        id: Uuid::new_v4(),
        pickup: payload.pickup,
        dropoff: payload.dropoff,
        priority: payload.priority,
        status: OrderStatus::Pending,
        assigned_courier: None,
        created_at: Utc::now(),
    };

    state.orders.insert(order.id, order.clone());
    enqueue_order(&state, order.clone()).await?;

    Ok(Json(order))
}

async fn get_order(
    State(state): State<Arc<AppState>>,
    Path(id): Path<Uuid>,
) -> Result<Json<DeliveryOrder>, AppError> {
    let order = state
        .orders
        .get(&id)
        .ok_or_else(|| AppError::NotFound(format!("order {} not found", id)))?;

    Ok(Json(order.value().clone()))
}

async fn list_assignments(State(state): State<Arc<AppState>>) -> Json<Vec<Assignment>> {
    let assignments = state
        .assignments
        .iter()
        .map(|entry| entry.value().clone())
        .collect();

    Json(assignments)
}
