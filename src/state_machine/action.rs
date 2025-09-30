pub trait Action: Clone {}
impl<T: Clone> Action for T {}
