use actix_web::http::StatusCode;
use actix_web::HttpResponse;
use actix_web::ResponseError;
use polars::prelude::PolarsError;
use serde_json::Error as SerdeError;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ApiError
{
    // #[error("Erro interno do servidor")]
    // Internal,

    // #[error("Dados inválidos: {0}")]
    // BadRequest(String),
    #[error("Erro na operação com DataFrame: {0}")]
    DataFrame(#[from] PolarsError),

    #[error("Falha ao (de)serializar JSON: {0}")]
    Json(#[from] SerdeError),
    // … outros variants conforme sua necessidade
}

impl ResponseError for ApiError
{
    fn status_code(&self) -> StatusCode
    {
        match *self
        {
            // ApiError::Internal => StatusCode::INTERNAL_SERVER_ERROR,
            // ApiError::BadRequest(_) => StatusCode::BAD_REQUEST,
            ApiError::DataFrame(_) => StatusCode::UNPROCESSABLE_ENTITY,
            ApiError::Json(_) => StatusCode::BAD_REQUEST,
        }
    }

    fn error_response(&self) -> HttpResponse
    {
        // Aqui você pode montar um JSON de erro mais rico, se quiser
        HttpResponse::build(self.status_code()).json(serde_json::json!({
            "error": self.to_string()
        }))
    }
}
