use actix_web::{web, App, HttpResponse, HttpServer, Responder};
use futures::executor::block_on;
use genetic::{evolution::EvolutionEngine, selection::SelectionType};
use serde::Deserialize;
use strategies::my_strategy::MyStrategy;

#[derive(Deserialize)]
struct Parameters {
    target: Option<String>,
}

async fn hello_world(parameters: web::Query<Parameters>) -> impl Responder {
    let target = parameters.target.clone().unwrap_or("florent".to_string());
    let bytes = target.as_bytes();
    let threshold = bytes.len() as f32;

    let mut runner = EvolutionEngine::new(SelectionType::Ranking, 128, |_, fitnesses| {
        fitnesses.iter().any(|&fitness| fitness >= threshold)
    });
    //let observer = Rc::new(MyObserver::new());
    //runner.register_observer(observer.clone());

    let result = block_on(runner.run(&MyStrategy::from(bytes)));

    //runner.unregister_observer(observer);

    match result {
        Ok(infos) => HttpResponse::Ok().body(format!(
            "{}-{:?}",
            infos.generation,
            infos
                .evaluations
                .iter()
                .enumerate()
                .filter(|e| e.1.fitness >= threshold)
                .map(|e| (e.0, unsafe {
                    String::from_utf8_unchecked(e.1.state.value.clone())
                }))
                .collect::<Vec<_>>()
        )),
        Err(err) => HttpResponse::InternalServerError().body(err.to_string()),
    }
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    HttpServer::new(|| {
        App::new().route("/", web::get().to(hello_world)) // Define a route for the hello world handler
    })
    .bind("127.0.0.1:8080")? // Bind the server to an address and port
    .run()
    .await
}
