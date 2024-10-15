use std::rc::Rc;

use actix_web::{
    post,
    web::{Data, Json, ServiceConfig},
    HttpResponse, Responder,
};
use common::subject_observer::Subject;
use futures::executor::block_on;
use genetic::{
    evolution::{EvolutionConfig, EvolutionEngine, GenerationRenewalConfig, GeneticRenewalParam},
    selection::SelectionType,
};
use genetic_ext::gateways::StatsdGateway;
use log::debug;
use rand::thread_rng;
use serde::{Deserialize, Serialize};
use strategies::my_strategy::MyStrategy;
use utoipa::ToSchema;

use crate::config::app::AppConfig;

// #[derive(OpenApi)]
// #[openapi(paths(run), components(schemas(Parameters)))]
// pub struct RunApi;

pub fn configure() -> impl FnOnce(&mut ServiceConfig) {
    |config: &mut ServiceConfig| {
        config.service(run);
    }
}

#[derive(Serialize, Deserialize, ToSchema, Clone, Debug)]
pub struct Parameters {
    crossover_rate: Option<f32>,
    crossover_mutation_rate: Option<f32>,
    crossover_selection_type: Option<SelectionType>,
    population_size: Option<usize>,
    target: Option<String>,
}

#[utoipa::path(
    request_body = Parameters,
//    responses(
//        (status = 201, description = "Todo created successfully", body = Todo),
//        (status = 409, description = "Todo with id already exists", body = ErrorResponse, example = json!(ErrorResponse::Conflict(String::from("id = 1"))))
//    )
)]
#[post("/run")]
pub async fn run(config: Data<AppConfig>, parameters: Json<Parameters>) -> impl Responder {
    let parameters = parameters.into_inner();
    debug!("Starting evolution with parameters: {:?}", parameters);

    let population_size = parameters.population_size.unwrap_or(128);
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
    debug!("Running evolution with configuration: {:?}", settings);

    let gateway =
        Rc::new(StatsdGateway::new((config.statsd_host.clone(), config.statsd_port)).unwrap());

    let mut engine = EvolutionEngine::default();
    engine.register_observer(gateway.clone());

    let result = block_on(engine.start(
        &MyStrategy::new(bytes),
        &settings,
        |_, fitnesses| fitnesses.iter().any(|&fitness| fitness >= threshold),
        &mut thread_rng(),
    ));

    engine.unregister_observer(gateway);

    match result {
        Ok(infos) => {
            debug!("Evolution done");
            HttpResponse::Ok().body(format!(
                "{}-{:?}",
                infos.generation,
                infos
                    .evaluations
                    .iter()
                    .enumerate()
                    .filter(|e| e.1.fitness >= threshold)
                    .map(|e| (e.0, unsafe {
                        String::from_utf8_unchecked(e.1.genome.clone())
                    }))
                    .collect::<Vec<_>>()
            ))
        }
        Err(err) => {
            debug!("Evolution failed");
            HttpResponse::InternalServerError().body(err.to_string())
        }
    }
}
