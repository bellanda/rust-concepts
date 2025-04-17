use actix_web::http::StatusCode;
use actix_web::HttpResponse;
use actix_web::ResponseError;
use http::Error as HttpError;
use polars::prelude::PolarsError;
use serde_json::Error as SerdeError;
use thiserror::Error; // importar o http::Error

#[derive(Error, Debug)]
pub enum ApiError
{
    #[error("Erro na operação com DataFrame: {0}")]
    DataFrame(#[from] PolarsError),

    #[error("Falha ao (de)serializar JSON: {0}")]
    Json(#[from] SerdeError),

    #[error("Falha ao construir resposta HTTP: {0}")]
    Http(#[from] HttpError), // adiciona o From<HttpError>
}

impl ResponseError for ApiError
{
    fn status_code(&self) -> StatusCode
    {
        match *self
        {
            ApiError::DataFrame(_) => StatusCode::UNPROCESSABLE_ENTITY,
            ApiError::Json(_) => StatusCode::BAD_REQUEST,
            ApiError::Http(_) => StatusCode::INTERNAL_SERVER_ERROR, // ou outro código adequado
        }
    }

    fn error_response(&self) -> HttpResponse
    {
        HttpResponse::build(self.status_code()).json(serde_json::json!({ "error": self.to_string() }))
    }
}
