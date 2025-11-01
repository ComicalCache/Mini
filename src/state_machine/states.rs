use crate::state_machine::{action::Action, command_map::Handler};
use std::{rc::Rc, time::Instant};
use termion::event::Key;

#[derive(Clone, Copy)]
/// An input token of the state machine.
pub enum Input {
    /// A key.
    Key(Key),
    /// A timeout event.
    Timeout,
}

impl From<Option<Key>> for Input {
    fn from(value: Option<Key>) -> Self {
        value.map_or(Self::Timeout, Self::Key)
    }
}

#[derive(Clone)]
/// The result of a chain of valid input events. Used for defining sequences.
pub enum ChainResult<A: Action> {
    /// An action should be taken, the sequence is over.
    Action(A),
    /// The input is not complete.
    Command(Command<A>),
}

#[derive(Clone)]
/// The missing sequence of inputs of a sequence.
pub enum Command<A: Action> {
    /// A single input is missing.
    Simple(A),
    /// An operator is missing.
    Operator(Handler<A>),
    // A prefix is missing.
    Prefix(Handler<A>),
}

#[derive(Clone)]
/// The state of the state machine.
pub(super) enum State<A: Action> {
    /// No input has yet been read.
    Normal,
    /// An operator input sequence was started and more input is needed.
    OperatorPending {
        handler: Rc<dyn Fn(Key) -> Option<ChainResult<A>>>,
    },
    /// A prefix input sequence was started and more input is needed.
    WaitingForMoreInput {
        handler: Rc<dyn Fn(Key) -> Option<ChainResult<A>>>,
        started: Instant,
    },
}
