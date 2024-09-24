use rand::{
    distributions::{uniform::SampleRange, Distribution, WeightedIndex},
    Rng,
};

pub trait RngWrapper {
    fn gen_range<R>(&mut self, range: R) -> usize
    where
        R: SampleRange<usize>;

    fn sample_from_distribution(&mut self, distribution: &WeightedIndex<f64>) -> usize;
}

pub struct Random<'a, T>
where
    T: Rng,
{
    rng: &'a mut T,
}

impl<'a, T> Random<'a, T>
where
    T: Rng,
{
    pub fn new(rng: &'a mut T) -> Self {
        Random { rng }
    }
}

impl<'a, T> RngWrapper for Random<'a, T>
where
    T: Rng,
{
    fn gen_range<R>(&mut self, range: R) -> usize
    where
        R: SampleRange<usize>,
    {
        self.rng.gen_range(range)
    }

    fn sample_from_distribution(&mut self, distribution: &WeightedIndex<f64>) -> usize {
        distribution.sample(self.rng)
    }
}

#[cfg(test)]
mod tests {
    use std::ptr;

    use common_test::get_seeded_rng;
    use rand::{distributions::WeightedIndex, Rng};

    use super::{Random, RngWrapper};

    #[test]
    fn test_random_new_should_init_with_arg() {
        // Given
        let mut rng = get_seeded_rng().unwrap();
        let rng_ptr = &mut rng as *mut _;

        // When
        let result = Random::new(&mut rng);

        // Then
        assert!(ptr::eq(rng_ptr, &*result.rng));
    }

    #[test]
    fn test_random_gen_range_should_respect_range() {
        // Given
        let mut rng = get_seeded_rng().unwrap();
        let low_b = rng.gen_range(0usize..10);
        let high_b = rng.gen_range(10..20);
        let mut random = Random::new(&mut rng);

        // When
        let result = random.gen_range(low_b..high_b);

        assert!(result >= low_b && result < high_b);
    }

    #[test]
    fn test_random_sample_from_distribution() {
        // Given
        let mut rng = get_seeded_rng().unwrap();
        let mut random = Random::new(&mut rng);
        let weights = vec![1.0, 2.0, 3.0];
        let distribution = WeightedIndex::new(weights).unwrap();

        // When
        let result = random.sample_from_distribution(&distribution);

        // Then
        assert!(result < 3);
    }
}

#[cfg(test)]
pub mod test_utils {
    use super::RngWrapper;

    pub struct RngTest {
        samples: Vec<usize>,
        index: usize,
    }

    impl RngTest {
        pub fn new() -> Self {
            RngTest {
                samples: vec![],
                index: 0,
            }
        }

        pub fn with_samples(samples: Vec<usize>) -> Self {
            RngTest { samples, index: 0 }
        }

        pub fn next(&mut self) -> usize {
            if self.index > self.samples.len() {
                panic!("Unable choose next item whereas no sample is defined")
            }
            let result = self.samples[self.index];
            self.index = (self.index + 1) % self.samples.len();
            result
        }
    }

    impl RngWrapper for RngTest {
        fn gen_range<R>(&mut self, _: R) -> usize
        where
            R: rand::distributions::uniform::SampleRange<usize>,
        {
            self.next()
        }

        fn sample_from_distribution(
            &mut self,
            _: &rand::distributions::WeightedIndex<f64>,
        ) -> usize {
            self.next()
        }
    }
}
