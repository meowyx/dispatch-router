use std::sync::Arc;
use std::time::Instant;

use chrono::Utc;
use tokio::sync::mpsc;
use tokio::time::{sleep, Duration};
use tracing::{error, info, warn};
use uuid::Uuid;

use crate::engine::queue::enqueue_order;
use crate::engine::scoring::compute_score;
use crate::error::AppError;
use crate::models::assignment::Assignment;
use crate::models::courier::{Courier, CourierStatus};
use crate::models::order::{DeliveryOrder, OrderStatus};
use crate::state::AppState;

pub async fn run_assignment_engine(state: Arc<AppState>, mut order_rx: mpsc::Receiver<DeliveryOrder>) {
    info!("assignment engine started");

    while let Some(order) = order_rx.recv().await {
        state.metrics.orders_in_queue.dec();

        let start = Instant::now();
        match process_order(state.clone(), order).await {
            Ok(()) => {
                let elapsed = start.elapsed().as_secs_f64();
                state
                    .metrics
                    .assignment_latency_seconds
                    .with_label_values(&["success"])
                    .observe(elapsed);
                state
                    .metrics
                    .assignments_total
                    .with_label_values(&["success"])
                    .inc();
            }
            Err(err) => {
                let elapsed = start.elapsed().as_secs_f64();
                state
                    .metrics
                    .assignment_latency_seconds
                    .with_label_values(&["error"])
                    .observe(elapsed);
                state
                    .metrics
                    .assignments_total
                    .with_label_values(&["error"])
                    .inc();
                error!(error = %err, "failed to process order");
            }
        }
    }

    warn!("assignment engine stopped: queue channel closed");
}

async fn process_order(state: Arc<AppState>, order: DeliveryOrder) -> Result<(), AppError> {
    let candidates: Vec<Courier> = state
        .couriers
        .iter()
        .filter_map(|entry| {
            let courier = entry.value();
            let can_take_order = courier.status == CourierStatus::Available
                && courier.current_load < courier.capacity;

            if can_take_order {
                Some(courier.clone())
            } else {
                None
            }
        })
        .collect();

    if candidates.is_empty() {
        warn!(order_id = %order.id, "no eligible couriers; re-queueing order");
        sleep(Duration::from_millis(250)).await;
        enqueue_order(&state, order).await?;
        return Ok(());
    }

    let (winning_courier, best_score, best_breakdown) = candidates
        .iter()
        .map(|courier| {
            let (score, breakdown) = compute_score(courier, &order);
            (courier, score, breakdown)
        })
        .max_by(|a, b| a.1.total_cmp(&b.1))
        .ok_or_else(|| AppError::Internal("failed to score couriers".to_string()))?;

    let mut updated_order = order.clone();
    updated_order.status = OrderStatus::Assigned;
    updated_order.assigned_courier = Some(winning_courier.id);
    state.orders.insert(updated_order.id, updated_order.clone());

    if let Some(mut courier) = state.couriers.get_mut(&winning_courier.id) {
        courier.current_load = courier.current_load.saturating_add(1);
        if courier.current_load >= courier.capacity {
            courier.status = CourierStatus::Busy;
        }
        courier.updated_at = Utc::now();

        let utilization = courier.current_load as f64 / courier.capacity as f64;
        state
            .metrics
            .courier_utilization
            .with_label_values(&[&winning_courier.id.to_string()])
            .set(utilization);
    }

    let assignment = Assignment {
        id: Uuid::new_v4(),
        order_id: updated_order.id,
        courier_id: winning_courier.id,
        score: best_score,
        score_breakdown: best_breakdown,
        assigned_at: Utc::now(),
    };

    state.assignments.insert(assignment.id, assignment.clone());
    let _ = state.assignment_events_tx.send(assignment.clone());

    info!(
        order_id = %updated_order.id,
        courier_id = %winning_courier.id,
        score = best_score,
        "order assigned"
    );

    Ok(())
}
