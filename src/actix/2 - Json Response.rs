use actix_web::{get, web, Responder, Result};
use serde::Serialize;

#[derive(Serialize)]
struct MyObj {
    name: String,
    age: Option<u8>,
}

#[get("/")]
async fn index() -> Result<impl Responder> {
    let obj = MyObj {
        name: "Gustavo".to_string(),
        age: Some(20),
    };
    Ok(web::Json(obj))
}

#[get("/a/{name}")]
async fn index_a(name: web::Path<String>) -> Result<impl Responder> {
    let obj = MyObj {
        name: name.to_string(),
        age: None,
    };
    Ok(web::Json(obj))
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    use actix_web::{App, HttpServer};

    HttpServer::new(|| App::new().service(index).service(index_a))
        .bind(("127.0.0.1", 8080))?
        .run()
        .await
}