use rand::Rng;

use crate::Evaluation;

use super::{
    rng_wrapper::Random, select_by_chance, select_by_rank, select_by_tournament, select_by_weight,
    SelectionError, SelectionType,
};

pub struct Selector {
    selection: SelectionType,
}

impl Selector {
    pub fn new(selection: SelectionType) -> Self {
        Selector { selection }
    }

    pub fn select_couples<State: Clone>(
        &self,
        evaluations: &[Evaluation<State>],
        rng: &mut impl Rng,
    ) -> Result<Vec<(usize, usize)>, SelectionError> {
        let mut couples = vec![];
        let mut random = Random::new(rng);
        for _ in 0..evaluations.len() {
            let couple = match self.selection {
                SelectionType::Chance => select_by_chance(evaluations, 2, &mut random),
                SelectionType::Ranking => select_by_rank(evaluations, 2),
                SelectionType::Tournament(pool_size) => {
                    select_by_tournament(evaluations, 2, pool_size, &mut random)
                }
                SelectionType::Weight => select_by_weight(evaluations, 2, &mut random),
            }?;
            couples.push((couple[0], couple[1]))
        }
        Ok(couples)
    }
}
