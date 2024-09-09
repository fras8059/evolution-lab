mod rng_wrapper;
mod selection_result;
pub mod selector;

use crate::Evaluation;
use rand::distributions::WeightedIndex;
use rng_wrapper::RngWrapper;
pub use selection_result::{SelectionError, SelectionResult};
use serde::Deserialize;
use std::cmp::Ordering;

#[derive(Copy, Clone, Debug, Deserialize)]
pub enum SelectionType {
    Chance,
    Ranking,
    Tournament(usize),
    Weight,
}

pub fn select_by_chance<State>(
    evaluations: &[Evaluation<State>],
    count: usize,
    rng: &mut impl RngWrapper,
) -> SelectionResult
where
    State: Clone,
{
    let len = evaluations.len();
    if count > len {
        return Err(SelectionError::InvalidSelection(count, len));
    }

    let mut indices = (0..len).collect::<Vec<_>>();
    let max_toss = count.min(len - 1);
    for rank in 0..max_toss {
        let index = rng.gen_range(rank..len);
        indices.swap(rank, index);
    }

    Ok(indices[0..count].to_vec())
}

pub fn select_by_rank<State>(evaluations: &[Evaluation<State>], count: usize) -> SelectionResult
where
    State: Clone,
{
    let len = evaluations.len();
    if count > len {
        return Err(SelectionError::InvalidSelection(count, len));
    }

    let mut indices = (0..len).collect::<Vec<_>>();
    if count > 0 {
        indices.sort_by(|&a, &b| {
            evaluations[b]
                .fitness
                .partial_cmp(&evaluations[a].fitness)
                .unwrap_or(Ordering::Equal)
        });
    }
    Ok(indices[0..count].to_vec())
}

pub fn select_by_tournament<State>(
    evaluations: &[Evaluation<State>],
    count: usize,
    pool_size: usize,
    rng: &mut impl RngWrapper,
) -> SelectionResult
where
    State: Clone,
{
    let len = evaluations.len();
    if count > len {
        return Err(SelectionError::InvalidSelection(count, len));
    }

    let mut indices = (0..len).collect::<Vec<_>>();
    let max = count.min(len - 1);
    for rank in 0..max {
        let mut candidates = indices[rank..len].to_vec();
        let mut pool = Vec::with_capacity(pool_size);
        for _ in 0..pool_size {
            let candidate = rng.gen_range(0..candidates.len());
            pool.push(candidate);
            candidates.swap_remove(candidate);
        }
        let winner = pool
            .iter()
            .map(|&i| (i, &evaluations[i]))
            .max_by(|&a, &b| {
                b.1.fitness
                    .partial_cmp(&a.1.fitness)
                    .unwrap_or(Ordering::Equal)
            })
            .unwrap()
            .0;
        indices.swap(rank, winner);
    }
    Ok(indices[0..count].to_vec())
}

pub fn select_by_weight<State>(
    evaluations: &[Evaluation<State>],
    count: usize,
    rng: &mut impl RngWrapper,
) -> SelectionResult
where
    State: Clone,
{
    let len = evaluations.len();
    if count > len {
        return Err(SelectionError::InvalidSelection(count, len));
    }

    let mut indices = (0..len).collect::<Vec<_>>();
    let max_toss = count.min(len - 1);
    if max_toss > 1 {
        let mut values = evaluations.iter().map(|e| e.fitness).collect::<Vec<_>>();
        for rank in 0..max_toss {
            let total = values.iter().map(|&v| 0.01 + v).sum::<f32>();
            let weights = values
                .iter()
                .map(|&v| (0.01 + v) / total)
                .collect::<Vec<_>>();
            let distribution = WeightedIndex::new(weights).expect("Fitnesses are invalid");
            let index = rng.sample_from_distribution(&distribution);
            indices.swap(rank, rank + index);
            values.remove(index);
        }
    }
    Ok(indices[0..count].to_vec())
}

#[cfg(test)]
mod tests {
    use super::RngWrapper;
    use crate::{
        selection::{select_by_chance, select_by_rank, select_by_weight, SelectionError},
        Evaluation,
    };

    struct RngTest {
        samples: Vec<usize>,
        index: usize,
    }

    impl RngTest {
        fn new(samples: Vec<usize>) -> Self {
            RngTest { samples, index: 0 }
        }

        fn next(&mut self) -> usize {
            let result = self.samples[self.index];
            self.index = (self.index + 1) % self.samples.len();
            result
        }
    }

    impl RngWrapper for RngTest {
        fn gen_range<R>(&mut self, _: R) -> usize
        where
            R: rand::distributions::uniform::SampleRange<usize>,
        {
            self.next()
        }

        fn sample_from_distribution(
            &mut self,
            _: &rand::distributions::WeightedIndex<f32>,
        ) -> usize {
            self.next()
        }
    }

    #[test]
    fn test_select_by_chance_should_return_result() {
        let evaluations = vec![
            Evaluation {
                state: 'a',
                fitness: 1.0,
            },
            Evaluation {
                state: 'b',
                fitness: 1.0,
            },
            Evaluation {
                state: 'c',
                fitness: 1.0,
            },
        ];

        let mut rng_mock = RngTest::new(vec![2, 1, 0, 2]);
        let result = select_by_chance(&evaluations, 0, &mut rng_mock);
        assert_eq!(result, Ok(vec![]));
        let result = select_by_chance(&evaluations, 3, &mut rng_mock);
        assert_eq!(result, Ok(vec![2, 1, 0]));
        let result = select_by_chance(&evaluations, 2, &mut rng_mock);
        assert_eq!(result, Ok(vec![0, 2]));
    }

    #[test]
    fn test_select_by_chance_should_return_error_when_not_valid_count() {
        let evaluations = vec![
            Evaluation {
                state: 'a',
                fitness: 1.0,
            },
            Evaluation {
                state: 'b',
                fitness: 1.0,
            },
        ];

        let mut rng_mock = RngTest::new(vec![]);
        let result = select_by_chance(&evaluations, 4, &mut rng_mock);
        assert_eq!(result, Err(SelectionError::InvalidSelection(4, 2)));
    }

    #[test]
    fn test_select_by_rank_should_return_result() {
        let evaluations = vec![
            Evaluation {
                state: 'a',
                fitness: 2.0,
            },
            Evaluation {
                state: 'b',
                fitness: 5.0,
            },
            Evaluation {
                state: 'c',
                fitness: 1.0,
            },
            Evaluation {
                state: 'd',
                fitness: 1.0,
            },
        ];

        let result = select_by_rank(&evaluations, 0);
        assert_eq!(result, Ok(vec![]));
        let result = select_by_rank(&evaluations, 4);
        assert_eq!(result, Ok(vec![1, 0, 2, 3]));
        let result = select_by_rank(&evaluations, 2);
        assert_eq!(result, Ok(vec![1, 0]));
    }

    #[test]
    fn test_select_by_rank_should_return_error_when_not_valid_count() {
        let evaluations = vec![
            Evaluation {
                state: 'a',
                fitness: 1.0,
            },
            Evaluation {
                state: 'b',
                fitness: 1.0,
            },
        ];

        let result = select_by_rank(&evaluations, 3);
        assert_eq!(result, Err(SelectionError::InvalidSelection(3, 2)));
    }

    #[test]
    fn test_select_by_weight_should_return_result() {
        let evaluations = vec![
            Evaluation {
                state: 'a',
                fitness: 1.0,
            },
            Evaluation {
                state: 'b',
                fitness: 1.0,
            },
            Evaluation {
                state: 'c',
                fitness: 1.0,
            },
        ];

        let mut rng_mock = RngTest::new(vec![2, 0, 0, 1]);
        let result = select_by_weight(&evaluations, 0, &mut rng_mock);
        assert_eq!(result, Ok(vec![]));
        let result = select_by_weight(&evaluations, 3, &mut rng_mock);
        assert_eq!(result, Ok(vec![2, 1, 0]));
        let result = select_by_weight(&evaluations, 2, &mut rng_mock);
        assert_eq!(result, Ok(vec![0, 2]));
    }

    #[test]
    fn test_select_by_weight_should_return_error_when_not_valid_count() {
        let evaluations = vec![Evaluation {
            state: 'a',
            fitness: 1.0,
        }];

        let mut rng_mock = RngTest::new(vec![]);
        let result = select_by_weight(&evaluations, 2, &mut rng_mock);
        assert_eq!(result, Err(SelectionError::InvalidSelection(2, 1)));
    }
}
