use super::ShellCommandResult;
use vte::Perform;

pub(super) struct Performer {
    pub(super) output: Vec<ShellCommandResult>,
}

impl Performer {
    pub fn new() -> Self {
        Self { output: Vec::new() }
    }
}

impl Perform for Performer {
    fn print(&mut self, c: char) {
        use ShellCommandResult::Data;

        if let Some(Data(s)) = self.output.last_mut() {
            s.push(c);
        } else {
            self.output.push(Data(c.to_string()));
        }
    }

    fn execute(&mut self, byte: u8) {
        use ShellCommandResult::{CarriageReturn, Data};

        match byte {
            b'\n' => {
                if let Some(Data(s)) = self.output.last_mut() {
                    s.push('\n');
                } else {
                    self.output.push(Data("\n".to_string()));
                }
            }
            b'\r' => self.output.push(CarriageReturn),
            _ => {}
        }
    }
}
