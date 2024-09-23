use std::rc::Rc;

use common::subject_observer::{Observer, SharedObservers, Subject};
use futures::future::join_all;
use rand::Rng;
use validator::Validate;

use crate::{
    selection::{select, select_couples},
    Evaluation, Strategy,
};

use super::{
    genetic_pool::GeneticPool, EventType, EvolutionConfig, EvolutionError, EvolutionResult,
    GenerationRenewalConfig, Snapshot,
};

pub struct EvolutionEngine<State> {
    observers: SharedObservers<Self, EventType>,
    population_info: Snapshot<State>,
}

impl<State> Default for EvolutionEngine<State> {
    fn default() -> Self {
        Self {
            observers: Default::default(),
            population_info: Default::default(),
        }
    }
}

impl<State> Subject<EventType> for EvolutionEngine<State>
where
    State: Clone,
{
    fn register_observer(&mut self, observer: Rc<dyn Observer<Self, EventType>>) {
        self.observers.push(observer);
    }

    fn unregister_observer(&mut self, observer: Rc<dyn Observer<Self, EventType>>) {
        self.observers.retain(|obs| !Rc::ptr_eq(obs, &observer));
    }

    fn notify_observers(&self, event: EventType) {
        for obs in &self.observers {
            obs.update(self, event.clone());
        }
    }
}

impl<G> EvolutionEngine<G>
where
    G: Clone,
{
    pub fn get_population_info(&self) -> Snapshot<G> {
        self.population_info.clone()
    }

    pub async fn run<T, F>(
        &mut self,
        strategy: &T,
        config: &EvolutionConfig,
        is_complete: F,
        rng: &mut impl Rng,
    ) -> EvolutionResult<G>
    where
        T: Strategy<G = G>,
        F: Fn(u64, &[f32]) -> bool,
    {
        config.validate().map_err(EvolutionError::InvalidSettings)?;

        let generation_renewal_config = config.generation_renewal_config.as_ref();
        let settings = resolve_settings(generation_renewal_config, config.population_size);

        let states = (0..config.population_size)
            .map(|_| strategy.generate_genome())
            .collect::<Vec<_>>();
        self.population_info.evaluations = to_evaluations(states);
        loop {
            self.notify_observers(EventType::NewGeneration);
            let challenge_runs = self
                .population_info
                .evaluations
                .iter()
                .map(|evaluation| run_challenge(&evaluation.genome, strategy));

            let fitnesses = join_all(challenge_runs).await;

            fitnesses
                .iter()
                .enumerate()
                .for_each(|(i, &f)| self.population_info.evaluations[i].fitness = f);
            self.notify_observers(EventType::Evaluation);

            if (is_complete)(self.population_info.generation, &fitnesses) {
                return Ok(self.population_info.clone());
            }

            let new_generation = self.get_new_generation(strategy, &settings, rng)?;

            self.population_info.evaluations = to_evaluations(new_generation);
            self.population_info.generation += 1;
        }
    }

    fn get_clones<T>(
        &self,
        strategy: &T,
        pool: &GeneticPool,
        rng: &mut impl Rng,
    ) -> Result<Vec<G>, EvolutionError>
    where
        T: Strategy<G = G>,
    {
        let clones = if pool.count > 0 {
            let selected_indexes_iter = select(
                &self.population_info.evaluations,
                pool.count,
                pool.selection_type,
                rng,
            )
            .map_err(|e| EvolutionError::InvalidSelection(e.to_string()))?
            .into_iter();

            if pool.mutation_rate > 0.0 {
                selected_indexes_iter
                    .map(|index| {
                        let mut genome = self.population_info.evaluations[index].genome.clone();
                        strategy.mutate(&mut genome, pool.mutation_rate);
                        genome
                    })
                    .collect()
            } else {
                selected_indexes_iter
                    .map(|index| self.population_info.evaluations[index].genome.clone())
                    .collect()
            }
        } else {
            vec![]
        };
        Ok(clones)
    }

    fn get_offsprings<T>(
        &self,
        strategy: &T,
        pool: &GeneticPool,
        rng: &mut impl Rng,
    ) -> Result<Vec<G>, EvolutionError>
    where
        T: Strategy<G = G>,
    {
        let offsprings = if pool.count > 0 {
            let selected_indexes_iter = select_couples(
                &self.population_info.evaluations,
                pool.count,
                pool.selection_type,
                rng,
            )
            .map_err(|e| EvolutionError::InvalidSelection(e.to_string()))?
            .into_iter();

            if pool.mutation_rate > 0.0 {
                selected_indexes_iter
                    .map(|(p1, p2)| {
                        let mut offspring = strategy.crossover((
                            &self.population_info.evaluations[p1].genome,
                            &self.population_info.evaluations[p2].genome,
                        ));
                        strategy.mutate(&mut offspring, pool.mutation_rate);

                        offspring
                    })
                    .collect()
            } else {
                selected_indexes_iter
                    .map(|(p1, p2)| {
                        strategy.crossover((
                            &self.population_info.evaluations[p1].genome,
                            &self.population_info.evaluations[p2].genome,
                        ))
                    })
                    .collect()
            }
        } else {
            vec![]
        };
        Ok(offsprings)
    }

    fn get_new_generation<T>(
        &self,
        strategy: &T,
        settings: &ExecutionSettings,
        rng: &mut impl Rng,
    ) -> Result<Vec<G>, EvolutionError>
    where
        T: Strategy<G = G>,
    {
        // Get clones
        let clones = self.get_clones(strategy, &settings.cloning_pool, rng)?;

        // Get offsprings
        let offsprings = self.get_offsprings(strategy, &settings.crossover_pool, rng)?;

        // Get random genomes
        let randoms = if settings.randoms_count > 0 {
            get_random_genomes(settings.randoms_count, strategy)
        } else {
            vec![]
        };

        Ok([clones.as_slice(), offsprings.as_slice(), randoms.as_slice()].concat())
    }
}

struct ExecutionSettings {
    cloning_pool: GeneticPool,
    crossover_pool: GeneticPool,
    randoms_count: usize,
}

fn resolve_settings(
    generation_renewal_config: Option<&GenerationRenewalConfig>,
    population_size: usize,
) -> ExecutionSettings {
    let cloning_pool = GeneticPool::from_optional_params(
        generation_renewal_config.and_then(|c| c.cloning.as_ref()),
        population_size,
    );
    let crossover_pool = GeneticPool::from_optional_params(
        generation_renewal_config.and_then(|c| c.crossover.as_ref()),
        population_size,
    );

    ExecutionSettings {
        cloning_pool,
        crossover_pool,
        randoms_count: population_size - (cloning_pool.count + crossover_pool.count),
    }
}

fn to_evaluations<G>(genomes: Vec<G>) -> Vec<Evaluation<G>> {
    genomes
        .into_iter()
        .map(|genome| Evaluation {
            genome,
            fitness: 0f32,
        })
        .collect()
}

async fn run_challenge<T: Strategy>(state: &T::G, strategy: &T) -> f32 {
    strategy.evaluate(state)
}

fn get_random_genomes<T, G>(count: usize, strategy: &T) -> Vec<G>
where
    T: Strategy<G = G>,
{
    (0..count)
        .map(|_| strategy.generate_genome())
        .collect::<Vec<_>>()
}

#[cfg(test)]
mod tests {
    use crate::{
        evolution::{
            evolution_engine::to_evaluations, genetic_pool::GeneticPool, EvolutionConfig,
            EvolutionError, GenerationRenewalConfig, GeneticRenewalParam, Snapshot,
        },
        selection::SelectionType,
        Evaluation, Strategy,
    };
    use futures::executor::block_on;
    use mockall::{mock, predicate::eq};
    use rand::thread_rng;

    use super::{get_random_genomes, resolve_settings, run_challenge, EvolutionEngine};

    mock! {
        TestStrategy {}

        impl Strategy for TestStrategy {
            type G = Vec<u8>;

            fn crossover<'a>(&self, genomes: (&'a <Self as Strategy>::G, &'a <Self as Strategy>::G)) -> <Self as Strategy>::G;

            fn evaluate<'a>(&self, genome: &'a <Self as Strategy>::G) -> f32;

            fn generate_genome(&self) -> <Self as Strategy>::G;

            fn mutate(&self, genome: &mut <Self as Strategy>::G, mutation_rate: f32);
        }

    }

    #[test]
    fn test_evolution_engine_run() {
        //Given
        let strategy = MockTestStrategy::new();
        let config = EvolutionConfig {
            generation_renewal_config: Some(GenerationRenewalConfig {
                cloning: Some(GeneticRenewalParam {
                    mutation_rate: None,
                    ratio: 2.0,
                    selection_type: SelectionType::Chance,
                }),
                crossover: None,
            }),
            population_size: 64,
        };
        let mut engine: EvolutionEngine<Vec<u8>> = EvolutionEngine::default();

        // When
        let result = block_on(engine.run(
            &strategy,
            &config,
            |generation, _| generation > 10,
            &mut thread_rng(),
        ));

        // Then
        assert!(
            matches!(result, Err(EvolutionError::InvalidSettings(_))),
            "Should validate configuration"
        );
    }

    #[test]
    fn test_evolution_engine_get_clones() {
        // TODO use seedable
        let mut rng = thread_rng();
        let mut engine = EvolutionEngine::<Vec<u8>>::default();
        engine.population_info = Snapshot {
            evaluations: vec![
                Evaluation {
                    fitness: 0.5,
                    genome: vec![3],
                },
                Evaluation {
                    fitness: 0.2,
                    genome: vec![5, 1],
                },
                Evaluation {
                    fitness: 0.8,
                    genome: vec![6, 3],
                },
            ],
            generation: 0,
        };

        // Given
        let pool = GeneticPool {
            count: 2,
            mutation_rate: 0.0,
            selection_type: SelectionType::Chance,
        };
        let mut strategy = MockTestStrategy::new();
        strategy.expect_mutate().times(0).return_const(());

        // When
        let result = engine.get_clones(&strategy, &pool, &mut rng).unwrap();

        // Then
        assert_eq!(
            pool.count,
            result.len(),
            "Should return the count of clone defined by the pool when mutation rate is 0"
        );

        // Given
        let pool = GeneticPool {
            count: 2,
            mutation_rate: 0.5,
            selection_type: SelectionType::Chance,
        };
        let mut strategy = MockTestStrategy::new();
        strategy.expect_mutate().times(2).return_const(());

        // When
        let result = engine.get_clones(&strategy, &pool, &mut rng).unwrap();

        // Then
        assert_eq!(
            pool.count,
            result.len(),
            "Should return the count of clone defined by the pool when mutation rate is greater than 0"
        );
    }

    #[test]
    fn test_evolution_engine_get_offsprings() {
        // TODO use seedable
        let mut rng = thread_rng();
        let mut engine = EvolutionEngine::<Vec<u8>>::default();
        engine.population_info = Snapshot {
            evaluations: vec![
                Evaluation {
                    fitness: 0.5,
                    genome: vec![3, 3],
                },
                Evaluation {
                    fitness: 0.2,
                    genome: vec![5, 1],
                },
                Evaluation {
                    fitness: 0.8,
                    genome: vec![6, 3],
                },
            ],
            generation: 0,
        };

        // Given
        let pool = GeneticPool {
            count: 2,
            mutation_rate: 0.0,
            selection_type: SelectionType::Chance,
        };
        let mut strategy = MockTestStrategy::new();
        strategy.expect_mutate().times(0).return_const(());
        strategy
            .expect_crossover()
            .times(pool.count)
            .return_const(vec![]);

        // When
        let result = engine.get_offsprings(&strategy, &pool, &mut rng).unwrap();

        // Then
        assert_eq!(
            pool.count,
            result.len(),
            "Should return the count of clone defined by the pool when mutation rate is 0"
        );

        // Given
        let pool = GeneticPool {
            count: 2,
            mutation_rate: 0.5,
            selection_type: SelectionType::Chance,
        };
        let mut strategy = MockTestStrategy::new();
        strategy.expect_mutate().times(pool.count).return_const(());
        strategy
            .expect_crossover()
            .times(pool.count)
            .return_const(vec![]);

        // When
        let result = engine.get_offsprings(&strategy, &pool, &mut rng).unwrap();

        // Then
        assert_eq!(
            pool.count,
            result.len(),
            "Should return the count of clone defined by the pool when mutation rate is greater than 0"
        );
    }

    #[test]
    fn test_evolution_info_population_info_should_be_defaulted_before_run() {
        // Given
        let engine: EvolutionEngine<Vec<u8>> = EvolutionEngine::default();

        // When
        let result = engine.get_population_info();

        // Then
        assert_eq!(Snapshot::<Vec<u8>>::default(), result);
    }

    #[test]
    fn test_resolve_settings() {
        // Given
        let config = GenerationRenewalConfig {
            cloning: Some(GeneticRenewalParam {
                mutation_rate: None,
                ratio: 0.5,
                selection_type: SelectionType::Chance,
            }),
            crossover: None,
        };

        // When
        let result = resolve_settings(Some(&config), 64);

        // Then
        assert_eq!(32, result.randoms_count)
    }

    #[test]
    fn test_to_evaluations() {
        // Given
        let states = vec![1, 2, 3];
        // When
        let result = to_evaluations(states);

        // Then
        assert_eq!(
            vec![
                Evaluation {
                    fitness: 0.0,
                    genome: 1
                },
                Evaluation {
                    fitness: 0.0,
                    genome: 2
                },
                Evaluation {
                    fitness: 0.0,
                    genome: 3
                }
            ],
            result
        );
    }

    #[test]
    fn test_run_challenges() {
        // Given
        let state: Vec<u8> = vec![1, 2];
        let mut strategy = MockTestStrategy::new();
        let fitness = 0.5f32;
        strategy
            .expect_evaluate()
            .with(eq(state.clone()))
            .return_const(fitness);

        // When
        let result = block_on(run_challenge(&state, &strategy));

        // Then
        assert_eq!(fitness, result, "Should call strategy evaluation");
    }

    #[test]
    fn test_get_random_genomes() {
        // Given
        let count = 3;
        let mut strategy = MockTestStrategy::new();
        strategy
            .expect_generate_genome()
            .times(count)
            .returning(|| vec![]);

        // When
        let result = get_random_genomes(count, &strategy);

        // Then
        assert_eq!(count, result.len(), "Should relay on strategy");
    }
}
