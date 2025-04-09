use actix_web::{post, web, App, HttpServer, Responder};
use serde::{Deserialize, Serialize};

// Estrutura que representa o JSON de entrada/saída
#[derive(Serialize, Deserialize, Debug)]
struct Dados {
    campo1: String,
    campo2: i32,
    // Adicione mais campos conforme necessário (o JSON pode ser grande e complexo)
}

// Endpoint que recebe JSON via POST e retorna o mesmo JSON
#[post("/echo")]
async fn echo_json(dados: web::Json<Dados>) -> impl Responder {
    println!("Recebido: {:?}", dados);
    web::Json(dados.into_inner()) // Retorna o mesmo JSON
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    HttpServer::new(|| {
        App::new()
            .service(echo_json)
    })
    .bind("127.0.0.1:8080")?
    .run()
    .await
}