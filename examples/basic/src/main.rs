use std::{env, rc::Rc};

use dipstick::{Input, InputScope, Log, LogScope};
use genetic::{
    evolution::{EventType, EvolutionEngine},
    selection::SelectionType,
};
use log::{error, info};
use simple_logger::SimpleLogger;
use strategy::{MyState, MyStrategy};

use futures::executor::block_on;

use common::subject_observer::{Observer, Subject};

mod strategy;

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

impl<F> Observer<EvolutionEngine<MyState, F>, EventType> for MyObserver
where
    F: Fn(u64, &[f32]) -> bool,
{
    fn update(&self, source: &EvolutionEngine<MyState, F>, event: EventType) {
        if event == EventType::Evaluation {
            let population_info = source.get_population_info();
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

    let mut runner = EvolutionEngine::new(SelectionType::Ranking, 128, |_, fitnesses| {
        fitnesses.iter().any(|&fitness| fitness >= threshold)
    });
    let observer = Rc::new(MyObserver::new());
    runner.register_observer(observer.clone());

    let result = block_on(runner.run(&MyStrategy::from(bytes)));

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
                    String::from_utf8_unchecked(e.1.state.value.clone())
                }))
                .collect::<Vec<_>>()
        ),
        Err(err) => error!("{}", err),
    };
}
