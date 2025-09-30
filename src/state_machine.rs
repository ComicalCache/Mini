mod action;
mod command_map;
mod states;

pub use command_map::{Command, CommandMap, HandlerFn};
pub use states::ChainResult;

use crate::state_machine::{
    action::Action,
    states::{Input, State},
};
use std::{
    rc::Rc,
    time::{Duration, Instant},
};

#[derive(PartialEq)]
pub enum StateMachineResult<A: Action> {
    Action(A),
    Incomplete,
    Invalid,
}

pub struct StateMachine<A: Action> {
    command_map: CommandMap<A>,
    state: State<A>,
    timeout_duration: Duration,
}

impl<A: Action> StateMachine<A> {
    pub fn new(command_map: CommandMap<A>, timeout_duration: Duration) -> Self {
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
            (State::Normal, Input::Key(key)) => match self.command_map.bindings.get(&key) {
                Some(Command::Simple(action)) => StateMachineResult::Action(action.clone()),
                Some(Command::Operator(handler)) => {
                    self.state = State::OperatorPending {
                        handler: Rc::clone(handler),
                    };
                    StateMachineResult::Incomplete
                }
                Some(Command::Prefix(handler)) => {
                    self.state = State::WaitingForMoreInput {
                        handler: Rc::clone(handler),
                        started: Instant::now(),
                    };
                    StateMachineResult::Incomplete
                }
                None => StateMachineResult::Invalid,
            },
            (State::OperatorPending { handler }, Input::Key(key)) => {
                self.process_chain(handler(key))
            }
            (State::WaitingForMoreInput { handler, started }, Input::Key(key)) => {
                if started.elapsed() > self.timeout_duration {
                    return self.tick(Input::Key(key));
                }
                self.process_chain(handler(key))
            }
            (State::WaitingForMoreInput { started, .. }, Input::Timeout) => {
                if started.elapsed() > self.timeout_duration {
                    StateMachineResult::Invalid
                } else {
                    self.state = current_state;
                    StateMachineResult::Incomplete
                }
            }
            (state, _) => {
                self.state = state.clone();
                StateMachineResult::Incomplete
            }
        }
    }

    fn process_chain(&mut self, result: Option<ChainResult<A>>) -> StateMachineResult<A> {
        match result {
            Some(ChainResult::Action(action)) => StateMachineResult::Action(action),
            Some(ChainResult::Command(Command::Simple(action))) => {
                StateMachineResult::Action(action)
            }
            Some(ChainResult::Command(Command::Operator(handler))) => {
                self.state = State::OperatorPending { handler };
                StateMachineResult::Incomplete
            }
            Some(ChainResult::Command(Command::Prefix(handler))) => {
                self.state = State::WaitingForMoreInput {
                    handler,
                    started: Instant::now(),
                };
                StateMachineResult::Incomplete
            }
            None => StateMachineResult::Invalid,
        }
    }
}
