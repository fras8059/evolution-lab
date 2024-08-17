use std::fmt::Debug;

pub mod evolution;
pub mod individual;
pub mod selection;

#[derive(Debug, Clone)]
pub struct Evaluation<State> {
    pub state: State,
    pub fitness: f32,
}
