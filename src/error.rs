use axum::{
    Json,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use serde::Serialize;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum AppError {
    #[error("invalid url")]
    InvalidUrl,
    #[error("qr code not found")]
    NotFound,
    #[error("database error: {0}")]
    Database(String),
    #[error("failed to generate unique token")]
    TokenGenerationFailed,
    #[error("invalid color")]
    InvalidColor,
    #[error("failed to render qr code image")]
    RenderFailed,
}

#[derive(Debug, Serialize)]
struct ErrorBody {
    error: &'static str,
    message: String,
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, code) = match self {
            Self::InvalidUrl => (StatusCode::BAD_REQUEST, "invalid_url"),
            Self::NotFound => (StatusCode::NOT_FOUND, "not_found"),
            Self::Database(_) => (StatusCode::INTERNAL_SERVER_ERROR, "database_error"),
            Self::TokenGenerationFailed => (StatusCode::CONFLICT, "token_generation_failed"),
            Self::InvalidColor => (StatusCode::BAD_REQUEST, "invalid_color"),
            Self::RenderFailed => (StatusCode::INTERNAL_SERVER_ERROR, "render_failed"),
        };

        (
            status,
            Json(ErrorBody {
                error: code,
                message: self.to_string(),
            }),
        )
            .into_response()
    }
}
