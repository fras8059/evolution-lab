use genetic::{adaptation::Strategy, Genome};

pub struct MyStrategy {
    target: Genome,
}

impl MyStrategy {
    pub fn new(target: &[u8]) -> Self {
        MyStrategy {
            target: target.to_vec(),
        }
    }
}

impl Strategy for MyStrategy {
    fn genome_size(&self) -> usize {
        self.target.len()
    }

    fn evaluate(&self, genome: &Genome) -> f32 {
        genome
            .iter()
            .zip(self.target.iter())
            .filter(|(a, b)| a == b)
            .count() as f32
            / self.target.len() as f32
    }
}
