use std::fmt::Debug;

pub mod evolution;
pub mod selection;
pub mod individual;

#[derive(Debug, Clone)]
pub struct Evaluation<State> {
    pub state: State,
    pub fitness: f32,
}

