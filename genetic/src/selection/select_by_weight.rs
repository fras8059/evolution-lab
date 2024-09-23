use std::{cmp::min, collections::VecDeque};

use rand::distributions::WeightedIndex;

use crate::{
    selection::{SelectionError, SelectionResult},
    Evaluation,
};

use super::rng_wrapper::RngWrapper;

const MIN_WEIGHT: f32 = 0.01;

pub fn select_by_weight<G>(
    evaluations: &[Evaluation<G>],
    expected_count: usize,
    rng: &mut impl RngWrapper,
) -> SelectionResult
where
    G: Clone,
{
    let len = evaluations.len();

    // Cannot select above evaluations count
    if expected_count > len {
        return Err(SelectionError::OutOfRange(expected_count, len));
    }

    let selected_indexes = if expected_count > 1 {
        let mut indexes = (0..len).collect::<Vec<_>>();
        let selection_count = min(expected_count, len - 1);
        let mut values = evaluations
            .iter()
            .map(|e| e.fitness)
            .collect::<VecDeque<_>>();
        for i in 0..selection_count {
            let total = values.iter().map(|&v| (MIN_WEIGHT + v) as f64).sum::<f64>();
            let weights = values
                .iter()
                .map(|&v| (MIN_WEIGHT + v) as f64 / total)
                .collect::<Vec<_>>();
            let distribution = WeightedIndex::new(weights)
                .map_err(|e| SelectionError::InvalidWeights(e.to_string()))?;
            let index = rng.sample_from_distribution(&distribution);
            indexes.swap(i, i + index);
            values.swap(0, index);
            values.pop_front();
        }
        indexes[0..expected_count].to_vec()
    } else {
        vec![]
    };

    Ok(selected_indexes)
}

#[cfg(test)]
mod tests {
    use super::select_by_weight;

    use crate::{
        selection::{rng_wrapper::test_utils::RngTest, SelectionError},
        Evaluation,
    };

    #[test]
    fn select_by_weight_should_return_result() {
        let evaluations = vec![
            Evaluation {
                genome: 'a',
                fitness: 1.0,
            },
            Evaluation {
                genome: 'b',
                fitness: 2.0,
            },
            Evaluation {
                genome: 'c',
                fitness: 1.0,
            },
        ];

        let mut rng_mock = RngTest::with_samples(vec![2, 0, 0, 1]);
        let result = select_by_weight(&evaluations, 3, &mut rng_mock);
        assert_eq!(result, Ok(vec![2, 1, 0]));
        let result = select_by_weight(&evaluations, 2, &mut rng_mock);
        assert_eq!(result, Ok(vec![0, 2]));
    }

    #[test]
    fn select_by_weight_should_return_error_when_not_valid_expected_count() {
        let evaluations = vec![Evaluation {
            genome: 'a',
            fitness: 1.0,
        }];

        let mut rng_mock = RngTest::new();
        let result = select_by_weight(&evaluations, 2, &mut rng_mock);
        assert_eq!(result, Err(SelectionError::OutOfRange(2, 1)));
    }

    #[test]
    fn select_by_weight_should_return_empty_collection_when_expected_count_is_0() {
        let evaluations = vec![Evaluation {
            genome: 'a',
            fitness: 1.0,
        }];

        let mut rng_mock = RngTest::new();
        let result = select_by_weight(&evaluations, 0, &mut rng_mock);
        assert_eq!(result, Ok(vec![]));
    }
}
