use ::polars::prelude::*;
use actix_web::get;
use actix_web::post;
use actix_web::web;
use actix_web::App;
use actix_web::HttpResponse;
use actix_web::HttpServer;
use actix_web::Responder;
use serde::Deserialize;
use serde::Serialize;
use utils::polars_df_to_json::df_to_json_each_column;
use utils::polars_df_to_json::df_to_json_each_row;

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
async fn users_df_json() -> impl Responder
{
    // Criar DataFrame com Series (ao invés de Column)
    let df = DataFrame::new(vec![
        Column::new("name".into(), &["John", "Jane", "Jim", "Jill"]),
        Column::new("age".into(), &[30, 25, 35, 28]),
        Column::new("city".into(), &["New York", "Los Angeles", "Chicago", "Houston"]),
    ])
    .expect("Falha ao criar DataFrame");

    match df_to_json_each_column(&df)
    {
        Ok(json_value) => HttpResponse::Ok()
            .content_type("application/json")
            .body(json_value.to_string()),
        Err(e) => HttpResponse::InternalServerError().body(e.to_string()),
    }
}

#[get("/users")]
async fn users() -> impl Responder
{
    // Criar DataFrame com Series (ao invés de Column)
    let mut df = DataFrame::new(vec![
        Column::new("name".into(), &["John", "Jane", "Jim", "Jill"]),
        Column::new("age".into(), &[30, 25, 35, 28]),
        Column::new("city".into(), &["New York", "Los Angeles", "Chicago", "Houston"]),
    ])
    .expect("Falha ao criar DataFrame");

    match df_to_json_each_row(&mut df)
    {
        Ok(json_value) => HttpResponse::Ok().content_type("application/json").body(json_value),
        Err(e) => HttpResponse::InternalServerError().body(e.to_string()),
    }
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
