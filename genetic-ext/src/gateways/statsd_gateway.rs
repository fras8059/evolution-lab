use std::{fmt::Debug, io::Error, net::ToSocketAddrs};

use common::subject_observer::Observer;
use dipstick::{Input, Statsd};
use genetic::{
    evolution::{EventType, EvolutionEngine},
    Evaluation,
};
use log::trace;

use crate::gateways::{MAX, MEAN, MIN, MY_PROXY, STD_DEV};

pub struct StatsdGateway {
    factor: f32,
}

impl StatsdGateway {
    pub fn new<A>(address: A, factor: f32) -> Result<Self, Error>
    where
        A: ToSocketAddrs + Debug + Clone,
    {
        let statsd_scope = Statsd::send_to(address)?.metrics();
        MY_PROXY.target(statsd_scope);

        Ok(StatsdGateway { factor })
    }

    fn compute_stats(&self, evaluations: &[Evaluation]) -> (f32, f32, f32, f32) {
        let fitness_iter = evaluations.iter().map(|e| e.fitness * self.factor);
        let (min, max, sum, count) = fitness_iter.clone().fold(
            (f32::INFINITY, f32::NEG_INFINITY, 0.0, 0),
            |(min, max, sum, count), value| {
                (min.min(value), max.max(value), sum + value, count + 1)
            },
        );
        let mean = sum / count as f32;
        let variance: f32 = fitness_iter
            .map(|value| (value - mean).powi(2))
            .sum::<f32>()
            / count as f32;
        let std_dev = variance.sqrt();
        (min, max, mean, std_dev)
    }
}

impl Observer<EvolutionEngine, EventType> for StatsdGateway {
    fn update(&self, source: &EvolutionEngine, event: EventType) {
        if event == EventType::Evaluated {
            let snapshot = source.snapshot();
            let (min, max, mean, std_dev) = self.compute_stats(&snapshot.evaluations);

            trace!("Sending metrics for generation {}: min={min}, max={max}, mean={mean}, std-dev={std_dev}", snapshot.generation);
            MIN.value(min);
            MAX.value(max);
            MEAN.value(mean);
            STD_DEV.value(std_dev);
        }
    }
}

#[cfg(test)]
mod tests {
    use genetic::Evaluation;

    use super::StatsdGateway;

    #[test]
    fn test_statsd_gateway_new() {
        let factor = 1000.0;

        // When
        let result = StatsdGateway::new("", factor);
        // Then
        assert!(
            matches!(result, Err(_)),
            "Should failed when adress is not valid"
        );

        // When
        let result = StatsdGateway::new("127.0.0.1:8125", factor);
        // Then
        assert!(
            matches!(result, Ok(_)),
            "Should succeed when adress is valid"
        );
        let result = result.unwrap();
        assert_eq!(factor, result.factor);
    }

    #[test]
    fn test_compute_stats() {
        // Given
        let factor = 1.0;
        let gateway = StatsdGateway::new("127.0.0.1:8125", factor).unwrap();
        let evaluations = vec![
            Evaluation {
                fitness: 1.0,
                genome: vec![],
            },
            Evaluation {
                fitness: 2.0,
                genome: vec![],
            },
            Evaluation {
                fitness: 3.0,
                genome: vec![],
            },
        ];

        // When
        let result = gateway.compute_stats(&evaluations);

        // Then
        assert_eq!(1.0, result.0);
        assert_eq!(3.0, result.1);
        assert_eq!(2.0, result.2);
        assert_eq!(0.81649658092773, result.3);
    }

    #[test]
    #[ignore = "todo"]
    fn test_statsd_gateway_update() {
        todo!()
    }
}
