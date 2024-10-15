use crate::Genome;

pub trait Strategy {
    fn genome_size(&self) -> usize;

    fn evaluate(&self, genome: &Genome) -> f32;
}
