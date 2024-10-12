use std::{
    rc::Rc,
    sync::{Arc, Mutex},
};

use common::subject_observer::{Observer, SharedObservers, Subject};
use futures::future::join_all;
use log::{debug, trace};
use rand::Rng;
use validator::Validate;

use crate::{
    selection::{select, select_couples},
    Evaluation, Strategy,
};

use super::{
    genetic_pool::GeneticPool, EventType, EvolutionConfig, EvolutionError, EvolutionResult,
    EvolutionStatus, GenerationRenewalConfig, Snapshot,
};

#[derive(Default)]
pub struct EvolutionEngine<G> {
    observers: SharedObservers<Self, EventType>,
    snapshot: Snapshot<G>,
    status: Arc<Mutex<EvolutionStatus>>,
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
    pub fn snapshot(&self) -> Snapshot<G> {
        self.snapshot.clone()
    }

    pub fn halt(&mut self) -> Result<bool, EvolutionError> {
        self.change_status(
            EvolutionStatus::Halting,
            Some(&|status| status == EvolutionStatus::Running),
        )
    }

    pub async fn start<T, F>(
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
        self.run(strategy, config, is_complete, rng, None).await
    }

    pub async fn start_from<T, F>(
        &mut self,
        strategy: &T,
        config: &EvolutionConfig,
        is_complete: F,
        rng: &mut impl Rng,
        snapshot: Snapshot<G>,
    ) -> EvolutionResult<G>
    where
        T: Strategy<G = G>,
        F: Fn(u64, &[f32]) -> bool,
    {
        self.run(strategy, config, is_complete, rng, Some(snapshot))
            .await
    }

    fn change_status<F>(
        &self,
        new_status: EvolutionStatus,
        additional_check: Option<&F>,
    ) -> Result<bool, EvolutionError>
    where
        F: Fn(EvolutionStatus) -> bool,
    {
        let mut current_status = self.status.lock()?;

        let result = (*current_status != new_status
            && additional_check.map_or(true, |check| check(*current_status)))
        .then(|| {
            trace!("Changing status from {} to {}", current_status, new_status);
            *current_status = new_status;
            drop(current_status);
            self.notify_observers(EventType::StatusChanged(new_status));
        })
        .is_some();
        Ok(result)
    }

    async fn run<T, F>(
        &mut self,
        strategy: &T,
        config: &EvolutionConfig,
        is_complete: F,
        rng: &mut impl Rng,
        snapshot: Option<Snapshot<G>>,
    ) -> EvolutionResult<G>
    where
        T: Strategy<G = G>,
        F: Fn(u64, &[f32]) -> bool,
    {
        // Validate configuration
        config.validate()?;

        // Run only from fresh engine
        if !self.change_status(
            EvolutionStatus::Initializing,
            Some(&|s| s == EvolutionStatus::New),
        )? {
            let status = self.status.lock()?.to_owned();
            debug!("Cannot run evolution from {} engine state", status);
            return Err(EvolutionError::InvalidStatus(status));
        }

        let generation_renewal_config = config.generation_renewal_config.as_ref();
        let settings = resolve_settings(generation_renewal_config, config.population_size);
        debug!("Running evolution with settings: {:?}", settings);

        self.snapshot = snapshot.unwrap_or_else(|| {
            let genomes = get_random_genomes(config.population_size, strategy);
            let evaluations = init_evaluations(genomes);
            Snapshot {
                evaluations,
                generation: 0,
            }
        });
        self.change_status::<fn(EvolutionStatus) -> bool>(EvolutionStatus::Running, None)?;
        loop {
            trace!("Running generation {}", self.snapshot.generation);
            // Try to halt the evolution if status Halting is set
            if self.change_status(
                EvolutionStatus::Halted,
                Some(&|s| s == EvolutionStatus::Halting),
            )? {
                debug!("Interruption of evolution by detecting halt request");
                break;
            }

            self.notify_observers(EventType::GenerationCreated);
            let challenge_runs = self
                .snapshot
                .evaluations
                .iter()
                .map(|evaluation| run_challenge(&evaluation.genome, strategy));

            let fitnesses = join_all(challenge_runs).await;

            fitnesses
                .iter()
                .enumerate()
                .for_each(|(i, &f)| self.snapshot.evaluations[i].fitness = f);
            self.notify_observers(EventType::Evaluated);

            if (is_complete)(self.snapshot.generation, &fitnesses) {
                debug!("Completion reached");
                self.change_status::<fn(EvolutionStatus) -> bool>(
                    EvolutionStatus::Completed,
                    None,
                )?;
                break;
            }

            let new_generation = self.get_new_generation(strategy, &settings, rng)?;
            self.snapshot.evaluations = init_evaluations(new_generation);
            self.snapshot.generation += 1;
        }
        Ok(self.snapshot.clone())
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
                &self.snapshot.evaluations,
                pool.count,
                pool.selection_type,
                rng,
            )?
            .into_iter();

            if pool.mutation_rate > 0.0 {
                selected_indexes_iter
                    .map(|index| {
                        let mut genome = self.snapshot.evaluations[index].genome.clone();
                        strategy.mutate(&mut genome, pool.mutation_rate);
                        genome
                    })
                    .collect()
            } else {
                selected_indexes_iter
                    .map(|index| self.snapshot.evaluations[index].genome.clone())
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
                &self.snapshot.evaluations,
                pool.count,
                pool.selection_type,
                rng,
            )?
            .into_iter();

            if pool.mutation_rate > 0.0 {
                selected_indexes_iter
                    .map(|(p1, p2)| {
                        let mut offspring = strategy.crossover((
                            &self.snapshot.evaluations[p1].genome,
                            &self.snapshot.evaluations[p2].genome,
                        ));
                        strategy.mutate(&mut offspring, pool.mutation_rate);

                        offspring
                    })
                    .collect()
            } else {
                selected_indexes_iter
                    .map(|(p1, p2)| {
                        strategy.crossover((
                            &self.snapshot.evaluations[p1].genome,
                            &self.snapshot.evaluations[p2].genome,
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

#[derive(Debug, Clone, Copy)]
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

fn init_evaluations<G>(genomes: Vec<G>) -> Vec<Evaluation<G>> {
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
    use std::{
        rc::Rc,
        sync::{Arc, Mutex},
    };

    use crate::{
        evolution::{
            genetic_pool::GeneticPool, EventType, EvolutionConfig, EvolutionError, EvolutionStatus,
            GenerationRenewalConfig, GeneticRenewalParam, Snapshot,
        },
        selection::SelectionType,
        Evaluation, Strategy,
    };
    use common::subject_observer::{Observer, Subject};
    use common_test::get_seeded_rng;
    use futures::executor::block_on;
    use mockall::{
        mock,
        predicate::{always, eq},
    };
    use rand::{seq::IteratorRandom, Rng};
    use strum::IntoEnumIterator;

    use super::{
        get_random_genomes, init_evaluations, resolve_settings, run_challenge, EvolutionEngine,
    };

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

    mock! {
        TestObserver {}

        impl Observer<EvolutionEngine<Vec<u8>>, EventType> for TestObserver {
            fn update(&self, source: &EvolutionEngine<Vec<u8>>, event: EventType);
        }

    }

    #[test]
    fn test_evolution_engine_halt() -> Result<(), EvolutionError> {
        // Given
        let mut rng = get_seeded_rng().unwrap();
        let mut engine: EvolutionEngine<Vec<u8>> = EvolutionEngine::default();
        let not_running = EvolutionStatus::iter()
            .filter(|s| *s != EvolutionStatus::Running)
            .choose(&mut rng)
            .unwrap();

        // When
        engine.status = Arc::new(Mutex::new(not_running));
        let result = engine.halt()?;
        // Then
        assert!(!result, "Should not halt when not running");

        // When
        engine.status = Arc::new(Mutex::new(EvolutionStatus::Running));
        let result = engine.halt()?;
        // Then
        assert!(result, "Should halt when running");
        assert_eq!(
            EvolutionStatus::Halting,
            engine.status.lock().unwrap().clone(),
            "Should set state to Halting"
        );

        Ok(())
    }

    #[test]
    fn test_evolution_engine_change_status() -> Result<(), EvolutionError> {
        // Given
        let mut rng = get_seeded_rng().unwrap();
        let mut engine: EvolutionEngine<Vec<u8>> = EvolutionEngine::default();
        let statuses = EvolutionStatus::iter().choose_multiple(&mut rng, 3);
        let mut observer = MockTestObserver::new();
        observer.expect_update().times(2).return_const(());
        engine.register_observer(Rc::new(observer));

        // When
        engine.status = Arc::new(Mutex::new(statuses[0]));
        let result = engine.change_status::<fn(EvolutionStatus) -> bool>(statuses[0], None)?;
        // Then
        assert!(!result, "Should not change the status when it's the same");

        // When
        let result = engine.change_status::<fn(EvolutionStatus) -> bool>(statuses[1], None)?;
        // Then
        assert!(result, "Should change the status when it's not the same");

        // When
        let result = engine.change_status(statuses[0], Some(&|s| s != statuses[1]))?;
        // Then
        assert!(
            !result,
            "Should not change the status when additional check fails"
        );

        // When
        let result = engine.change_status(statuses[0], Some(&|s| s == statuses[1]))?;
        // Then
        assert!(
            result,
            "Should change the status when additional check succeeds"
        );

        Ok(())
    }

    #[test]
    fn test_evolution_engine_run() {
        // Given
        let mut rng = get_seeded_rng().unwrap();
        let population_size = rng.gen_range(10..128);
        let mut strategy = MockTestStrategy::new();
        let config = EvolutionConfig {
            generation_renewal_config: Some(GenerationRenewalConfig {
                cloning: Some(GeneticRenewalParam {
                    mutation_rate: None,
                    ratio: 2.0,
                    selection_type: SelectionType::Chance,
                }),
                crossover: None,
            }),
            population_size,
        };
        let mut engine: EvolutionEngine<Vec<u8>> = EvolutionEngine::default();

        // When
        let result = block_on(engine.run(
            &strategy,
            &config,
            |generation, _| generation > 1,
            &mut rng,
            None,
        ));

        // Then
        assert!(
            matches!(result, Err(EvolutionError::InvalidSettings(_))),
            "Should validate configuration"
        );

        // Given
        let config = EvolutionConfig {
            generation_renewal_config: None,
            population_size,
        };
        strategy
            .expect_generate_genome()
            .times(2 * population_size)
            .return_const(vec![1]);
        strategy
            .expect_evaluate()
            .times(2 * population_size)
            .return_const(0.5);
        let mut engine: EvolutionEngine<Vec<u8>> = EvolutionEngine::default();
        let observer = build_observer_mock(&vec![
            EventType::StatusChanged(EvolutionStatus::Initializing),
            EventType::StatusChanged(EvolutionStatus::Running),
            EventType::GenerationCreated,
            EventType::Evaluated,
            EventType::StatusChanged(EvolutionStatus::Completed),
        ]);
        engine.register_observer(Rc::new(observer));

        // When
        let result = block_on(engine.run(
            &strategy,
            &config,
            |generation, _| generation > 0,
            &mut rng,
            None,
        ));

        // Then
        assert!(
            matches!(result, Ok(snapshot) if snapshot.generation == 1 && snapshot.evaluations.len() == population_size),
            "Should have rigth snapshot when completed"
        );

        // When
        let result = block_on(engine.run(
            &strategy,
            &config,
            |generation, _| generation > 0,
            &mut rng,
            None,
        ));

        // Then
        assert!(
            matches!(result, Err(EvolutionError::InvalidStatus(_))),
            "Should not run when status is not valid"
        );
    }

    #[test]
    fn test_evolution_engine_get_clones() {
        let mut rng = get_seeded_rng().unwrap();
        let mut engine = EvolutionEngine::<Vec<u8>>::default();
        engine.snapshot = Snapshot {
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
        let mut rng = get_seeded_rng().unwrap();
        let mut engine = EvolutionEngine::<Vec<u8>>::default();
        engine.snapshot = Snapshot {
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
    fn test_evolution_engine_snapshot_should_be_defaulted_before_run() {
        // Given
        let engine: EvolutionEngine<Vec<u8>> = EvolutionEngine::default();

        // When
        let result = engine.snapshot();

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
    fn test_init_evaluations() {
        // Given
        let mut rng = get_seeded_rng().unwrap();
        let size = rng.gen_range(0..10);
        let genomes = (0..size).map(|_| rng.gen()).collect::<Vec<i32>>();

        // When
        let result = init_evaluations(genomes.clone());

        // Then
        assert_eq!(result.len(), genomes.len());
        let result_states = result.iter().map(|e| e.genome).collect::<Vec<_>>();
        assert_eq!(result_states, genomes);
        assert!(result.iter().map(|e| e.fitness).all(|x| x == 0f32));
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

    fn build_observer_mock(events: &[EventType]) -> MockTestObserver {
        let mut observer = MockTestObserver::new();

        for event in events.iter().cloned() {
            observer
                .expect_update()
                .with(always(), eq(event))
                .return_const(());
        }

        observer
    }
}
