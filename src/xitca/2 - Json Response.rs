use ::polars::prelude::*;
use serde::Deserialize;
use serde::Serialize;
use utils::polars_df_to_json::df_to_json_each_column;
use utils::polars_df_to_json::df_to_json_each_row;
use xitca_web::handler::handler_service;
use xitca_web::handler::json::Json;
use xitca_web::middleware::Logger;
use xitca_web::route::get;
use xitca_web::App;

mod utils;

#[derive(Serialize, Deserialize)]
struct User
{
    name: String,
    email: String,
    age: u32,
    names: Vec<String>,
}

async fn index() -> &'static str
{
    "Hello world!!"
}

async fn get_user() -> Json<User>
{
    // Example user data
    let user = User {
        name: "John Doe".to_string(),
        email: "john@example.com".to_string(),
        age: 30,
        names: vec!["John".to_string(), "Doe".to_string()],
    };

    Json(user)
}

async fn get_users_df_json() -> String
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
        Ok(json_value) => json_value.to_string(),
        Err(e) => e.to_string(),
    }
}

async fn get_users_df_json_json_writer() -> String
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
        Ok(json_value) => json_value,
        Err(e) => e.to_string(),
    }
}

fn main() -> std::io::Result<()>
{
    println!("Server running on http://localhost:8080");

    App::new()
        .at("/", get(handler_service(index)))
        .at("/user", get(handler_service(get_user)))
        .at("/users-df-json", get(handler_service(get_users_df_json)))
        .at("/users", get(handler_service(get_users_df_json_json_writer)))
        .enclosed(Logger::new())
        .serve()
        .bind("localhost:8080")?
        .run()
        .wait()
}
