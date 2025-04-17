use std::sync::Arc;

use axum::body::Body;
use axum::extract::Extension;
use axum::http::header;
use axum::http::StatusCode;
use axum::response::Response;
use axum::routing::get;
use axum::Router;
use axum_examples::api_errors::AppError;
use oracle::connection::EngineOracle;
use serde_json::Value as JsonValue;
use tokio::net::TcpListener;
use utils::polars_df_to_json::df_to_json_each_column;

mod axum_examples;
mod oracle;
mod utils;

#[tokio::main]
async fn main() -> Result<(), std::io::Error>
{
    // 1) .env + logger
    dotenv::dotenv().ok();

    // 2) Instancia o EngineOracle e embala num Arc
    let engine = Arc::new(EngineOracle::new().expect("falha ao conectar no Oracle"));

    // 3) Cria o Router e injeta o Arc<EngineOracle> como camada de estado
    let app = Router::new().route("/df", get(get_df)).layer(Extension(engine));

    // 4) Sobe o servidor
    let listener = TcpListener::bind("127.0.0.1:8080").await?;
    println!("Listening on http://{}", listener.local_addr()?);
    let _ = axum::serve(listener, app).await;
    Ok(())
}

// Agora o handler recebe o Arc<EngineOracle> como parâmetro
async fn get_df(Extension(engine): Extension<Arc<EngineOracle>>) -> Result<Response, AppError>
{
    let sql = r#"
        SELECT *
        FROM SYSADM.PS_MMC_CHASSI_LOC
        WHERE ROWNUM <= :1
    "#;

    // usa a mesma instância que veio de main
    let df = engine.query_to_polars_df(sql, &[&500000])?;

    let data_json: JsonValue = df_to_json_each_column(&df)?;
    let body_str = serde_json::to_string(&data_json)?;

    let response = Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(body_str))?;
    Ok(response)
}
