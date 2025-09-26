use crate::util::CommandResult;
use termion::event::Key;

pub trait Tick {
    fn tick(&mut self, key: Option<Key>) -> CommandResult;
}
