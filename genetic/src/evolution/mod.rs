mod evolution_engine;

pub use evolution_engine::EvolutionEngine;

use crate::{
    selection::{SelectionError, SelectionType},
    Evaluation,
};

#[derive(Debug, Clone, PartialEq)]
pub enum EventType {
    NewGeneration,
    Evaluation,
}

#[derive(Clone, Debug)]
pub struct EvolutionSettings {
    pub mutation_rate: f32,
    pub population_size: usize,
    pub selection_type: SelectionType,
}

pub type EvolutionResult<State> = Result<Snapshot<State>, SelectionError>;

#[derive(Debug, Clone)]
pub struct Snapshot<State> {
    pub generation: u64,
    pub evaluations: Vec<Evaluation<State>>,
}

impl<State> Default for Snapshot<State> {
    fn default() -> Self {
        Self {
            generation: Default::default(),
            evaluations: Default::default(),
        }
    }
}
