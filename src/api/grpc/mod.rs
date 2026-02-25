use std::pin::Pin;
use std::sync::Arc;

use chrono::Utc;
use tokio_stream::wrappers::BroadcastStream;
use tokio_stream::Stream;
use tokio_stream::StreamExt;
use tonic::{Request, Response, Status};
use uuid::Uuid;

use crate::engine::queue::enqueue_order;
use crate::models::courier::{Courier, CourierStatus};
use crate::models::order::{DeliveryOrder, OrderStatus, Priority};
use crate::state::AppState;

pub mod pb {
    tonic::include_proto!("dispatch");
}

use pb::dispatch_service_server::DispatchService;
use pb::{
    AssignmentEvent, CourierResponse, CreateCourierRequest, CreateOrderRequest, GeoPoint,
    GetAssignmentsRequest, GetAssignmentsResponse, GetCouriersRequest, GetCouriersResponse,
    OrderResponse, ScoreBreakdown, WatchAssignmentsRequest,
};

pub struct GrpcDispatchService {
    state: Arc<AppState>,
}

impl GrpcDispatchService {
    pub fn new(state: Arc<AppState>) -> Self {
        Self { state }
    }
}

fn courier_to_proto(c: &Courier) -> CourierResponse {
    CourierResponse {
        id: c.id.to_string(),
        name: c.name.clone(),
        location: Some(GeoPoint {
            lat: c.location.lat,
            lng: c.location.lng,
        }),
        capacity: c.capacity as u32,
        current_load: c.current_load as u32,
        status: format!("{:?}", c.status),
        rating: c.rating,
    }
}

fn assignment_to_proto(a: &crate::models::assignment::Assignment) -> AssignmentEvent {
    AssignmentEvent {
        id: a.id.to_string(),
        order_id: a.order_id.to_string(),
        courier_id: a.courier_id.to_string(),
        score: a.score,
        score_breakdown: Some(ScoreBreakdown {
            distance_score: a.score_breakdown.distance_score,
            load_score: a.score_breakdown.load_score,
            rating_score: a.score_breakdown.rating_score,
            priority_score: a.score_breakdown.priority_score,
        }),
        assigned_at: a.assigned_at.to_rfc3339(),
    }
}

fn parse_priority(s: &str) -> Result<Priority, Status> {
    match s {
        "Low" => Ok(Priority::Low),
        "Normal" => Ok(Priority::Normal),
        "High" => Ok(Priority::High),
        "Urgent" => Ok(Priority::Urgent),
        other => Err(Status::invalid_argument(format!(
            "unknown priority: {other}, expected Low/Normal/High/Urgent"
        ))),
    }
}

#[tonic::async_trait]
impl DispatchService for GrpcDispatchService {
    async fn create_courier(
        &self,
        request: Request<CreateCourierRequest>,
    ) -> Result<Response<CourierResponse>, Status> {
        let req = request.into_inner();

        if req.name.trim().is_empty() {
            return Err(Status::invalid_argument("name cannot be empty"));
        }
        if req.capacity == 0 {
            return Err(Status::invalid_argument("capacity must be > 0"));
        }

        let location = req
            .location
            .ok_or_else(|| Status::invalid_argument("location is required"))?;

        let courier = Courier {
            id: Uuid::new_v4(),
            name: req.name,
            location: crate::models::courier::GeoPoint {
                lat: location.lat,
                lng: location.lng,
            },
            capacity: req.capacity.min(255) as u8,
            current_load: 0,
            status: CourierStatus::Available,
            rating: req.rating.clamp(0.0, 5.0),
            updated_at: Utc::now(),
        };

        self.state.couriers.insert(courier.id, courier.clone());
        Ok(Response::new(courier_to_proto(&courier)))
    }

    async fn get_couriers(
        &self,
        _request: Request<GetCouriersRequest>,
    ) -> Result<Response<GetCouriersResponse>, Status> {
        let couriers: Vec<CourierResponse> = self
            .state
            .couriers
            .iter()
            .map(|entry| courier_to_proto(entry.value()))
            .collect();

        Ok(Response::new(GetCouriersResponse { couriers }))
    }

    async fn create_order(
        &self,
        request: Request<CreateOrderRequest>,
    ) -> Result<Response<OrderResponse>, Status> {
        let req = request.into_inner();

        let pickup = req
            .pickup
            .ok_or_else(|| Status::invalid_argument("pickup is required"))?;
        let dropoff = req
            .dropoff
            .ok_or_else(|| Status::invalid_argument("dropoff is required"))?;

        let priority = parse_priority(&req.priority)?;

        let order = DeliveryOrder {
            id: Uuid::new_v4(),
            pickup: crate::models::courier::GeoPoint {
                lat: pickup.lat,
                lng: pickup.lng,
            },
            dropoff: crate::models::courier::GeoPoint {
                lat: dropoff.lat,
                lng: dropoff.lng,
            },
            priority: priority.clone(),
            status: OrderStatus::Pending,
            assigned_courier: None,
            created_at: Utc::now(),
        };

        self.state.orders.insert(order.id, order.clone());
        enqueue_order(&self.state, order.clone())
            .await
            .map_err(|err| Status::internal(format!("enqueue failed: {err}")))?;

        Ok(Response::new(OrderResponse {
            id: order.id.to_string(),
            pickup: Some(GeoPoint {
                lat: pickup.lat,
                lng: pickup.lng,
            }),
            dropoff: Some(GeoPoint {
                lat: dropoff.lat,
                lng: dropoff.lng,
            }),
            priority: format!("{:?}", order.priority),
            status: format!("{:?}", order.status),
        }))
    }

    async fn get_assignments(
        &self,
        _request: Request<GetAssignmentsRequest>,
    ) -> Result<Response<GetAssignmentsResponse>, Status> {
        let assignments: Vec<AssignmentEvent> = self
            .state
            .assignments
            .iter()
            .map(|entry| assignment_to_proto(entry.value()))
            .collect();

        Ok(Response::new(GetAssignmentsResponse { assignments }))
    }

    type WatchAssignmentsStream =
        Pin<Box<dyn Stream<Item = Result<AssignmentEvent, Status>> + Send>>;

    async fn watch_assignments(
        &self,
        _request: Request<WatchAssignmentsRequest>,
    ) -> Result<Response<Self::WatchAssignmentsStream>, Status> {
        let rx = self.state.assignment_events_tx.subscribe();
        let stream = BroadcastStream::new(rx).filter_map(|result| match result {
            Ok(assignment) => Some(Ok(assignment_to_proto(&assignment))),
            Err(_) => None,
        });

        Ok(Response::new(Box::pin(stream)))
    }
}
