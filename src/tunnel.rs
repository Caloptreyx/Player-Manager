//! Query tunnels: sockets Wings opens to a port of the server's container (see
//! [`crate::context::Context::tcp`]). Only ever use ports derived from the server itself (its
//! allocation or its own config files), never from request input.
use axum::http::StatusCode;
use shared::response::ApiResponse;
use std::{io, time::Duration};

/// How long one exchange through a tunnel may take, opening it included.
pub const TIMEOUT: Duration = Duration::from_secs(3);

/// Why talking through a tunnel failed.
#[derive(Debug, PartialEq, Eq)]
pub enum TunnelError {
    /// Wings could not open the tunnel, or the game answered something unreadable (502).
    Failed(String),
    /// Nothing answered in time, the port refused or the connection closed (504).
    NoAnswer(String),
}

impl TunnelError {
    pub fn failed(message: impl Into<String>) -> Self {
        Self::Failed(message.into())
    }

    /// The same error with `what` (the protocol or the step) in front of its message.
    pub fn about(self, what: &str) -> Self {
        match self {
            Self::Failed(message) => Self::Failed(format!("{what}: {message}")),
            Self::NoAnswer(message) => Self::NoAnswer(format!("{what}: {message}")),
        }
    }

    pub fn response(self) -> ApiResponse {
        match self {
            Self::Failed(message) => {
                ApiResponse::error(message).with_status(StatusCode::BAD_GATEWAY)
            }
            Self::NoAnswer(message) => {
                ApiResponse::error(message).with_status(StatusCode::GATEWAY_TIMEOUT)
            }
        }
    }
}

impl From<io::Error> for TunnelError {
    fn from(err: io::Error) -> Self {
        match err.kind() {
            io::ErrorKind::TimedOut => Self::NoAnswer("no answer in time".into()),
            io::ErrorKind::ConnectionRefused => Self::NoAnswer("connection refused".into()),
            io::ErrorKind::UnexpectedEof
            | io::ErrorKind::ConnectionReset
            | io::ErrorKind::ConnectionAborted
            | io::ErrorKind::BrokenPipe => Self::NoAnswer("the connection was closed".into()),
            _ => Self::Failed(err.to_string()),
        }
    }
}

/// Runs `exchange`, giving up after [`TIMEOUT`].
pub async fn bounded<T>(
    exchange: impl Future<Output = Result<T, TunnelError>>,
) -> Result<T, TunnelError> {
    tokio::time::timeout(TIMEOUT, exchange)
        .await
        .unwrap_or_else(|_| Err(TunnelError::NoAnswer("no answer in time".into())))
}
