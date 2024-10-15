use std::{fmt::Debug, io::Error, net::ToSocketAddrs};

use common::subject_observer::Observer;
use dipstick::{Input, Statsd};
use genetic::evolution::{EventType, EvolutionEngine};
use log::trace;

use super::{BEST_EVAL, MY_PROXY};

pub struct StatsdGateway {}

impl StatsdGateway {
    pub fn new<A>(address: A) -> Result<Self, Error>
    where
        A: ToSocketAddrs + Debug + Clone,
    {
        let statsd_scope = Statsd::send_to(address)?.metrics();
        MY_PROXY.target(statsd_scope);
        Ok(StatsdGateway {})
    }
}

impl Observer<EvolutionEngine, EventType> for StatsdGateway {
    fn update(&self, source: &EvolutionEngine, event: EventType) {
        if event == EventType::Evaluated {
            let population_info = source.snapshot();
            if let Some(max_fitness) = population_info
                .evaluations
                .iter()
                .map(|e| e.fitness)
                .reduce(f32::max)
            {
                trace!("Sending best-eval metric: {}", max_fitness);
                BEST_EVAL.value(max_fitness);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::StatsdGateway;

    #[test]
    fn test_statsd_gateway_new() {
        // When
        let result = StatsdGateway::new("");
        // Then
        assert!(
            matches!(result, Err(_)),
            "Should failed when adress is not valid"
        );

        // When
        let result = StatsdGateway::new("127.0.0.1:8125");
        // Then
        assert!(
            matches!(result, Ok(_)),
            "Should succeed when adress is valid"
        );
    }

    #[test]
    #[ignore = "todo"]
    fn test_statsd_gateway_update() {
        todo!()
    }
}
