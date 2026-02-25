use dashmap::DashMap;
use tokio::sync::{broadcast, mpsc};
use uuid::Uuid;

use crate::models::assignment::Assignment;
use crate::models::courier::Courier;
use crate::models::order::DeliveryOrder;
use crate::observability::metrics::Metrics;

pub struct AppState {
    pub couriers: DashMap<Uuid, Courier>,
    pub orders: DashMap<Uuid, DeliveryOrder>,
    pub assignments: DashMap<Uuid, Assignment>,
    pub order_tx: mpsc::Sender<DeliveryOrder>,
    pub assignment_events_tx: broadcast::Sender<Assignment>,
    pub metrics: Metrics,
}

impl AppState {
    pub fn new(
        order_queue_size: usize,
        event_buffer_size: usize,
    ) -> (Self, mpsc::Receiver<DeliveryOrder>) {
        let (order_tx, order_rx) = mpsc::channel(order_queue_size);
        let (assignment_events_tx, _unused_rx) = broadcast::channel(event_buffer_size);

        (
            Self {
                couriers: DashMap::new(),
                orders: DashMap::new(),
                assignments: DashMap::new(),
                order_tx,
                assignment_events_tx,
                metrics: Metrics::new(),
            },
            order_rx,
        )
    }
}
