use axum::body::Body;
use axum::extract::Json;
use axum::http::header;
use axum::http::StatusCode;
use axum::response::Response;
use axum::routing::get;
use axum::routing::post;
use axum::Router;
use axum_examples::api_errors::AppError;
use polars::prelude::*;
use serde::Deserialize;
use serde::Serialize;
use serde_json::Value as JsonValue;
use tokio::net::TcpListener;
use utils::polars_df_to_json::df_to_json_each_column;
use utils::polars_df_to_json::df_to_json_each_row;

mod axum_examples;
mod utils;

#[derive(Serialize, Deserialize, Debug)]
struct Dados
{
    campo1: String,
    campo2: i32,
}

#[derive(Serialize, Deserialize)]
struct User
{
    name: String,
    email: String,
    age: u32,
    names: Vec<String>,
}

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<(), std::io::Error>
{
    let app = Router::new()
        .route("/echo", post(echo_json))
        .route("/user", get(get_user))
        .route("/users-df", get(get_users_df))
        .route("/users-large-df", get(get_large_users_df))
        .route("/users", get(get_users));

    let listener = TcpListener::bind("127.0.0.1:8080").await?;
    println!("Listening on http://{}", listener.local_addr()?);
    let _ = axum::serve(listener, app).await;
    Ok(())
}

// POST
async fn echo_json(Json(dados): Json<Dados>) -> Json<Dados>
{
    println!("Recebido: {:?}", dados);
    Json(dados)
}

// GET
async fn get_user() -> Json<User>
{
    let user = User {
        name: "John Doe".to_string(),
        email: "john@example.com".to_string(),
        age: 30,
        names: vec!["John".to_string(), "Doe".to_string()],
    };
    Json(user)
}

// GET JSON
async fn get_users_df() -> Result<Response, AppError>
{
    // 1) Simula 100.000 linhas
    let n = 10_000;
    let mut names = Vec::with_capacity(n);
    let mut ages = Vec::with_capacity(n);
    let mut weights = Vec::with_capacity(n);
    let mut heights = Vec::with_capacity(n);

    for i in 0..n
    {
        names.push(format!("User{}", i % 100_000));
        ages.push((i % 60 + 18) as u32);
        weights.push(50.0 + ((i % 50) as f64) * 0.5);
        heights.push(1.50 + ((i % 50) as f64) * 0.01);
    }

    // 2) DataFrame
    let df = DataFrame::new(vec![
        Series::new("name".into(), names).into(),
        Series::new("age".into(), ages).into(),
        Series::new("weight".into(), weights).into(),
        Series::new("height".into(), heights).into(),
    ])?;

    // 3) Pipeline Lazy
    let lazy_df = df
        .lazy()
        .filter(col("age").gt(lit(30u32)))
        .with_column((col("weight") / col("height").pow(2)).alias("bmi"))
        .group_by(vec![col("name")])
        .agg(vec![
            col("bmi").mean().alias("mean_bmi"),
            col("bmi").std(0).alias("std_bmi"),
            col("age").min().alias("min_age"),
            col("age").max().alias("max_age"),
            col("age").count().alias("count"),
        ])
        .filter(col("mean_bmi").gt(lit(20.0)))
        .sort(
            vec!["mean_bmi".to_string()],
            SortMultipleOptions::default()
                .with_order_descending(false)
                .with_nulls_last(true),
        )
        .limit(20_000);

    #[derive(Serialize, Deserialize)]
    struct Wrapper
    {
        total: usize,
        data: JsonValue,
    }

    // 4) Collect
    let result_df = lazy_df.collect()?;

    // 4.1) Conta quantas linhas vieram
    let total = result_df.height();

    // 5) Serializa data + total
    let data_json: JsonValue = df_to_json_each_column(&result_df)?; // seu array de objetos

    // 5) Serializa o struct DIRETAMENTE
    let wrapper = Wrapper { total, data: data_json };
    let body_str = serde_json::to_string(&wrapper)?;

    // 6) Monta a resposta HTTP
    let response = Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(body_str))?;
    Ok(response)
}

// GET JSON
async fn get_large_users_df() -> Result<Response, AppError>
{
    // 1) Simula 100.000 linhas
    let n = 50_000;
    let mut names = Vec::with_capacity(n);
    let mut ages = Vec::with_capacity(n);
    let mut weights = Vec::with_capacity(n);
    let mut heights = Vec::with_capacity(n);

    for i in 0..n
    {
        names.push(format!("User{}", i % 100_000));
        ages.push((i % 60 + 18) as u32);
        weights.push(50.0 + ((i % 50) as f64) * 0.5);
        heights.push(1.50 + ((i % 50) as f64) * 0.01);
    }

    // 2) DataFrame
    let df = DataFrame::new(vec![
        Series::new("name".into(), names).into(),
        Series::new("age".into(), ages).into(),
        Series::new("weight".into(), weights).into(),
        Series::new("height".into(), heights).into(),
    ])?;

    #[derive(Serialize, Deserialize)]
    struct Wrapper
    {
        total: usize,
        data: JsonValue,
    }

    // 4.1) Conta quantas linhas vieram
    let total = df.height();

    // 5) Serializa data + total
    let data_json: JsonValue = df_to_json_each_column(&df)?; // seu array de objetos

    // 5) Serializa o struct DIRETAMENTE
    let wrapper = Wrapper { total, data: data_json };
    let body_str = serde_json::to_string(&wrapper)?;

    // 6) Monta a resposta HTTP
    let response = Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(body_str))?;
    Ok(response)
}

async fn get_users() -> Result<Response, AppError>
{
    let mut df = DataFrame::new(vec![
        Series::new("name".into(), &["John", "Jane", "Jim", "Jill"]).into(),
        Series::new("age".into(), &[30, 25, 35, 28]).into(),
        Series::new("city".into(), &["New York", "Los Angeles", "Chicago", "Houston"]).into(),
    ])?;

    let json_str = df_to_json_each_row(&mut df)?;
    Ok(Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(json_str))
        .unwrap())
}
