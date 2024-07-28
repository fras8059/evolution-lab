use rand::Rng;

use crate::Evaluation;

use super::{
    rng_wrapper::Random, select_by_chance, select_by_rank, select_by_tournament, select_by_weight,
    Selection, SelectionError,
};

pub struct Selector {
    selection: Selection,
}

impl Selector {
    pub fn new(selection: Selection) -> Self {
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
                Selection::Chance => select_by_chance(evaluations, 2, &mut random),
                Selection::Ranking => select_by_rank(evaluations, 2),
                Selection::Tournament(pool_size) => {
                    select_by_tournament(evaluations, 2, pool_size, &mut random)
                }
                Selection::Weight => select_by_weight(evaluations, 2, &mut random),
            }?;
            couples.push((couple[0], couple[1]))
        }
        Ok(couples)
    }
}
