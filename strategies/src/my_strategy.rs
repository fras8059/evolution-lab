use genetic::individual::Strategy;
use std::{cell::RefCell, fmt::Debug};

use rand::{distributions::Standard, rngs::StdRng, Rng, SeedableRng};

#[derive(Clone, Debug)]
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
    type Score = Vec<u8>;
    type State = MyState;

    fn challenge(&self, subject: &Self::State) -> Self::Score {
        subject.value.clone()
    }

    fn crossover(&self, state1: &Self::State, state2: &Self::State) -> Self::State {
        let mut rng = self.rng.borrow_mut();
        let crossover_point = rng.gen_range(0..self.target.len());
        MyState {
            value: [
                &state1.value[..crossover_point],
                &state2.value[crossover_point..],
            ]
            .concat(),
        }
    }

    fn evaluate(&self, score: &Self::Score) -> f32 {
        score
            .iter()
            .zip(self.target.iter())
            .filter(|(a, b)| a == b)
            .count() as f32
    }

    fn mutate(&self, state: &mut Self::State) {
        let mut rng = self.rng.borrow_mut();
        state.value = state
            .value
            .iter()
            .map(|&c| {
                if rng.gen::<f64>() < 0.01f64 {
                    rng.gen::<u8>()
                } else {
                    c
                }
            })
            .collect::<Vec<_>>();
    }

    fn init_states(&self, population_size: usize) -> Vec<Self::State> {
        let mut rng = self.rng.borrow_mut();
        (0..population_size)
            .map(|_| MyState {
                value: (&mut *rng)
                    .sample_iter(Standard)
                    .take(self.target.len())
                    .collect::<Vec<u8>>(),
            })
            .collect::<Vec<_>>()
    }
}