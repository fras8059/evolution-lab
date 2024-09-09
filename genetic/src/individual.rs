use std::fmt::Debug;

pub trait Strategy {
    type State: Clone + Debug;
    type Score;

    fn challenge(&self, subject: &Self::State) -> Self::Score;

    fn crossover(&self, state1: &Self::State, state2: &Self::State) -> Self::State;

    fn evaluate(&self, score: &Self::Score) -> f32;

    fn get_random_state(&self) -> Self::State;

    fn mutate(&self, state: &mut Self::State);
}
