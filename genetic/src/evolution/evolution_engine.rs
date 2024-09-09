use std::rc::Rc;

use common::subject_observer::{Observer, SharedObservers, Subject};
use futures::future::join_all;
use rand::Rng;

use crate::{individual::Strategy, selection::selector::Selector, Evaluation};

use super::{EventType, EvolutionResult, EvolutionSettings, Snapshot};

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

impl<State> EvolutionEngine<State>
where
    State: Clone,
{
    pub fn get_population_info(&self) -> Snapshot<State> {
        self.population_info.clone()
    }

    pub async fn run<T, F>(
        &mut self,
        strategy: &T,
        settings: &EvolutionSettings,
        is_complete: F,
        rng: &mut impl Rng,
    ) -> EvolutionResult<State>
    where
        T: Strategy<State = State>,
        F: Fn(u64, &[f32]) -> bool,
    {
        let states = (0..settings.population_size)
            .map(|_| strategy.get_random_state())
            .collect::<Vec<_>>();
        self.population_info.evaluations = to_evaluations(states);
        let selector = Selector::new(settings.selection_type);
        loop {
            self.notify_observers(EventType::NewGeneration);
            let challenge_runs = self
                .population_info
                .evaluations
                .iter()
                .map(|evaluation| run_challenge(&evaluation.state, strategy));

            let fitnesses = join_all(challenge_runs).await;

            fitnesses
                .iter()
                .enumerate()
                .for_each(|(i, &f)| self.population_info.evaluations[i].fitness = f);
            self.notify_observers(EventType::Evaluation);

            if (is_complete)(self.population_info.generation, &fitnesses) {
                return Ok(self.population_info.clone());
            }

            let new_generation = selector
                .select_couples(&self.population_info.evaluations, rng)?
                .into_iter()
                .map(|(p1, p2)| {
                    let mut child = strategy.crossover(
                        &self.population_info.evaluations[p1].state,
                        &self.population_info.evaluations[p2].state,
                    );
                    strategy.mutate(&mut child, settings.mutation_rate);
                    child
                })
                .collect::<Vec<_>>();
            self.population_info.evaluations = to_evaluations(new_generation);
            self.population_info.generation += 1;
        }
    }
}

fn to_evaluations<State>(states: Vec<State>) -> Vec<Evaluation<State>> {
    states
        .into_iter()
        .map(|state| Evaluation {
            state,
            fitness: 0f32,
        })
        .collect()
}

async fn run_challenge<T: Strategy>(state: &T::State, strategy: &T) -> f32 {
    let score = strategy.challenge(state);
    strategy.evaluate(&score)
}
