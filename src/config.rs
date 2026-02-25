use std::env;

use crate::error::AppError;

#[derive(Debug, Clone)]
pub struct Config {
    pub http_port: u16,
    pub grpc_port: u16,
    pub log_level: String,
    pub order_queue_size: usize,
    pub event_buffer_size: usize,
}

impl Config {
    pub fn from_env() -> Result<Self, AppError> {
        let _ = dotenvy::dotenv();

        Ok(Self {
            http_port: parse_or_default("HTTP_PORT", 3000)?,
            grpc_port: parse_or_default("GRPC_PORT", 50051)?,
            log_level: env::var("LOG_LEVEL").unwrap_or_else(|_| "info".to_string()),
            order_queue_size: parse_or_default("ORDER_QUEUE_SIZE", 1024)?,
            event_buffer_size: parse_or_default("EVENT_BUFFER_SIZE", 1024)?,
        })
    }
}

fn parse_or_default<T>(key: &str, default: T) -> Result<T, AppError>
where
    T: std::str::FromStr,
    T::Err: std::fmt::Display,
{
    match env::var(key) {
        Ok(raw) => raw
            .parse::<T>()
            .map_err(|err| AppError::Internal(format!("invalid {key}: {err}"))),
        Err(_) => Ok(default),
    }
}
