use std::fmt::Debug;

pub mod evolution;
pub mod selection;

#[derive(Debug, Clone, PartialEq)]
pub struct Evaluation<G> {
    pub genome: G,
    pub fitness: f32,
}

pub trait Strategy {
    type G: Clone + Debug;

    fn crossover(&self, genomes: (&Self::G, &Self::G)) -> Self::G;

    fn evaluate(&self, genome: &Self::G) -> f32;

    fn generate_genome(&self) -> Self::G;

    fn mutate(&self, genome: &mut Self::G, mutation_rate: f32);
}
