use rand::{
    distributions::{uniform::SampleRange, Distribution, WeightedIndex},
    Rng,
};

pub trait RngWrapper {
    fn gen_range<R>(&mut self, range: R) -> usize
    where
        R: SampleRange<usize>;

    fn sample_from_distribution(&mut self, distribution: &WeightedIndex<f32>) -> usize;
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

    fn sample_from_distribution(&mut self, distribution: &WeightedIndex<f32>) -> usize {
        distribution.sample(self.rng)
    }
}
