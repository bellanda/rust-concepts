use axum::http::header;
use axum::http::Error as HttpError;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::response::Response;
use oracle;
use polars::prelude::PolarsError;
use serde_json::json;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum AppError
{
    #[error("Erro na operação Polars: {0}")]
    Polars(#[from] PolarsError),

    #[error("Falha ao (de)serializar JSON: {0}")]
    Json(#[from] serde_json::Error),

    #[error("Falha ao construir resposta HTTP: {0}")]
    Http(#[from] HttpError),

    #[error("Erro no Oracle: {0}")]
    Oracle(#[from] oracle::Error),

    /// captura qualquer `Box<dyn Error + Send + Sync>`
    #[error("Erro genérico: {0}")]
    Generic(#[from] Box<dyn std::error::Error + Send + Sync>),
}
impl IntoResponse for AppError
{
    fn into_response(self) -> Response
    {
        let (status, msg) = match &self
        {
            AppError::Polars(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()),
            AppError::Json(e) => (StatusCode::BAD_REQUEST, e.to_string()),
            AppError::Http(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()),
            AppError::Oracle(e) => (StatusCode::BAD_GATEWAY, e.to_string()),
            AppError::Generic(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()),
        };

        let body = json!({ "error": msg });
        (status, [(header::CONTENT_TYPE, "application/json")], axum::Json(body)).into_response()
    }
}
