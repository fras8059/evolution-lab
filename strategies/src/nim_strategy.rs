use anyhow::{bail, Result};
use genetic::adaptation::Strategy;

const MAX_STICK_CHOICE: u8 = 3;
const MIN_STICK_CHOICE: u8 = 1;

const MOD_CHOICE: u8 = MAX_STICK_CHOICE + MIN_STICK_CHOICE;

pub struct NimStrategy {
    best_choices: Vec<u8>,
}

impl NimStrategy {
    pub fn new(stick_count: u8) -> Result<Self> {
        if stick_count <= MIN_STICK_CHOICE {
            bail!("Invalid initial stick count: {stick_count}; It must be greater than {MIN_STICK_CHOICE}");
        }
        Ok(NimStrategy {
            best_choices: do_best_choices(stick_count),
        })
    }
}

impl Strategy for NimStrategy {
    fn genome_size(&self) -> usize {
        self.best_choices.len()
    }

    fn evaluate(&self, genome: &genetic::Genome) -> f32 {
        genome
            .iter()
            .zip(self.best_choices.iter())
            .map(|(&a, &b)| ((b as i8 - a as i8).abs()) as f32)
            .sum()
    }
}

fn do_best_choices(remaining_stick_count: u8) -> Vec<u8> {
    (MIN_STICK_CHOICE + 1..=remaining_stick_count)
        .rev()
        .map(do_best_choice)
        .collect()
}

fn do_best_choice(remaining_stick_count: u8) -> u8 {
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

    use super::{do_best_choices, NimStrategy, MIN_STICK_CHOICE, MOD_CHOICE};

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
            result.best_choices,
            do_best_choices(stick_count),
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
    fn test_do_best_choices() {
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

        let result = do_best_choices(stick_count);
        assert_eq!(expected, result, "Should return the best choices");
    }
}
