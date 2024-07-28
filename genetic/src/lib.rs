use std::{fmt::Debug, rc::Rc};

use common::subject_observer::{Observer, Subject};
use futures::future::join_all;
use rand::thread_rng;
use selection::{selector::Selector, Selection, SelectionError};

pub mod selection;

#[derive(Debug, Clone)]
pub struct Evaluation<State> {
    pub state: State,
    pub fitness: f32,
}

pub trait Strategy {
    type State: Clone + Debug;
    type Score;

    fn challenge(&self, subject: &Self::State) -> Self::Score;
    fn evaluate(&self, score: &Self::Score) -> f32;
    fn init_states(&self, size: u32) -> Vec<Self::State>;
    fn mutate(&self, state: &mut Self::State);
    fn crossover(&self, state1: &Self::State, state2: &Self::State) -> Self::State;
}

#[derive(Debug, Clone, PartialEq)]
pub enum EventType {
    NewGeneration,
    Evaluation,
}

#[derive(Debug, Clone)]
pub struct PopulationInfo<State> {
    pub generation: u64,
    pub evaluations: Vec<Evaluation<State>>,
}

//#[derive(Debug, Clone)]
//pub struct PopulationEvent<State> {
//    pub event_type: EventType,
//    pub population_info: PopulationInfo<State>,
//}

// impl<State> PopulationEvent<State> {
//     fn new_generation(population_info: PopulationInfo<State>) -> Self {
//         PopulationEvent {
//             event_type: EventType::NewGeneration,
//             population_info,
//         }
//     }

//     fn evaluation(population_info: PopulationInfo<State>) -> Self {
//         PopulationEvent {
//             event_type: EventType::Evaluation,
//             population_info,
//         }
//     }
// }

type Observers<Subject> = Vec<Rc<dyn Observer<Subject, EventType>>>;

pub struct PopulationRunner<State> {
    observers: Observers<Self>,
    selection: Selection,
    population_info: PopulationInfo<State>,
}

impl<State: Clone> Subject<EventType> for PopulationRunner<State> {
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

pub type RunResult<State> = Result<PopulationInfo<State>, SelectionError>;

impl<State: Clone> PopulationRunner<State> {
    pub fn new(selection: Selection) -> Self {
        PopulationRunner {
            observers: vec![],
            selection,
            population_info: PopulationInfo {
                generation: 0,
                evaluations: vec![],
            },
        }
    }

    pub fn get_population_info(&self) -> PopulationInfo<State> {
        self.population_info.clone()
    }

    pub async fn run<T: Strategy<State = State>, F: Fn(u64, &[f32]) -> bool>(
        &mut self,
        strategy: &T,
        size: u32,
        is_complete: F,
    ) -> RunResult<State> {
        self.population_info.evaluations = to_evaluations(strategy.init_states(size));
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

            if is_complete(self.population_info.generation, &fitnesses) {
                return Ok(self.population_info.clone());
            }

            let new_generation = selector
                .select_couples(&self.population_info.evaluations, &mut thread_rng())?
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
