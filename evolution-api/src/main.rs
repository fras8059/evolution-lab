use std::rc::Rc;

use actix_web::{web, App, HttpResponse, HttpServer, Responder};
use common::subject_observer::Subject;
use futures::executor::block_on;
use genetic::{
    evolution::{EvolutionEngine, EvolutionSettings},
    selection::SelectionType,
};
use genetic_ext::StatsdGateway;
use rand::thread_rng;
use serde::Deserialize;
use strategies::my_strategy::MyStrategy;

#[derive(Deserialize)]
struct Parameters {
    mutation_rate: Option<f32>,
    population_size: Option<usize>,
    selection_type: Option<SelectionType>,
    target: Option<String>,
}

async fn hello_world(parameters: web::Query<Parameters>) -> impl Responder {
    let mutation_rate = parameters.mutation_rate.unwrap_or(0.1);
    let population_size = parameters.population_size.unwrap_or(100);
    let target = parameters.target.clone().unwrap_or("florent".to_string());
    let selection_type = parameters.selection_type.unwrap_or(SelectionType::Weight);

    let bytes = target.as_bytes();
    let threshold = bytes.len() as f32;

    let settings = EvolutionSettings {
        mutation_rate,
        population_size,
        selection_type,
    };

    let gateway = Rc::new(StatsdGateway::new("graphite:8125"));

    let mut engine = EvolutionEngine::default();
    engine.register_observer(gateway.clone());

    let result = block_on(engine.run(
        &MyStrategy::from_entropy(bytes),
        &settings,
        |_, fitnesses| fitnesses.iter().any(|&fitness| fitness >= threshold),
        &mut thread_rng(),
    ));

    engine.unregister_observer(gateway);

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
