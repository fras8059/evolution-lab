mod rng_wrapper;
mod select_by_chance;
mod select_by_rank;
mod select_by_tournament;
mod select_by_weight;

use rand::Rng;
use rng_wrapper::Random;
use select_by_chance::select_by_chance;
use select_by_rank::select_by_rank;
use select_by_tournament::select_by_tournament;
use select_by_weight::select_by_weight;
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::Evaluation;

#[derive(Error, Debug, PartialEq)]
pub enum SelectionError {
    #[error("Unable to select by weight: {0}")]
    InvalidWeights(String),
    #[error("Unable to select {0} genome(s) whereas only {1} is(are) available")]
    OutOfRange(usize, usize),
    #[error("Unable to select by rank {0} genome(s) whereas the max rank is {1}")]
    OutOfRank(usize, usize),
}

pub type SelectionResult = Result<Vec<usize>, SelectionError>;

#[derive(Copy, Clone, Debug, Serialize, Deserialize, PartialEq, Default)]
pub enum SelectionType {
    Chance,
    Ranking(usize),
    Tournament(usize),
    #[default]
    Weight,
}

pub fn select(
    evaluations: &[Evaluation],
    selection_count: usize,
    selection_type: SelectionType,
    rng: &mut impl Rng,
) -> SelectionResult {
    let mut random = Random::new(rng);
    match selection_type {
        SelectionType::Chance => select_by_chance(evaluations, selection_count, &mut random),
        SelectionType::Ranking(max_rank) => {
            select_by_rank(evaluations, selection_count, max_rank, &mut random)
        }
        SelectionType::Tournament(pool_size) => {
            select_by_tournament(evaluations, selection_count, pool_size, &mut random)
        }
        SelectionType::Weight => select_by_weight(evaluations, selection_count, &mut random),
    }
}

pub fn select_couples(
    evaluations: &[Evaluation],
    couples_count: usize,
    selection_type: SelectionType,
    rng: &mut impl Rng,
) -> Result<Vec<(usize, usize)>, SelectionError> {
    let mut random = Random::new(rng);
    let mut selector: Box<dyn FnMut() -> Result<Vec<_>, SelectionError>> = match selection_type {
        SelectionType::Chance => Box::new(|| select_by_chance(evaluations, 2, &mut random)),
        SelectionType::Ranking(max_rank) => {
            Box::new(move || select_by_rank(evaluations, 2, max_rank, &mut random))
        }
        SelectionType::Tournament(pool_size) => {
            Box::new(move || select_by_tournament(evaluations, 2, pool_size, &mut random))
        }
        SelectionType::Weight => Box::new(|| select_by_weight(evaluations, 2, &mut random)),
    };

    (0..couples_count)
        .map(|_| selector().map(|arr| (arr[0], arr[1])))
        .collect()
}

#[cfg(test)]
mod tests {
    use common_test::get_seeded_rng;

    use crate::{
        selection::{
            rng_wrapper::Random, select_by_chance::select_by_chance,
            select_by_rank::select_by_rank, select_by_tournament::select_by_tournament,
            select_by_weight::select_by_weight, SelectionType,
        },
        Evaluation,
    };

    use super::{select, select_couples};

    #[test]
    fn test_select() {
        let evaluations = vec![
            Evaluation {
                genome: vec![3],
                fitness: 0.1,
            },
            Evaluation {
                genome: vec![5],
                fitness: 0.4,
            },
            Evaluation {
                genome: vec![4],
                fitness: 0.5,
            },
            Evaluation {
                genome: vec![8],
                fitness: 0.9,
            },
        ];
        let max_rank = 3;
        let selection_count = 3;
        let pool_size = 2;

        // Given
        let mut rng = get_seeded_rng().unwrap();

        // When
        let result = select(
            &evaluations,
            selection_count,
            SelectionType::Chance,
            &mut rng,
        )
        .unwrap();

        // Then
        assert_eq!(
            selection_count,
            result.len(),
            "Should return the required count for SelectionType::Chance"
        );
        assert_eq!(
            select_by_chance(
                &evaluations,
                selection_count,
                &mut Random::new(&mut get_seeded_rng().unwrap())
            )
            .unwrap(),
            result,
            "Should use select_by_chance to match selection_type"
        );

        // Given
        let mut rng = get_seeded_rng().unwrap();

        // When
        let result = select(
            &evaluations,
            selection_count,
            SelectionType::Ranking(max_rank),
            &mut rng,
        )
        .unwrap();

        // Then
        assert_eq!(
            selection_count,
            result.len(),
            "Should return the required count for SelectionType::Ranking(_)"
        );
        assert_eq!(
            select_by_rank(
                &evaluations,
                selection_count,
                max_rank,
                &mut Random::new(&mut get_seeded_rng().unwrap())
            )
            .unwrap(),
            result,
            "Should use select_by_rank to match selection_type"
        );

        // Given
        let mut rng = get_seeded_rng().unwrap();

        // When
        let result = select(
            &evaluations,
            selection_count,
            SelectionType::Tournament(pool_size),
            &mut rng,
        )
        .unwrap();

        // Then
        assert_eq!(
            selection_count,
            result.len(),
            "Should return the required count for SelectionType::Tournament(_)"
        );
        assert_eq!(
            select_by_tournament(
                &evaluations,
                selection_count,
                pool_size,
                &mut Random::new(&mut get_seeded_rng().unwrap())
            )
            .unwrap(),
            result,
            "Should use select_by_tournament to match selection_type"
        );

        // Given
        let mut rng = get_seeded_rng().unwrap();

        // When
        let result = select(
            &evaluations,
            selection_count,
            SelectionType::Weight,
            &mut rng,
        )
        .unwrap();

        // Then
        assert_eq!(
            selection_count,
            result.len(),
            "Should return the required count for SelectionType::Weight"
        );
        assert_eq!(
            select_by_weight(
                &evaluations,
                selection_count,
                &mut Random::new(&mut get_seeded_rng().unwrap())
            )
            .unwrap(),
            result,
            "Should use select_by_weight to match selection_type"
        );
    }

    #[test]
    fn test_select_couples() {
        let evaluations = vec![
            Evaluation {
                genome: vec![3],
                fitness: 0.1,
            },
            Evaluation {
                genome: vec![5],
                fitness: 0.4,
            },
            Evaluation {
                genome: vec![4],
                fitness: 0.5,
            },
            Evaluation {
                genome: vec![8],
                fitness: 0.9,
            },
        ];
        let max_rank = 3;
        let couples_count = 3;
        let pool_size = 2;

        // Given
        let mut rng = get_seeded_rng().unwrap();

        // When
        let result =
            select_couples(&evaluations, couples_count, SelectionType::Chance, &mut rng).unwrap();

        // Then
        assert_eq!(
            couples_count,
            result.len(),
            "Should return the required count for SelectionType::Chance"
        );
        assert_eq!(
            {
                let mut rng = &mut get_seeded_rng().unwrap();
                let mut random = Random::new(&mut rng);
                (0..couples_count)
                    .map(|_| {
                        select_by_chance(&evaluations, 2, &mut random)
                            .map(|couple| (couple[0], couple[1]))
                            .unwrap()
                    })
                    .collect::<Vec<_>>()
            },
            result,
            "Should use select_by_chance to match selection_type"
        );

        // Given
        let mut rng = get_seeded_rng().unwrap();

        // When
        let result = select_couples(
            &evaluations,
            couples_count,
            SelectionType::Ranking(max_rank),
            &mut rng,
        )
        .unwrap();

        // Then
        assert_eq!(
            couples_count,
            result.len(),
            "Should return the required count for SelectionType::Ranking(_)"
        );
        assert_eq!(
            {
                let mut rng = &mut get_seeded_rng().unwrap();
                let mut random = Random::new(&mut rng);
                (0..couples_count)
                    .map(|_| {
                        select_by_rank(&evaluations, 2, max_rank, &mut random)
                            .map(|couple| (couple[0], couple[1]))
                            .unwrap()
                    })
                    .collect::<Vec<_>>()
            },
            result,
            "Should use select_by_rank to match selection_type"
        );

        // Given
        let mut rng = get_seeded_rng().unwrap();

        // When
        let result = select_couples(
            &evaluations,
            couples_count,
            SelectionType::Tournament(pool_size),
            &mut rng,
        )
        .unwrap();

        // Then
        assert_eq!(
            couples_count,
            result.len(),
            "Should return the required count for SelectionType::Tournament(_)"
        );
        assert_eq!(
            {
                let mut rng = &mut get_seeded_rng().unwrap();
                let mut random = Random::new(&mut rng);
                (0..couples_count)
                    .map(|_| {
                        select_by_tournament(&evaluations, 2, pool_size, &mut random)
                            .map(|couple| (couple[0], couple[1]))
                            .unwrap()
                    })
                    .collect::<Vec<_>>()
            },
            result,
            "Should use select_by_tournament to match selection_type"
        );

        // Given
        let mut rng = get_seeded_rng().unwrap();

        // When
        let result =
            select_couples(&evaluations, couples_count, SelectionType::Weight, &mut rng).unwrap();

        // Then
        assert_eq!(
            couples_count,
            result.len(),
            "Should return the required count for SelectionType::Weight"
        );
        assert_eq!(
            {
                let mut rng = &mut get_seeded_rng().unwrap();
                let mut random = Random::new(&mut rng);
                (0..couples_count)
                    .map(|_| {
                        select_by_weight(&evaluations, 2, &mut random)
                            .map(|couple| (couple[0], couple[1]))
                            .unwrap()
                    })
                    .collect::<Vec<_>>()
            },
            result,
            "Should use select_by_weight to match selection_type"
        );
    }
}
