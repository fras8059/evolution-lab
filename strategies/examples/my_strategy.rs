use std::{env, rc::Rc};

use dipstick::{Input, InputScope, Log, LogScope};
use genetic::{
    evolution::{
        EventType, EvolutionConfig, EvolutionEngine, GenerationRenewalConfig, GeneticRenewalParam,
    },
    selection::SelectionType,
};
use log::{error, info};
use rand::thread_rng;

use futures::executor::block_on;

use common::subject_observer::{Observer, Subject};
use simple_logger::SimpleLogger;
use strategies::my_strategy::{MyState, MyStrategy};

struct MyObserver {
    log_scope: LogScope,
}

impl MyObserver {
    fn new() -> Self {
        MyObserver {
            log_scope: Log::to_log().level(log::Level::Trace).metrics(),
        }
    }
}

impl Observer<EvolutionEngine<MyState>, EventType> for MyObserver {
    fn update(&self, source: &EvolutionEngine<MyState>, event: EventType) {
        if event == EventType::Evaluated {
            let population_info = source.snapshot();
            //trace!("{:?}:{:?}", event, population_info);
            for (index, evaluation) in population_info.evaluations.iter().enumerate() {
                let gauge = self.log_scope.gauge(format!("fitness_{}", index).as_str());
                gauge.value(evaluation.fitness);
            }
        }
    }
}

fn main() {
    SimpleLogger::new()
        .with_level(log::LevelFilter::Info)
        .env()
        .init()
        .unwrap();

    let target = env::args().nth(1).unwrap_or("florent".to_string());
    let bytes = target.as_bytes();
    let threshold = bytes.len() as f32;

    let settings = EvolutionConfig {
        population_size: 128,
        generation_renewal_config: Some(GenerationRenewalConfig {
            cloning: None,
            crossover: Some(GeneticRenewalParam {
                mutation_rate: None,
                ratio: 1.0,
                selection_type: SelectionType::Weight,
            }),
        }),
    };

    let mut runner = EvolutionEngine::default();
    let observer = Rc::new(MyObserver::new());
    runner.register_observer(observer.clone());

    let result = block_on(runner.start(
        &MyStrategy::from_entropy(bytes),
        &settings,
        |_, fitnesses| fitnesses.iter().any(|&fitness| fitness >= threshold),
        &mut thread_rng(),
    ));

    runner.unregister_observer(observer);

    match result {
        Ok(infos) => info!(
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
        ),
        Err(err) => error!("{}", err),
    };
}
