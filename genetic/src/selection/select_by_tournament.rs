use std::cmp::{min, Ordering};

use crate::{
    selection::{SelectionError, SelectionResult},
    Evaluation,
};

use super::rng_wrapper::RngWrapper;

pub fn select_by_tournament<G>(
    evaluations: &[Evaluation<G>],
    expected_count: usize,
    pool_size: usize,
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

    let selected_indexes = if expected_count > 0 {
        let mut indexes = (0..len).collect::<Vec<_>>();
        let selection_count = min(expected_count, len - 1);
        for i in 0..selection_count {
            let mut candidates = indexes[i..len].to_vec();
            let mut pool = Vec::with_capacity(pool_size);
            for _ in 0..pool_size {
                let candidate = rng.gen_range(0..candidates.len());
                pool.push(candidate);
                candidates.swap_remove(candidate);
            }
            let winner = pool
                .iter()
                .copied()
                .max_by(|&a, &b| {
                    evaluations[b]
                        .fitness
                        .partial_cmp(&evaluations[a].fitness)
                        .unwrap_or(Ordering::Equal)
                })
                .unwrap();
            indexes.swap(i, winner);
        }
        indexes[0..expected_count].to_vec()
    } else {
        vec![]
    };

    Ok(selected_indexes)
}

#[cfg(test)]
mod tests {

    use super::select_by_tournament;

    use crate::{
        selection::{rng_wrapper::test_utils::RngTest, SelectionError},
        Evaluation,
    };

    #[test]
    #[ignore = "todo"]
    fn select_by_tournament_should_return_result() {
        // let evaluations = vec![
        //     Evaluation {
        //         genome: 'a',
        //         fitness: 2.0,
        //     },
        //     Evaluation {
        //         genome: 'b',
        //         fitness: 5.0,
        //     },
        //     Evaluation {
        //         genome: 'c',
        //         fitness: 1.0,
        //     },
        //     Evaluation {
        //         genome: 'd',
        //         fitness: 1.0,
        //     },
        // ];

        // let mut rng_mock = RngTest::with_samples(vec![2, 2, 1, 2]);
        // let result = select_by_tournament(&evaluations, 3, 3, &mut rng_mock);
        // assert_eq!(result, Ok(vec![2, 1, 0]));
        // let result = select_by_tournament(&evaluations, 2, 3, &mut rng_mock);
        // assert_eq!(result, Ok(vec![0, 2]));
        todo!()
    }

    #[test]
    fn select_by_tournament_should_return_error_when_not_valid_expected_count() {
        // Given
        let evaluations = vec![
            Evaluation {
                genome: 'a',
                fitness: 1.0,
            },
            Evaluation {
                genome: 'b',
                fitness: 1.0,
            },
        ];
        let mut rng_mock = RngTest::with_samples(vec![2, 0, 0, 1]);

        // When
        let result = select_by_tournament(&evaluations, 3, 3, &mut rng_mock);

        // Then
        assert_eq!(
            result,
            Err(SelectionError::OutOfRange(3, 2)),
            "expected_count should be lesser or equal to evaluations size"
        );
    }

    #[test]
    fn select_by_tournament_should_return_empty_collection_when_expected_count_is_0() {
        // Given
        let evaluations = vec![Evaluation {
            genome: 'a',
            fitness: 1.0,
        }];
        let mut rng_mock = RngTest::new();

        // When
        let result = select_by_tournament(&evaluations, 0, 1, &mut rng_mock);

        // Then
        assert_eq!(result, Ok(vec![]));
    }
}
