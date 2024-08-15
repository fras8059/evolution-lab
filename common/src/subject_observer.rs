use std::rc::Rc;

pub trait Observer<S: Subject<E>, E: Clone> {
    fn update(&self, source: &S, event: E);
}

pub trait Subject<E: Clone> {
    fn register_observer(&mut self, observer: Rc<dyn Observer<Self, E>>);
    fn unregister_observer(&mut self, observer: Rc<dyn Observer<Self, E>>);
    fn notify_observers(&self, event: E);
}

pub type Observers<S, E> = Vec<dyn Observer<S, E>>;
pub type SharedObservers<S, E> = Vec<Rc<dyn Observer<S, E>>>;
