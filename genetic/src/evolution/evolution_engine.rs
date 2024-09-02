use std::rc::Rc;

use common::subject_observer::{Observer, SharedObservers, Subject};
use futures::future::join_all;
use rand::Rng;

use crate::{
    individual::Strategy,
    selection::{selector::Selector, SelectionType},
    Evaluation,
};

use super::{EventType, EvolutionResult, Snapshot};

pub struct EvolutionEngine<State, F>
where
    F: Fn(u64, &[f32]) -> bool,
{
    observers: SharedObservers<Self, EventType>,
    selection: SelectionType,
    population_info: Snapshot<State>,
    population_size: usize,
    is_complete: F,
}

impl<State, F> Subject<EventType> for EvolutionEngine<State, F>
where
    State: Clone,
    F: Fn(u64, &[f32]) -> bool,
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

impl<State, F> EvolutionEngine<State, F>
where
    State: Clone,
    F: Fn(u64, &[f32]) -> bool,
{
    pub fn new(selection: SelectionType, population_size: usize, is_complete: F) -> Self {
        EvolutionEngine {
            observers: vec![],
            selection,
            population_info: Snapshot {
                generation: 0,
                evaluations: vec![],
            },
            population_size,
            is_complete,
        }
    }

    pub fn get_population_info(&self) -> Snapshot<State> {
        self.population_info.clone()
    }

    pub async fn run<T: Strategy<State = State>>(
        &mut self,
        strategy: &T,
        rng: &mut impl Rng,
    ) -> EvolutionResult<State> {
        self.population_info.evaluations =
            to_evaluations(strategy.init_states(self.population_size));
        let selector = Selector::new(self.selection);
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

            if (self.is_complete)(self.population_info.generation, &fitnesses) {
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
                    strategy.mutate(&mut child);
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
