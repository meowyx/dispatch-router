mod api;
mod config;
mod engine;
mod error;
mod geo;
mod models;
mod observability;
mod state;

use std::sync::Arc;

use tonic::transport::Server as TonicServer;
use tracing_subscriber::EnvFilter;

use crate::api::grpc::pb::dispatch_service_server::DispatchServiceServer;
use crate::api::grpc::GrpcDispatchService;

#[tokio::main]
async fn main() -> Result<(), error::AppError> {
    let config = config::Config::from_env()?;

    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::new(config.log_level.clone()))
        .with_target(false)
        .compact()
        .init();

    let (app_state, order_rx) =
        state::AppState::new(config.order_queue_size, config.event_buffer_size);
    let shared_state = Arc::new(app_state);

    let app = api::rest::router(shared_state.clone());

    tokio::spawn(engine::assignment::run_assignment_engine(
        shared_state.clone(),
        order_rx,
    ));

    let grpc_addr = format!("0.0.0.0:{}", config.grpc_port)
        .parse()
        .map_err(|err| error::AppError::Internal(format!("invalid grpc address: {err}")))?;
    let grpc_service = GrpcDispatchService::new(shared_state.clone());

    tokio::spawn(async move {
        tracing::info!(grpc_port = %grpc_addr, "grpc server started");
        if let Err(err) = TonicServer::builder()
            .add_service(DispatchServiceServer::new(grpc_service))
            .serve(grpc_addr)
            .await
        {
            tracing::error!(error = %err, "grpc server failed");
        }
    });

    let bind_addr = format!("0.0.0.0:{}", config.http_port);
    let listener = tokio::net::TcpListener::bind(&bind_addr)
        .await
        .map_err(|err| error::AppError::Internal(format!("failed to bind {bind_addr}: {err}")))?;

    tracing::info!(http_port = config.http_port, "http server started");

    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await
        .map_err(|err| error::AppError::Internal(format!("server error: {err}")))?;

    Ok(())
}

async fn shutdown_signal() {
    if let Err(err) = tokio::signal::ctrl_c().await {
        tracing::error!(error = %err, "failed to listen for shutdown signal");
    }
}
