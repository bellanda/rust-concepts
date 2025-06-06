use xitca_web::handler::handler_service;
use xitca_web::middleware::Logger;
use xitca_web::route::get;
use xitca_web::App;

async fn index() -> &'static str
{
    "Hello world!!"
}

async fn return_json() -> Json<String>
{
    Json("Hello world!!".to_string())
}

fn main() -> std::io::Result<()>
{
    App::new()
        .at("/", get(handler_service(index)))
        .enclosed(Logger::new())
        .serve()
        .bind("localhost:8080")?
        .run()
        .wait()
}
