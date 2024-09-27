use std::{fmt::Debug, net::ToSocketAddrs};

use common::subject_observer::Observer;
use dipstick::*;
use genetic::evolution::{EventType, EvolutionEngine};

metrics! {
    pub MY_PROXY: Proxy = "Graphite_Proxy" => {
        BEST_EVAL: Gauge = "best-eval";
    }
}

pub struct GraphiteGateway {}

pub struct StatsdGateway {}

impl GraphiteGateway {
    pub fn new<A>(address: A) -> Self
    where
        A: ToSocketAddrs + Debug + Clone,
    {
        let graphite_scope = Graphite::send_to(address).expect("").metrics();
        MY_PROXY.target(graphite_scope);
        GraphiteGateway {}
    }
}

impl StatsdGateway {
    pub fn new<A>(address: A) -> Self
    where
        A: ToSocketAddrs + Debug + Clone,
    {
        let statsd_scope = Statsd::send_to(address).expect("").metrics();
        MY_PROXY.target(statsd_scope);
        StatsdGateway {}
    }
}

impl<State> Observer<EvolutionEngine<State>, EventType> for GraphiteGateway
where
    State: Clone,
{
    fn update(&self, source: &EvolutionEngine<State>, event: EventType) {
        if event == EventType::Evaluated {
            let population_info = source.snapshot();
            if let Some(max_fitness) = population_info
                .evaluations
                .iter()
                .map(|e| e.fitness)
                .reduce(f32::max)
            {
                BEST_EVAL.value(max_fitness);
            }
        }
    }
}

impl<State> Observer<EvolutionEngine<State>, EventType> for StatsdGateway
where
    State: Clone,
{
    fn update(&self, source: &EvolutionEngine<State>, event: EventType) {
        if event == EventType::Evaluated {
            let population_info = source.snapshot();
            if let Some(max_fitness) = population_info
                .evaluations
                .iter()
                .map(|e| e.fitness)
                .reduce(f32::max)
            {
                BEST_EVAL.value(max_fitness);
            }
        }
    }
}
