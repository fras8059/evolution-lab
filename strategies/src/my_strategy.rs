use genetic::Strategy;
use std::{cell::RefCell, fmt::Debug};

use rand::{distributions::Standard, rngs::StdRng, Rng, SeedableRng};

#[derive(Clone, Debug, Default)]
pub struct MyState {
    pub value: Vec<u8>,
}

pub struct MyStrategy {
    target: Vec<u8>,
    rng: RefCell<StdRng>,
}

impl MyStrategy {
    pub fn from(target: &[u8], seed: u64) -> Self {
        MyStrategy {
            target: target.to_vec(),
            rng: RefCell::new(StdRng::seed_from_u64(seed)),
        }
    }

    pub fn from_entropy(target: &[u8]) -> Self {
        MyStrategy {
            target: target.to_vec(),
            rng: RefCell::new(StdRng::from_entropy()),
        }
    }
}

impl Strategy for MyStrategy {
    type G = MyState;

    fn crossover(&self, genomes: (&Self::G, &Self::G)) -> Self::G {
        let mut rng = self.rng.borrow_mut();
        let crossover_point = rng.gen_range(0..self.target.len());
        MyState {
            value: [
                &genomes.0.value[..crossover_point],
                &genomes.1.value[crossover_point..],
            ]
            .concat(),
        }
    }

    fn evaluate(&self, genome: &Self::G) -> f32 {
        genome
            .value
            .iter()
            .zip(self.target.iter())
            .filter(|(a, b)| a == b)
            .count() as f32
    }

    fn generate_genome(&self) -> Self::G {
        let mut rng = self.rng.borrow_mut();
        MyState {
            value: (&mut *rng)
                .sample_iter(Standard)
                .take(self.target.len())
                .collect::<Vec<u8>>(),
        }
    }

    fn mutate(&self, genome: &mut Self::G, mutation_rate: f32) {
        let mut rng = self.rng.borrow_mut();
        genome.value = genome
            .value
            .iter()
            .map(|&c| {
                if rng.gen::<f32>() < mutation_rate {
                    rng.gen::<u8>()
                } else {
                    c
                }
            })
            .collect::<Vec<_>>();
    }
}
