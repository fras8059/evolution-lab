use std::{fmt::Debug, io::Error, net::ToSocketAddrs};

use common::subject_observer::Observer;
use dipstick::{Graphite, Input};
use genetic::evolution::{EventType, EvolutionEngine};

use super::{MAX, MY_PROXY};

pub struct GraphiteGateway {}

impl GraphiteGateway {
    pub fn new<A>(address: A) -> Result<Self, Error>
    where
        A: ToSocketAddrs + Debug + Clone,
    {
        let graphite_scope = Graphite::send_to(address)?.metrics();
        MY_PROXY.target(graphite_scope);
        Ok(GraphiteGateway {})
    }
}

impl Observer<EvolutionEngine, EventType> for GraphiteGateway {
    fn update(&self, source: &EvolutionEngine, event: EventType) {
        if event == EventType::Evaluated {
            let snapshot = source.snapshot();
            if let Some(max_fitness) = snapshot
                .evaluations
                .iter()
                .map(|e| e.fitness)
                .reduce(f32::max)
            {
                MAX.value(max_fitness);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::GraphiteGateway;

    #[test]
    fn test_graphite_gateway_new() {
        // When
        let result = GraphiteGateway::new("");
        // Then
        assert!(
            matches!(result, Err(_)),
            "Should failed when adress is not valid"
        );

        // When
        let result = GraphiteGateway::new("127.0.0.1:8125");
        // Then
        assert!(
            matches!(result, Ok(_)),
            "Should succeed when adress is valid"
        );
    }

    #[test]
    #[ignore = "todo"]
    fn test_graphite_gateway_update() {
        todo!()
    }
}
