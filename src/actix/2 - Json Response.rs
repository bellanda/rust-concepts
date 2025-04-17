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
    let df: DataFrame = df!(
        "name" => ["Alice Archer", "Ben Brown", "Chloe Cooper", "Daniel Donovan"],
        "birthdate" => [
            NaiveDate::from_ymd_opt(1997, 1, 10).unwrap(),
            NaiveDate::from_ymd_opt(1985, 2, 15).unwrap(),
            NaiveDate::from_ymd_opt(1983, 3, 22).unwrap(),
            NaiveDate::from_ymd_opt(1981, 4, 30).unwrap(),
        ],
        "weight" => [57.9, 72.5, 53.6, 83.1],  // (kg)
        "height" => [1.56, 1.77, 1.65, 1.75],  // (m)
    )
    .unwrap();

    let result = df
        .clone()
        .lazy()
        .select([
            col("name"),
            col("birthdate").dt().year().alias("birth_year"),
            (col("weight") / col("height").pow(2)).alias("bmi"),
        ])
        .collect()?;
    println!("{}", result);

    // serialização para JSON (pode falhar e virar ApiError::Json)
    let json = df_to_json_each_column(&df)?.to_string();

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
    .bind("127.0.0.1:8080")?
    .run()
    .await
}
