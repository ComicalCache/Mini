mod render;
mod tick;

pub use render::Render;
pub use tick::Tick;

pub trait Buffer: Render + Tick {}
