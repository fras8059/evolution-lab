use std::cmp::{min, Ordering};

use crate::{
    selection::{SelectionError, SelectionResult},
    Evaluation,
};

use super::rng_wrapper::RngWrapper;

pub fn select_by_rank(
    evaluations: &[Evaluation],
    expected_count: usize,
    max_rank: usize,
    rng: &mut impl RngWrapper,
) -> SelectionResult {
    // Cannot select above max_rank
    if expected_count > max_rank {
        return Err(SelectionError::OutOfRank(expected_count, max_rank));
    }

    let len = evaluations.len();

    // Cannot select above evaluations count
    if expected_count > len {
        return Err(SelectionError::OutOfRange(expected_count, len));
    }

    let selected_indexes = if expected_count > 0 {
        let mut indexes: Vec<usize> = (0..len).collect();
        indexes.sort_by(|&a, &b| {
            evaluations[b]
                .fitness
                .partial_cmp(&evaluations[a].fitness)
                .unwrap_or(Ordering::Equal)
        });
        let max_rank = min(max_rank, len);
        let selection_count = min(expected_count, max_rank - 1);
        for i in 0..selection_count {
            let selected_index = rng.gen_range(i..max_rank);
            indexes.swap(i, selected_index);
        }
        indexes[0..expected_count].to_vec()
    } else {
        vec![]
    };

    Ok(selected_indexes)
}

#[cfg(test)]
mod tests {
    use super::select_by_rank;

    use crate::{
        selection::{rng_wrapper::test_utils::RngTest, SelectionError},
        Evaluation,
    };

    #[test]
    fn select_by_rank_should_return_result() {
        let evaluations = vec![
            Evaluation {
                genome: vec![1],
                fitness: 2.0,
            },
            Evaluation {
                genome: vec![2],
                fitness: 5.0,
            },
            Evaluation {
                genome: vec![3],
                fitness: 1.0,
            },
            Evaluation {
                genome: vec![4],
                fitness: 1.0,
            },
        ];

        let mut rng_mock = RngTest::with_samples(vec![2, 2, 1, 2]);
        let result = select_by_rank(&evaluations, 3, 3, &mut rng_mock);
        assert_eq!(result, Ok(vec![2, 1, 0]));
        let result = select_by_rank(&evaluations, 2, 3, &mut rng_mock);
        assert_eq!(result, Ok(vec![0, 2]));
    }

    #[test]
    fn select_by_rank_should_return_error_when_not_valid_expected_count() {
        let evaluations = vec![
            Evaluation {
                genome: vec![1],
                fitness: 1.0,
            },
            Evaluation {
                genome: vec![2],
                fitness: 1.0,
            },
        ];

        let mut rng_mock = RngTest::with_samples(vec![2, 0, 0, 1]);
        let result = select_by_rank(&evaluations, 3, 3, &mut rng_mock);
        assert_eq!(
            result,
            Err(SelectionError::OutOfRange(3, 2)),
            "expected_count should be lesser or equal to evaluations size"
        );

        let result = select_by_rank(&evaluations, 3, 2, &mut rng_mock);
        assert_eq!(
            result,
            Err(SelectionError::OutOfRank(3, 2)),
            "expected_count should be lesser or equal to max_rank"
        );
    }

    #[test]
    fn select_by_rank_should_return_empty_collection_when_expected_count_is_0() {
        let evaluations = vec![Evaluation {
            genome: vec![1],
            fitness: 1.0,
        }];

        let mut rng_mock = RngTest::new();
        let result = select_by_rank(&evaluations, 0, 1, &mut rng_mock);
        assert_eq!(result, Ok(vec![]));
    }
}
