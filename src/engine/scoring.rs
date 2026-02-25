use crate::geo::haversine_km;
use crate::models::assignment::ScoreBreakdown;
use crate::models::courier::Courier;
use crate::models::order::{DeliveryOrder, Priority};

const DISTANCE_WEIGHT: f64 = 0.40;
const LOAD_WEIGHT: f64 = 0.30;
const RATING_WEIGHT: f64 = 0.20;
const PRIORITY_WEIGHT: f64 = 0.10;

pub fn compute_score(courier: &Courier, order: &DeliveryOrder) -> (f64, ScoreBreakdown) {
    let distance_km = haversine_km(&courier.location, &order.pickup);

    let breakdown = ScoreBreakdown {
        distance_score: distance_score(distance_km),
        load_score: load_score(courier.current_load, courier.capacity),
        rating_score: rating_score(courier.rating),
        priority_score: priority_score(&order.priority),
    };

    let score = weighted_score(&breakdown);
    (score, breakdown)
}

pub fn weighted_score(breakdown: &ScoreBreakdown) -> f64 {
    (breakdown.distance_score * DISTANCE_WEIGHT)
        + (breakdown.load_score * LOAD_WEIGHT)
        + (breakdown.rating_score * RATING_WEIGHT)
        + (breakdown.priority_score * PRIORITY_WEIGHT)
}

fn distance_score(distance_km: f64) -> f64 {
    1.0 / (1.0 + distance_km.max(0.0))
}

fn load_score(current_load: u8, capacity: u8) -> f64 {
    if capacity == 0 {
        return 0.0;
    }

    let utilization = current_load as f64 / capacity as f64;
    (1.0 - utilization).clamp(0.0, 1.0)
}

fn rating_score(rating: f64) -> f64 {
    (rating / 5.0).clamp(0.0, 1.0)
}

fn priority_score(priority: &Priority) -> f64 {
    match priority {
        Priority::Low => 0.5,
        Priority::Normal => 0.7,
        Priority::High => 0.85,
        Priority::Urgent => 1.0,
    }
}

#[cfg(test)]
mod tests {
    use chrono::Utc;
    use uuid::Uuid;

    use super::compute_score;
    use crate::models::courier::{Courier, CourierStatus, GeoPoint};
    use crate::models::order::{DeliveryOrder, OrderStatus, Priority};

    fn courier(id_seed: u128, lat: f64, lng: f64, load: u8, capacity: u8, rating: f64) -> Courier {
        Courier {
            id: Uuid::from_u128(id_seed),
            name: "test-courier".to_string(),
            location: GeoPoint { lat, lng },
            capacity,
            current_load: load,
            status: CourierStatus::Available,
            rating,
            updated_at: Utc::now(),
        }
    }

    fn order(priority: Priority, lat: f64, lng: f64) -> DeliveryOrder {
        DeliveryOrder {
            id: Uuid::new_v4(),
            pickup: GeoPoint { lat, lng },
            dropoff: GeoPoint {
                lat: lat + 0.01,
                lng: lng + 0.01,
            },
            priority,
            status: OrderStatus::Pending,
            assigned_courier: None,
            created_at: Utc::now(),
        }
    }

    #[test]
    fn closer_courier_gets_higher_score_when_other_factors_match() {
        let pickup_order = order(Priority::Normal, 53.5511, 9.9937);

        let near = courier(1, 53.5512, 9.9938, 0, 3, 4.5);
        let far = courier(2, 53.7, 10.2, 0, 3, 4.5);

        let (near_score, _) = compute_score(&near, &pickup_order);
        let (far_score, _) = compute_score(&far, &pickup_order);

        assert!(near_score > far_score);
    }

    #[test]
    fn heavily_loaded_courier_is_penalized() {
        let pickup_order = order(Priority::Normal, 53.5511, 9.9937);

        let light_load = courier(1, 53.5512, 9.9938, 0, 3, 4.5);
        let heavy_load = courier(2, 53.5512, 9.9938, 2, 3, 4.5);

        let (light_score, _) = compute_score(&light_load, &pickup_order);
        let (heavy_score, _) = compute_score(&heavy_load, &pickup_order);

        assert!(light_score > heavy_score);
    }

    #[test]
    fn urgent_priority_increases_priority_component() {
        let courier = courier(1, 53.5512, 9.9938, 0, 3, 4.5);

        let normal_order = order(Priority::Normal, 53.5511, 9.9937);
        let urgent_order = order(Priority::Urgent, 53.5511, 9.9937);

        let (_normal_total, normal_breakdown) = compute_score(&courier, &normal_order);
        let (_urgent_total, urgent_breakdown) = compute_score(&courier, &urgent_order);

        assert!(urgent_breakdown.priority_score > normal_breakdown.priority_score);
    }
}
