use crate::state_machine::{action::Action, states::ChainResult};
use std::{collections::HashMap, rc::Rc};
use termion::event::Key;

pub trait HandlerFn<A: Action> = Fn(Key) -> Option<ChainResult<A>>;
pub type Handler<A> = Rc<dyn HandlerFn<A>>;

#[derive(Clone)]
pub enum Command<A: Action> {
    Simple(A),
    Operator(Handler<A>),
    Prefix(Handler<A>),
}

pub struct CommandMap<A: Action> {
    pub(super) bindings: HashMap<Key, Command<A>>,
}

impl<A: Action> CommandMap<A> {
    pub fn new() -> Self {
        Self {
            bindings: HashMap::new(),
        }
    }

    /// Adds a simple one key command.
    pub fn simple(mut self, key: Key, action: A) -> Self {
        self.bindings.insert(key, Command::Simple(action));
        self
    }

    /// Adds an operator command, a command followed by more keys without timeout.
    pub fn operator<F: HandlerFn<A> + 'static>(mut self, key: Key, handler: F) -> Self {
        self.bindings
            .insert(key, Command::Operator(Rc::new(handler)));
        self
    }

    /// Adds a prefix command, a command followed by more keys with timeout.
    pub fn prefix<F: HandlerFn<A> + 'static>(mut self, key: Key, handler: F) -> Self {
        self.bindings.insert(key, Command::Prefix(Rc::new(handler)));
        self
    }
}
