mod evolution_engine;

pub use evolution_engine::EvolutionEngine;

use crate::{selection::SelectionError, Evaluation};

#[derive(Debug, Clone, PartialEq)]
pub enum EventType {
    NewGeneration,
    Evaluation,
}

pub type EvolutionResult<State> = Result<Snapshot<State>, SelectionError>;

#[derive(Debug, Clone)]
pub struct Snapshot<State> {
    pub generation: u64,
    pub evaluations: Vec<Evaluation<State>>,
}
