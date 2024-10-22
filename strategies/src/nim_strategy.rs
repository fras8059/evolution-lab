use anyhow::{bail, Result};
use genetic::adaptation::Strategy;

const MAX_STICK_CHOICE: u8 = 3;
const MIN_STICK_CHOICE: u8 = 1;
const MOD_CHOICE: u8 = MAX_STICK_CHOICE + MIN_STICK_CHOICE;

const CODES_COUNT: u16 = u8::MAX as u16 + 1;
const ACTIONS_COUNT: u8 = MAX_STICK_CHOICE - MIN_STICK_CHOICE + 1;
const ACTIONS_COUNT_U16: u16 = ACTIONS_COUNT as u16;

pub struct NimStrategy {
    best_actions: Vec<u8>,
    normalization_factor: f32,
}

impl NimStrategy {
    pub fn new(stick_count: u8) -> Result<Self> {
        if stick_count <= MIN_STICK_CHOICE {
            bail!("Invalid initial stick count: {stick_count}; It must be greater than {MIN_STICK_CHOICE}");
        }

        let best_actions = get_best_actions(stick_count);
        let normalization_factor = (ACTIONS_COUNT as usize * best_actions.len()) as f32;
        Ok(NimStrategy {
            best_actions,
            normalization_factor,
        })
    }
}

impl Strategy for NimStrategy {
    fn genome_size(&self) -> usize {
        self.best_actions.len()
    }

    fn evaluate(&self, genome: &genetic::Genome) -> f32 {
        genome
            .iter()
            .zip(self.best_actions.iter())
            .map(|(&gene, &best)| {
                let expression =
                    ((gene as u16 * ACTIONS_COUNT_U16 + CODES_COUNT - 1) / CODES_COUNT) as u8;
                (ACTIONS_COUNT - best.abs_diff(expression)) as f32
            })
            .sum::<f32>()
            / self.normalization_factor
    }
}

fn get_best_actions(remaining_stick_count: u8) -> Vec<u8> {
    (MIN_STICK_CHOICE + 1..=remaining_stick_count)
        .rev()
        .map(get_best_action)
        .collect()
}

fn get_best_action(remaining_stick_count: u8) -> u8 {
    if remaining_stick_count % MOD_CHOICE == MIN_STICK_CHOICE {
        MIN_STICK_CHOICE
    } else {
        remaining_stick_count - MIN_STICK_CHOICE
    }
}

#[cfg(test)]
mod tests {
    use common_test::get_seeded_rng;
    use genetic::adaptation::Strategy;
    use rand::Rng;

    use super::{get_best_actions, NimStrategy, MIN_STICK_CHOICE, MOD_CHOICE};

    #[test]
    fn test_new() {
        let mut rng = get_seeded_rng().unwrap();

        let result = NimStrategy::new(MIN_STICK_CHOICE);
        assert!(
            matches!(result, Err(_)),
            "Should not support {MIN_STICK_CHOICE} as a valid stick count"
        );

        let stick_count = rng.gen_range(MIN_STICK_CHOICE + 1..128);
        let result = NimStrategy::new(stick_count).unwrap();
        assert_eq!(
            result.best_actions,
            get_best_actions(stick_count),
            "Should initialized best choices"
        );
    }

    #[test]
    fn test_nim_strategy_genome_size() {
        let mut rng = get_seeded_rng().unwrap();
        let stick_count = rng.gen_range(MIN_STICK_CHOICE + 1..128);
        let strategy = NimStrategy::new(stick_count).unwrap();

        let result = strategy.genome_size();
        assert_eq!(
            result,
            (stick_count - MIN_STICK_CHOICE) as usize,
            "Should have a right genome size"
        );
    }

    #[test]
    fn test_evaluate() {
        // Given
        let strategy = NimStrategy::new(4).unwrap();

        // When
        let result = strategy.evaluate(&vec![200, 100, 50]);
        // Then
        assert_eq!(
            1.0, result,
            "Should return maximum fitness when genome leads to get best actions"
        );

        // When
        let result = strategy.evaluate(&vec![50, 50, 50]);
        // Then
        assert_eq!(2.0 / 3.0, result, "Should return right fitness");
    }

    #[test]
    fn test_choose_best_actions() {
        let mut rng = get_seeded_rng().unwrap();
        let stick_count = rng.gen_range(MIN_STICK_CHOICE + 1..128);
        let expected: Vec<_> = (MIN_STICK_CHOICE + 1..=stick_count)
            .rev()
            .map(|r| {
                if r % MOD_CHOICE == MIN_STICK_CHOICE {
                    MIN_STICK_CHOICE
                } else {
                    r - MIN_STICK_CHOICE
                }
            })
            .collect();

        let result = get_best_actions(stick_count);
        assert_eq!(expected, result, "Should return the best actions");
    }
}
