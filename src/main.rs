use ::polars::prelude::*;
use actix::api_errors::ApiError;
use actix_web::get;
use actix_web::post;
use actix_web::web;
use actix_web::App;
use actix_web::HttpResponse;
use actix_web::HttpServer;
use actix_web::Responder;
use actix_web::Result;
use chrono::prelude::*;
use serde::Deserialize;
use serde::Serialize;
use utils::polars_df_to_json::df_to_json_each_column;
use utils::polars_df_to_json::df_to_json_each_row; // o enum acima
mod actix;
mod utils;

#[derive(Serialize, Deserialize, Debug)]
struct Dados
{
    campo1: String,
    campo2: i32,
    // Adicione mais campos conforme necessário (o JSON pode ser grande e complexo)
}

#[derive(Serialize, Deserialize)]
struct User
{
    name: String,
    email: String,
    age: u32,
    names: Vec<String>,
}

// Endpoint que recebe JSON via POST e retorna o mesmo JSON
#[post("/echo")]
async fn echo_json(dados: web::Json<Dados>) -> impl Responder
{
    println!("Recebido: {:?}", dados);
    web::Json(dados.into_inner()) // Retorna o mesmo JSON
}

#[get("/user")]
async fn user() -> impl Responder
{
    // Example user data
    let user = User {
        name: "John Doe".to_string(),
        email: "john@example.com".to_string(),
        age: 30,
        names: vec!["John".to_string(), "Doe".to_string()],
    };

    web::Json(user)
}

#[get("/users-df-json")]
async fn users_df_json() -> Result<HttpResponse, ApiError>
{
    // 1) Simula 100.000 linhas de dados
    let n = 100_000;
    let mut names = Vec::with_capacity(n);
    let mut ages = Vec::with_capacity(n);
    let mut weights = Vec::with_capacity(n);
    let mut heights = Vec::with_capacity(n);

    for i in 0..n
    {
        names.push(format!("User{}", i % 1_000));
        ages.push((i % 60 + 18) as u32);
        weights.push(50.0 + ((i % 50) as f64) * 0.5);
        heights.push(1.50 + ((i % 50) as f64) * 0.01);
    }

    // 2) Cria o DataFrame a partir de Series
    let df = DataFrame::new(vec![
        Series::new("name".into(), names).into(),
        Series::new("age".into(), ages).into(),
        Series::new("weight".into(), weights).into(),
        Series::new("height".into(), heights).into(),
    ])?;

    // 3) Pipeline Lazy com filtro mais generoso e agregações
    let lazy_df = df
        .lazy()
        // filtra quem tem mais de 30 anos
        .filter(col("age").gt(lit(30u32)))
        // calcula BMI
        .with_column((col("weight") / col("height").pow(2)).alias("bmi"))
        // agrupa por nome
        .group_by(vec![col("name")])
        .agg(vec![
            col("bmi").mean().alias("mean_bmi"),
            col("bmi").std(0).alias("std_bmi"),
            col("age").min().alias("min_age"),
            col("age").max().alias("max_age"),
            col("age").count().alias("count"),
        ])
        // limiar menor para incluir quase todos os grupos
        .filter(col("mean_bmi").gt(lit(20.0)))
        // ordena pelo BMI médio crescente
        .sort(
            vec!["mean_bmi".to_string()],
            SortMultipleOptions::default()
                .with_order_descending(false)
                .with_nulls_last(true),
        )
        // limite alto para capturar entre 1.000 e 10.000 grupos
        .limit(10_000);

    // 4) Executa só o collect()
    let result_df = lazy_df.collect()?;

    // 5) Serializa para JSON
    let json = df_to_json_each_column(&result_df)?.to_string();
    Ok(HttpResponse::Ok().content_type("application/json").body(json))
}

#[get("/users")]
async fn users() -> Result<HttpResponse, ApiError>
{
    // Criar DataFrame com Series (ao invés de Column)
    let mut df = DataFrame::new(vec![
        Column::new("name".into(), &["John", "Jane", "Jim", "Jill"]),
        Column::new("age".into(), &[30, 25, 35, 28]),
        Column::new("city".into(), &["New York", "Los Angeles", "Chicago", "Houston"]),
    ])
    .expect("Falha ao criar DataFrame");

    // se falhar, vai virar ApiError::Json
    let json = df_to_json_each_row(&mut df)?;
    Ok(HttpResponse::Ok().content_type("application/json").body(json))
}

#[actix_web::main]
async fn main() -> std::io::Result<()>
{
    HttpServer::new(|| {
        App::new()
            .service(echo_json)
            .service(user)
            .service(users_df_json)
            .service(users)
    })
    .workers(4)
    .bind("127.0.0.1:8080")?
    .run()
    .await
}
