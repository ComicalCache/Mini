mod action;
mod command_map;
mod states;

pub use command_map::{CommandMap, HandlerFn};
pub use states::{ChainResult, Command};

use crate::state_machine::{
    action::Action,
    states::{Input, State},
};
use std::{
    rc::Rc,
    time::{Duration, Instant},
};

#[derive(PartialEq, Eq)]
/// The result of ticking the state machine.
pub enum StateMachineResult<A: Action> {
    /// An action should be taken.
    Action(A),
    /// The sequence is incomplete.
    Incomplete,
    /// The input sequence is invalid.
    Invalid,
}

/// A state machine processing user input key by key.
pub struct StateMachine<A: Action> {
    /// Maps input commands to commands.
    pub command_map: CommandMap<A>,
    /// The current state.
    state: State<A>,
    /// The timeout duration for processing a prefix.
    timeout_duration: Duration,
}

impl<A: Action> StateMachine<A> {
    pub const fn new(command_map: CommandMap<A>, timeout_duration: Duration) -> Self {
        Self {
            command_map,
            state: State::Normal,
            timeout_duration,
        }
    }

    /// Advances the state machine by one step.
    pub fn tick(&mut self, input: Input) -> StateMachineResult<A> {
        let current_state = self.state.clone();
        self.state = State::Normal;

        match (&current_state, input) {
            // New input chain started.
            (State::Normal, Input::Key(key)) => match self.command_map.bindings.get(&key) {
                // Immediate action.
                Some(Command::Simple(action)) => StateMachineResult::Action(action.clone()),
                // Operator sequence started.
                Some(Command::Operator(handler)) => {
                    self.state = State::OperatorPending {
                        handler: Rc::clone(handler),
                    };
                    StateMachineResult::Incomplete
                }
                // Prefix sequence started.
                Some(Command::Prefix(handler)) => {
                    self.state = State::WaitingForMoreInput {
                        handler: Rc::clone(handler),
                        started: Instant::now(),
                    };
                    StateMachineResult::Incomplete
                }
                // Key is not mapped.
                None => StateMachineResult::Invalid,
            },
            // Operator sequence continues.
            (State::OperatorPending { handler }, Input::Key(key)) => {
                self.process_chain(handler(key))
            }
            // Prefix sequence continues.
            (State::WaitingForMoreInput { handler, started }, Input::Key(key)) => {
                if started.elapsed() > self.timeout_duration {
                    return self.tick(Input::Key(key));
                }
                self.process_chain(handler(key))
            }
            // Prefix sequence times out.
            (State::WaitingForMoreInput { started, .. }, Input::Timeout) => {
                if started.elapsed() > self.timeout_duration {
                    StateMachineResult::Invalid
                } else {
                    self.state = current_state;
                    StateMachineResult::Incomplete
                }
            }
            // Anything other case.
            (state, _) => {
                self.state = state.clone();
                StateMachineResult::Incomplete
            }
        }
    }

    fn process_chain(&mut self, result: Option<ChainResult<A>>) -> StateMachineResult<A> {
        match result {
            // Unwrap action.
            Some(ChainResult::Action(action)) => StateMachineResult::Action(action),
            // Unwrap simple.
            Some(ChainResult::Command(Command::Simple(action))) => {
                StateMachineResult::Action(action)
            }
            // Unwrap operator.
            Some(ChainResult::Command(Command::Operator(handler))) => {
                self.state = State::OperatorPending { handler };
                StateMachineResult::Incomplete
            }
            // Unwrap prefix.
            Some(ChainResult::Command(Command::Prefix(handler))) => {
                self.state = State::WaitingForMoreInput {
                    handler,
                    started: Instant::now(),
                };
                StateMachineResult::Incomplete
            }
            // Invalid key in sequence.
            None => StateMachineResult::Invalid,
        }
    }
}
