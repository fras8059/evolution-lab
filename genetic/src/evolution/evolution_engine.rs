use std::{
    rc::Rc,
    sync::{Arc, Mutex},
};

use common::subject_observer::{Observer, SharedObservers, Subject};
use futures::future::join_all;
use log::{debug, trace};
use rand::{distributions::Standard, Rng};
use validator::Validate;

use crate::{
    adaptation::Strategy,
    selection::{select, select_couples},
    Genome, IntoEvaluations,
};

use super::{
    genetic_pool::GeneticPool, EventType, EvolutionConfig, EvolutionError, EvolutionResult,
    EvolutionStatus, GenerationRenewalConfig, Snapshot,
};

#[derive(Debug, Clone, Copy)]
struct ExecutionSettings {
    cloning_pool: GeneticPool,
    crossover_pool: GeneticPool,
    randoms_count: usize,
}

#[derive(Default)]
pub struct EvolutionEngine {
    observers: SharedObservers<Self, EventType>,
    snapshot: Snapshot,
    status: Arc<Mutex<EvolutionStatus>>,
}

impl Subject<EventType> for EvolutionEngine {
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

impl EvolutionEngine {
    pub fn snapshot(&self) -> Snapshot {
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
    ) -> EvolutionResult
    where
        T: Strategy,
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
        snapshot: Snapshot,
    ) -> EvolutionResult
    where
        T: Strategy,
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
        snapshot: Option<Snapshot>,
    ) -> EvolutionResult
    where
        T: Strategy,
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

        let genome_size = strategy.genome_size();

        self.snapshot = snapshot.unwrap_or_else(|| {
            let evaluations = get_random_genomes_iter(config.population_size, genome_size, rng)
                .into_evaluations()
                .collect();
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

            self.snapshot.evaluations = self
                .get_new_generation(genome_size, &settings, rng)?
                .into_iter()
                .into_evaluations()
                .collect();
            self.snapshot.generation += 1;
        }
        Ok(self.snapshot.clone())
    }

    fn get_clones(
        &self,
        pool: &GeneticPool,
        rng: &mut impl Rng,
    ) -> Result<Vec<Genome>, EvolutionError> {
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
                        mutate(&mut genome, pool.mutation_rate, rng);
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

    fn get_offsprings(
        &self,
        genome_size: usize,
        pool: &GeneticPool,
        rng: &mut impl Rng,
    ) -> Result<Vec<Genome>, EvolutionError> {
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
                        let mut offspring = crossover(
                            (
                                &self.snapshot.evaluations[p1].genome,
                                &self.snapshot.evaluations[p2].genome,
                            ),
                            genome_size,
                            rng,
                        );
                        mutate(&mut offspring, pool.mutation_rate, rng);

                        offspring
                    })
                    .collect()
            } else {
                selected_indexes_iter
                    .map(|(p1, p2)| {
                        crossover(
                            (
                                &self.snapshot.evaluations[p1].genome,
                                &self.snapshot.evaluations[p2].genome,
                            ),
                            genome_size,
                            rng,
                        )
                    })
                    .collect()
            }
        } else {
            vec![]
        };
        Ok(offsprings)
    }

    fn get_new_generation(
        &self,
        genome_size: usize,
        settings: &ExecutionSettings,
        rng: &mut impl Rng,
    ) -> Result<Vec<Genome>, EvolutionError> {
        // Get clones
        let clones = self.get_clones(&settings.cloning_pool, rng)?;

        // Get offsprings
        let offsprings = self.get_offsprings(genome_size, &settings.crossover_pool, rng)?;

        // Get random genomes
        let randoms = if settings.randoms_count > 0 {
            get_random_genomes_iter(settings.randoms_count, genome_size, rng).collect()
        } else {
            vec![]
        };

        Ok([clones.as_slice(), offsprings.as_slice(), randoms.as_slice()].concat())
    }
}

fn crossover(parents: (&Genome, &Genome), genome_size: usize, rng: &mut impl Rng) -> Genome {
    let crossover_point = rng.gen_range(0..genome_size);
    [&parents.0[..crossover_point], &parents.1[crossover_point..]].concat()
}

fn get_random_genomes_iter(
    count: usize,
    genome_size: usize,
    rng: &mut impl Rng,
) -> impl Iterator<Item = Genome> + '_ {
    (0..count).map(move |_| {
        (&mut *rng)
            .sample_iter(Standard)
            .take(genome_size)
            .collect()
    })
}

fn mutate(genome: &mut Genome, mutation_rate: f32, rng: &mut impl Rng) {
    for part in genome.iter_mut() {
        if rng.gen::<f32>() < mutation_rate {
            *part = rng.gen();
        }
    }
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

async fn run_challenge<T: Strategy>(genome: &Genome, strategy: &T) -> f32 {
    strategy.evaluate(genome)
}

#[cfg(test)]
mod tests {
    use std::{
        rc::Rc,
        sync::{Arc, Mutex},
    };

    use crate::{
        adaptation::Strategy,
        evolution::{
            evolution_engine::get_random_genomes_iter, genetic_pool::GeneticPool, EventType,
            EvolutionConfig, EvolutionError, EvolutionStatus, GenerationRenewalConfig,
            GeneticRenewalParam, Snapshot,
        },
        selection::SelectionType,
        Evaluation, Genome,
    };
    use common::subject_observer::{Observer, Subject};
    use common_test::get_seeded_rng;
    use futures::executor::block_on;
    use mockall::{
        mock,
        predicate::{always, eq},
    };
    use rand::{distributions::Standard, seq::IteratorRandom, Rng};
    use strum::IntoEnumIterator;

    use super::{resolve_settings, run_challenge, EvolutionEngine};

    mock! {
        TestStrategy {}

        impl Strategy for TestStrategy {

            fn genome_size(&self) -> usize;

            fn evaluate<'a>(&self, genome: &'a Genome) -> f32;

        }

    }

    mock! {
        TestObserver {}

        impl Observer<EvolutionEngine, EventType> for TestObserver {
            fn update(&self, source: &EvolutionEngine, event: EventType);
        }

    }

    #[test]
    fn test_evolution_engine_halt() -> Result<(), EvolutionError> {
        // Given
        let mut rng = get_seeded_rng().unwrap();
        let mut engine = EvolutionEngine::default();
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
        let mut engine = EvolutionEngine::default();
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
        let genome_size = rng.gen_range(1usize..10);
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
        let mut engine = EvolutionEngine::default();

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
            .expect_evaluate()
            .times(2 * population_size)
            .return_const(0.5);
        strategy.expect_genome_size().return_const(genome_size);
        let mut engine = EvolutionEngine::default();
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
        let mut engine = EvolutionEngine::default();
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

        // When
        let result = engine.get_clones(&pool, &mut rng).unwrap();

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

        // When
        let result = engine.get_clones(&pool, &mut rng).unwrap();

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
        let genome_size = rng.gen_range(1usize..10);
        let mut engine = EvolutionEngine::default();
        engine.snapshot = Snapshot {
            evaluations: vec![
                Evaluation {
                    fitness: 0.5,
                    genome: rng
                        .clone()
                        .sample_iter(Standard)
                        .take(genome_size)
                        .collect(),
                },
                Evaluation {
                    fitness: 0.2,
                    genome: rng
                        .clone()
                        .sample_iter(Standard)
                        .take(genome_size)
                        .collect(),
                },
                Evaluation {
                    fitness: 0.8,
                    genome: rng
                        .clone()
                        .sample_iter(Standard)
                        .take(genome_size)
                        .collect(),
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

        // When
        let result = engine.get_offsprings(genome_size, &pool, &mut rng).unwrap();

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

        // When
        let result = engine.get_offsprings(genome_size, &pool, &mut rng).unwrap();

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
        let engine = EvolutionEngine::default();

        // When
        let result = engine.snapshot();

        // Then
        assert_eq!(Snapshot::default(), result);
    }

    #[test]
    fn test_get_random_genomes_iter() {
        // Given
        let mut rng = get_seeded_rng().unwrap();
        let count = rng.gen_range(0..64);
        let genome_size = rng.gen_range(0..10);

        // When
        let result: Vec<Genome> = get_random_genomes_iter(count, genome_size, &mut rng).collect();

        // Then
        assert_eq!(
            count,
            result.len(),
            "Should return the required count of genomes"
        );
        assert!(
            result.iter().all(|g| g.len() == genome_size),
            "Should generate genomes with the required length"
        );
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
    fn test_run_challenges() {
        // Given
        let genome = vec![1, 2];
        let mut strategy = MockTestStrategy::new();
        let fitness = 0.5f32;
        strategy
            .expect_evaluate()
            .with(eq(genome.clone()))
            .return_const(fitness);

        // When
        let result = block_on(run_challenge(&genome, &strategy));

        // Then
        assert_eq!(fitness, result, "Should call strategy evaluation");
    }

    #[test]
    fn test_generate_genomes() {
        // Given
        let mut rng = get_seeded_rng().unwrap();
        let count = rng.gen_range(0..10);
        let len = rng.gen_range(0..10);

        // When
        let result: Vec<Genome> = get_random_genomes_iter(count, len, &mut rng).collect();

        // Then
        assert_eq!(count, result.len(), "Should generate the requested count");
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
