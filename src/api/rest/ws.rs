use std::sync::Arc;

use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::extract::State;
use axum::response::IntoResponse;
use futures::SinkExt;
use futures::StreamExt;
use tracing::{info, warn};

use crate::state::AppState;

pub async fn ws_handler(
    ws: WebSocketUpgrade,
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    ws.on_upgrade(|socket| handle_socket(socket, state))
}

async fn handle_socket(socket: WebSocket, state: Arc<AppState>) {
    let (mut sender, mut receiver) = socket.split();
    let mut rx = state.assignment_events_tx.subscribe();

    info!("websocket client connected");

    let send_task = tokio::spawn(async move {
        while let Ok(assignment) = rx.recv().await {
            let json = match serde_json::to_string(&assignment) {
                Ok(json) => json,
                Err(err) => {
                    warn!(error = %err, "failed to serialize assignment for ws");
                    continue;
                }
            };

            if sender.send(Message::Text(json.into())).await.is_err() {
                break;
            }
        }
    });

    let recv_task = tokio::spawn(async move {
        while let Some(Ok(_msg)) = receiver.next().await {}
    });

    tokio::select! {
        _ = send_task => {},
        _ = recv_task => {},
    }

    info!("websocket client disconnected");
}
