pub mod couriers;
pub mod orders;
pub mod ws;

use std::sync::Arc;

use axum::extract::State;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::routing::get;
use axum::Json;
use axum::Router;
use serde::Serialize;
use tower_http::services::ServeDir;

use crate::state::AppState;

pub fn router(state: Arc<AppState>) -> Router {
    Router::new()
        .merge(couriers::router())
        .merge(orders::router())
        .route("/health", get(health))
        .route("/metrics", get(metrics))
        .route("/ws", get(ws::ws_handler))
        .with_state(state)
        .fallback_service(ServeDir::new("static"))
}

#[derive(Serialize)]
struct HealthResponse {
    status: &'static str,
    couriers: usize,
    orders: usize,
    assignments: usize,
}

async fn health(State(state): State<Arc<AppState>>) -> Json<HealthResponse> {
    Json(HealthResponse {
        status: "ok",
        couriers: state.couriers.len(),
        orders: state.orders.len(),
        assignments: state.assignments.len(),
    })
}

async fn metrics(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    match state.metrics.encode() {
        Ok(body) => (
            StatusCode::OK,
            [("content-type", "text/plain; version=0.0.4; charset=utf-8")],
            body,
        )
            .into_response(),
        Err(err) => (StatusCode::INTERNAL_SERVER_ERROR, err).into_response(),
    }
}
