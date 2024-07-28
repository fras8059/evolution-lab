use std::{env, rc::Rc};

use dipstick::{Input, InputScope, Log, LogScope};
use genetic::{selection::Selection, EventType, PopulationRunner};
use log::{error, info, trace};
use simple_logger::SimpleLogger;
use strategy::{MyState, MyStrategy};

use futures::executor::block_on;

use common::subject_observer::{Observer, Subject};

mod strategy;

struct MyObserver {
    logScope: LogScope,
}

impl MyObserver {
    fn new() -> Self {
        MyObserver {
            logScope: Log::to_log().level(log::Level::Trace).metrics(),
        }
    }
}

impl Observer<PopulationRunner<MyState>, EventType> for MyObserver {
    fn update(&self, source: &PopulationRunner<MyState>, event: EventType) {
        if event == EventType::Evaluation {
            let population_info = source.get_population_info();
            //trace!("{:?}:{:?}", event, population_info);
            for (index, evaluation) in population_info.evaluations.iter().enumerate() {
                let gauge = self.logScope.gauge(format!("fitness_{}", index).as_str());
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

    let mut runner = PopulationRunner::new(Selection::Ranking);
    let observer = Rc::new(MyObserver::new());
    runner.register_observer(observer.clone());

    let target = env::args().nth(1).unwrap_or("florent".to_string());
    let bytes = target.as_bytes();
    let threshold = bytes.len() as f32;
    let result = block_on(runner.run(&MyStrategy::from(bytes), 128, |_, fitnesses| {
        fitnesses.iter().any(|&fitness| fitness >= threshold)
    }));

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
