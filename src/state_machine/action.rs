/// A marker trait to define the type of an action at the end of a state machine sequence.
pub trait Action: Clone {}
impl<T: Clone> Action for T {}
