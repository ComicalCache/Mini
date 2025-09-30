use crate::state_machine::{action::Action, command_map::Command};
use std::{rc::Rc, time::Instant};
use termion::event::Key;

#[derive(Clone, Copy)]
pub enum Input {
    Key(Key),
    Timeout,
}

impl From<Option<Key>> for Input {
    fn from(value: Option<Key>) -> Self {
        match value {
            Some(key) => Input::Key(key),
            None => Input::Timeout,
        }
    }
}

#[derive(Clone)]
pub enum ChainResult<A: Action> {
    Action(A),
    Command(Command<A>),
}

#[derive(Clone)]
pub(super) enum State<A: Action> {
    Normal,
    OperatorPending {
        handler: Rc<dyn Fn(Key) -> Option<ChainResult<A>>>,
    },
    WaitingForMoreInput {
        handler: Rc<dyn Fn(Key) -> Option<ChainResult<A>>>,
        started: Instant,
    },
}
