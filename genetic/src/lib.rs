use std::fmt::Debug;

pub mod adaptation;
pub mod evolution;
pub mod selection;

pub type Genome = Vec<u8>;

#[derive(Debug, Clone, PartialEq)]
pub struct Evaluation {
    pub genome: Genome,
    pub fitness: f32,
}

impl From<Genome> for Evaluation {
    fn from(genome: Genome) -> Self {
        Self {
            genome,
            fitness: 0f32,
        }
    }
}

pub trait IntoEvaluations {
    fn into_evaluations(self) -> impl Iterator<Item = Evaluation>
    where
        Self: Sized; // Ensure that Self has a known size
}

impl<I> IntoEvaluations for I
where
    I: Iterator<Item = Genome>,
{
    fn into_evaluations(self) -> impl Iterator<Item = Evaluation> {
        self.map(Evaluation::from)
    }
}

#[cfg(test)]
mod tests {
    use common_test::get_seeded_rng;
    use rand::Rng;

    use super::{Evaluation, Genome, IntoEvaluations};

    #[test]
    fn test_from() {
        let genome = vec![3];
        let result = Evaluation::from(genome.clone());

        assert_eq!(
            Evaluation {
                genome,
                fitness: 0f32
            },
            result
        );
    }

    #[test]
    fn test_to_evaluations() {
        // Given
        let mut rng = get_seeded_rng().unwrap();
        let size = rng.gen_range(0usize..10);
        let genomes: Vec<Genome> = (0..size).map(|_| vec![rng.gen()]).collect();

        // When
        let result: Vec<Evaluation> = genomes.clone().into_iter().into_evaluations().collect();

        // Then
        assert_eq!(result.len(), genomes.len());
        let result_states = result.iter().map(|e| e.genome.clone()).collect::<Vec<_>>();
        assert_eq!(result_states, genomes);
        assert!(result.iter().map(|e| e.fitness).all(|x| x == 0f32));
    }
}
