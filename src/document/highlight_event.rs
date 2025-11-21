use termion::color::{Bg, Fg, Rgb};

pub enum HighlightEvent {
    HighlightStart { fg: Fg<Rgb>, bg: Bg<Rgb> },
    HighlightEnd,
    Source { start: usize, end: usize },
}
