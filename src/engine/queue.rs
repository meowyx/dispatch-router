use crate::error::AppError;
use crate::models::order::DeliveryOrder;
use crate::state::AppState;

pub async fn enqueue_order(state: &AppState, order: DeliveryOrder) -> Result<(), AppError> {
    state
        .order_tx
        .send(order)
        .await
        .map_err(|err| AppError::Internal(format!("order queue send failed: {err}")))?;

    state.metrics.orders_in_queue.inc();
    Ok(())
}
