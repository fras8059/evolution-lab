use std::cmp::min;

use crate::{
    selection::{SelectionError, SelectionResult},
    Evaluation,
};

use super::rng_wrapper::RngWrapper;

pub fn select_by_chance(
    evaluations: &[Evaluation],
    expected_count: usize,
    rng: &mut impl RngWrapper,
) -> SelectionResult {
    let len = evaluations.len();

    // Cannot select above evaluations count
    if expected_count > len {
        return Err(SelectionError::OutOfRange(expected_count, len));
    }

    let selected_indexes = if expected_count > 0 {
        let mut indexes: Vec<usize> = (0..len).collect();
        let selection_count = min(expected_count, len - 1);
        for i in 0..selection_count {
            let selected_index = rng.gen_range(i..len);
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

    use super::select_by_chance;

    use crate::{
        selection::{rng_wrapper::test_utils::RngTest, SelectionError},
        Evaluation,
    };

    #[test]
    fn select_by_chance_should_return_result() {
        let evaluations = vec![
            Evaluation {
                genome: vec![1],
                fitness: 1.0,
            },
            Evaluation {
                genome: vec![2],
                fitness: 2.0,
            },
            Evaluation {
                genome: vec![3],
                fitness: 1.0,
            },
        ];

        let mut rng_mock = RngTest::with_samples(vec![2, 1, 0, 2]);
        let result = select_by_chance(&evaluations, 3, &mut rng_mock);
        assert_eq!(result, Ok(vec![2, 1, 0]));
        let result = select_by_chance(&evaluations, 2, &mut rng_mock);
        assert_eq!(result, Ok(vec![0, 2]));
    }

    #[test]
    fn select_by_chance_should_return_error_when_not_valid_expected_count() {
        let evaluations = vec![Evaluation {
            genome: vec![1],
            fitness: 1.0,
        }];

        let mut rng_mock = RngTest::new();
        let result = select_by_chance(&evaluations, 4, &mut rng_mock);
        assert_eq!(result, Err(SelectionError::OutOfRange(4, 1)));
    }

    #[test]
    fn select_by_chance_should_return_empty_collection_when_expected_count_is_0() {
        let evaluations = vec![Evaluation {
            genome: vec![1],
            fitness: 1.0,
        }];

        let mut rng_mock = RngTest::new();
        let result = select_by_chance(&evaluations, 0, &mut rng_mock);
        assert_eq!(result, Ok(vec![]));
    }
}
