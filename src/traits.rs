mod contents;
mod render;
mod tick;

pub use contents::Contents;
pub use render::Render;
pub use tick::Tick;

pub trait Buffer: Render + Tick + Contents {}
