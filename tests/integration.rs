use std::sync::Arc;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use dispatch_router::api::rest::router;
use dispatch_router::engine::assignment::run_assignment_engine;
use dispatch_router::state::AppState;
use serde_json::{json, Value};
use tokio::sync::mpsc;
use tower::ServiceExt;

use dispatch_router::models::order::DeliveryOrder;

fn setup() -> (axum::Router, mpsc::Receiver<DeliveryOrder>) {
    let (state, rx) = AppState::new(1024, 1024);
    (router(Arc::new(state)), rx)
}

fn json_request(method: &str, uri: &str, body: Value) -> Request<Body> {
    Request::builder()
        .method(method)
        .uri(uri)
        .header("content-type", "application/json")
        .body(Body::from(serde_json::to_string(&body).unwrap()))
        .unwrap()
}

fn get_request(uri: &str) -> Request<Body> {
    Request::builder()
        .method("GET")
        .uri(uri)
        .body(Body::empty())
        .unwrap()
}

fn patch_request(uri: &str, body: Value) -> Request<Body> {
    Request::builder()
        .method("PATCH")
        .uri(uri)
        .header("content-type", "application/json")
        .body(Body::from(serde_json::to_string(&body).unwrap()))
        .unwrap()
}

async fn body_json(response: axum::response::Response) -> Value {
    let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    serde_json::from_slice(&bytes).unwrap()
}

async fn body_string(response: axum::response::Response) -> String {
    let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    String::from_utf8(bytes.to_vec()).unwrap()
}

#[tokio::test]
async fn health_returns_ok() {
    let (app, _rx) = setup();
    let response = app.oneshot(get_request("/health")).await.unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = body_json(response).await;
    assert_eq!(body["status"], "ok");
    assert_eq!(body["couriers"], 0);
    assert_eq!(body["orders"], 0);
    assert_eq!(body["assignments"], 0);
}

#[tokio::test]
async fn metrics_returns_prometheus_format() {
    let (app, _rx) = setup();
    let response = app.oneshot(get_request("/metrics")).await.unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let content_type = response
        .headers()
        .get("content-type")
        .unwrap()
        .to_str()
        .unwrap()
        .to_string();
    assert!(content_type.contains("text/plain"));

    let body = body_string(response).await;
    assert!(body.contains("orders_in_queue"));
}

#[tokio::test]
async fn create_courier_returns_courier() {
    let (app, _rx) = setup();
    let response = app
        .oneshot(json_request(
            "POST",
            "/couriers",
            json!({
                "name": "Alice",
                "location": { "lat": 52.52, "lng": 13.405 },
                "capacity": 5,
                "rating": 4.5
            }),
        ))
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = body_json(response).await;
    assert_eq!(body["name"], "Alice");
    assert_eq!(body["capacity"], 5);
    assert_eq!(body["current_load"], 0);
    assert_eq!(body["status"], "Available");
    assert_eq!(body["rating"], 4.5);
    assert!(body["id"].as_str().unwrap().len() > 0);
}

#[tokio::test]
async fn create_courier_empty_name_returns_400() {
    let (app, _rx) = setup();
    let response = app
        .oneshot(json_request(
            "POST",
            "/couriers",
            json!({
                "name": "  ",
                "location": { "lat": 52.52, "lng": 13.405 },
                "capacity": 5,
                "rating": 4.5
            }),
        ))
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn create_courier_zero_capacity_returns_400() {
    let (app, _rx) = setup();
    let response = app
        .oneshot(json_request(
            "POST",
            "/couriers",
            json!({
                "name": "Bob",
                "location": { "lat": 52.52, "lng": 13.405 },
                "capacity": 0,
                "rating": 4.5
            }),
        ))
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn create_courier_rating_clamped_to_5() {
    let (app, _rx) = setup();
    let response = app
        .oneshot(json_request(
            "POST",
            "/couriers",
            json!({
                "name": "Max",
                "location": { "lat": 52.52, "lng": 13.405 },
                "capacity": 3,
                "rating": 9.9
            }),
        ))
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = body_json(response).await;
    assert_eq!(body["rating"], 5.0);
}

#[tokio::test]
async fn list_couriers_initially_empty() {
    let (app, _rx) = setup();
    let response = app.oneshot(get_request("/couriers")).await.unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = body_json(response).await;
    assert_eq!(body.as_array().unwrap().len(), 0);
}

#[tokio::test]
async fn update_courier_status() {
    let (state, _rx) = AppState::new(1024, 1024);
    let shared = Arc::new(state);
    let app = router(shared.clone());

    let res = app
        .clone()
        .oneshot(json_request(
            "POST",
            "/couriers",
            json!({
                "name": "Eve",
                "location": { "lat": 52.0, "lng": 13.0 },
                "capacity": 3,
                "rating": 4.0
            }),
        ))
        .await
        .unwrap();
    let courier = body_json(res).await;
    let id = courier["id"].as_str().unwrap();

    let res = app
        .oneshot(patch_request(
            &format!("/couriers/{id}/status"),
            json!({ "status": "Offline" }),
        ))
        .await
        .unwrap();

    assert_eq!(res.status(), StatusCode::OK);
    let body = body_json(res).await;
    assert_eq!(body["status"], "Offline");
}

#[tokio::test]
async fn update_courier_location() {
    let (state, _rx) = AppState::new(1024, 1024);
    let shared = Arc::new(state);
    let app = router(shared.clone());

    let res = app
        .clone()
        .oneshot(json_request(
            "POST",
            "/couriers",
            json!({
                "name": "Frank",
                "location": { "lat": 52.0, "lng": 13.0 },
                "capacity": 2,
                "rating": 3.5
            }),
        ))
        .await
        .unwrap();
    let courier = body_json(res).await;
    let id = courier["id"].as_str().unwrap();

    let res = app
        .oneshot(patch_request(
            &format!("/couriers/{id}/location"),
            json!({ "location": { "lat": 48.85, "lng": 2.35 } }),
        ))
        .await
        .unwrap();

    assert_eq!(res.status(), StatusCode::OK);
    let body = body_json(res).await;
    assert_eq!(body["location"]["lat"], 48.85);
    assert_eq!(body["location"]["lng"], 2.35);
}

#[tokio::test]
async fn get_nonexistent_order_returns_404() {
    let (app, _rx) = setup();
    let fake_id = "00000000-0000-0000-0000-000000000000";
    let response = app
        .oneshot(get_request(&format!("/orders/{fake_id}")))
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn create_order_returns_pending() {
    let (app, _rx) = setup();
    let response = app
        .oneshot(json_request(
            "POST",
            "/orders",
            json!({
                "pickup": { "lat": 52.51, "lng": 13.39 },
                "dropoff": { "lat": 52.54, "lng": 13.42 },
                "priority": "Normal"
            }),
        ))
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = body_json(response).await;
    assert_eq!(body["status"], "Pending");
    assert!(body["assigned_courier"].is_null());
}

#[tokio::test]
async fn full_assignment_flow() {
    let (state, rx) = AppState::new(1024, 1024);
    let shared = Arc::new(state);
    tokio::spawn(run_assignment_engine(shared.clone(), rx));
    let app = router(shared.clone());

    let res = app
        .clone()
        .oneshot(json_request(
            "POST",
            "/couriers",
            json!({
                "name": "Dispatch Dan",
                "location": { "lat": 52.52, "lng": 13.405 },
                "capacity": 5,
                "rating": 4.8
            }),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let courier = body_json(res).await;
    let courier_id = courier["id"].as_str().unwrap().to_string();

    let res = app
        .clone()
        .oneshot(json_request(
            "POST",
            "/orders",
            json!({
                "pickup": { "lat": 52.51, "lng": 13.39 },
                "dropoff": { "lat": 52.54, "lng": 13.42 },
                "priority": "Urgent"
            }),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let order = body_json(res).await;
    let order_id = order["id"].as_str().unwrap().to_string();

    tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;

    let res = app
        .clone()
        .oneshot(get_request("/assignments"))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let assignments = body_json(res).await;
    let list = assignments.as_array().unwrap();
    assert_eq!(list.len(), 1);

    let assignment = &list[0];
    assert_eq!(assignment["courier_id"], courier_id);
    assert_eq!(assignment["order_id"], order_id);
    assert!(assignment["score"].as_f64().unwrap() > 0.0);
    assert!(assignment["score_breakdown"]["distance_score"].as_f64().unwrap() > 0.0);
    assert!(assignment["score_breakdown"]["load_score"].as_f64().unwrap() > 0.0);
    assert!(assignment["score_breakdown"]["rating_score"].as_f64().unwrap() > 0.0);
    assert!(assignment["score_breakdown"]["priority_score"].as_f64().unwrap() > 0.0);

    let res = app
        .clone()
        .oneshot(get_request(&format!("/orders/{order_id}")))
        .await
        .unwrap();
    let updated_order = body_json(res).await;
    assert_eq!(updated_order["status"], "Assigned");
    assert_eq!(updated_order["assigned_courier"], courier_id);

    let res = app.oneshot(get_request("/couriers")).await.unwrap();
    let couriers = body_json(res).await;
    let updated_courier = &couriers.as_array().unwrap()[0];
    assert_eq!(updated_courier["current_load"], 1);
}
