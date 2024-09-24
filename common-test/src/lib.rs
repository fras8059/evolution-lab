use std::{
    collections::HashMap,
    env,
    error::Error,
    sync::{OnceLock, RwLock},
};

use rand::{random, rngs::StdRng, SeedableRng};

pub const DEFAULT_TEST_SEED_ENV: &str = "DEFAULT_TEST_SEED";

static SEEDS: OnceLock<RwLock<HashMap<&'static str, u64>>> = OnceLock::new();

fn get_seeds_lock() -> &'static RwLock<HashMap<&'static str, u64>> {
    SEEDS.get_or_init(|| RwLock::new(HashMap::new()))
}

fn get_seed(key: &'static str) -> Result<u64, Box<dyn Error>> {
    let mut seeds = get_seeds_lock().write()?;
    Ok(seeds
        .entry(key)
        .or_insert_with(|| {
            let seed = env::var(key)
                .ok()
                .and_then(|seed_var| seed_var.parse::<u64>().ok())
                .unwrap_or_else(random);
            println!("Using seed {} for {}", seed, key);
            seed
        })
        .to_owned())
}

fn build_rng(key: Option<&'static str>) -> Result<StdRng, Box<dyn Error>> {
    let seed = get_seed(key.unwrap_or(DEFAULT_TEST_SEED_ENV))?;
    Ok(StdRng::seed_from_u64(seed))
}

pub fn get_seeded_rng() -> Result<StdRng, Box<dyn Error>> {
    build_rng(None)
}

pub fn get_seeded_rng_from_scope(key: &'static str) -> Result<StdRng, Box<dyn Error>> {
    build_rng(Some(key))
}

#[cfg(test)]
mod tests {
    use std::env;

    use crate::{get_seed, get_seeded_rng_from_scope, get_seeds_lock};

    #[test]
    fn test_get_seeded_rng_from_scope() {
        // Given
        let key = "test_get_seeded_rng_from_scope";
        let seed = 1u64;
        env::set_var(key, seed.to_string());

        // When
        get_seeded_rng_from_scope(key).unwrap();

        // Then
        assert!(get_seeds_lock().read().unwrap().contains_key(key));
        assert_eq!(seed, get_seed(key).unwrap())
    }
}
