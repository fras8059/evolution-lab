use std::rc::Rc;

use actix_web::{web, App, HttpResponse, HttpServer, Responder};
use common::subject_observer::Subject;
use futures::executor::block_on;
use genetic::{
    evolution::{EvolutionConfig, EvolutionEngine, GenerationRenewalConfig, GeneticRenewalParam},
    selection::SelectionType,
};
use genetic_ext::gateways::StatsdGateway;
use rand::thread_rng;
use serde::Deserialize;
use strategies::my_strategy::MyStrategy;

#[derive(Deserialize)]
struct Parameters {
    crossover_rate: Option<f32>,
    crossover_mutation_rate: Option<f32>,
    crossover_selection_type: Option<SelectionType>,
    population_size: Option<usize>,
    target: Option<String>,
}

async fn hello_world(parameters: web::Query<Parameters>) -> impl Responder {
    let population_size = parameters.population_size.unwrap_or(100);
    let target = parameters.target.clone().unwrap_or("florent".to_string());

    let bytes = target.as_bytes();
    let threshold = bytes.len() as f32;

    let settings = EvolutionConfig {
        generation_renewal_config: Some(GenerationRenewalConfig {
            cloning: None,
            crossover: Some(GeneticRenewalParam {
                mutation_rate: parameters.crossover_mutation_rate,
                ratio: parameters.crossover_rate.unwrap_or(1.0),
                selection_type: parameters
                    .crossover_selection_type
                    .unwrap_or(SelectionType::Weight),
            }),
        }),
        population_size,
    };

    let gateway = Rc::new(StatsdGateway::new("graphite:8125").unwrap());

    let mut engine = EvolutionEngine::default();
    engine.register_observer(gateway.clone());

    let result = block_on(engine.start(
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
                    String::from_utf8_unchecked(e.1.genome.value.clone())
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
