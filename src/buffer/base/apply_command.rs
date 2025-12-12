use crate::{
    INFO_MSG,
    buffer::{BufferKind, BufferResult, base::BaseBuffer},
    cursor::{self, Cursor},
    selection::{Selection, SelectionKind},
    util::line_column,
};
use regex::Regex;

impl<ModeEnum> BaseBuffer<ModeEnum> {
    fn search(&mut self, args: &str) -> BufferResult {
        if args.len() == 2 || !args.starts_with('/') || !args.ends_with('/') {
            return BufferResult::Error(
                "Expected a valid regular expression like '/<regex>/'".to_string(),
            );
        }

        let regex = match Regex::new(&args[1..args.len() - 1]) {
            Ok(regex) => regex,
            Err(err) => {
                return BufferResult::Error(format!(
                    "'{args}' is not a valid regular expression:\n{err}"
                ));
            }
        };

        // Use selections or search entire buffer.
        self.selections.sort_unstable();
        let selections = if self.selections.is_empty() {
            // Save previous cursor position.
            let tmp_doc_cur = self.doc.cur;

            let start = Cursor::new(0, 0);
            cursor::jump_to_end_of_file(&mut self.doc);
            let end = self.doc.cur;

            // Restore previous cursor position.
            self.doc.cur = tmp_doc_cur;

            &vec![Selection::new(
                start,
                end,
                SelectionKind::Normal,
                None,
                None,
            )]
        } else {
            &self.selections
        };

        self.matches.clear();
        self.matches_idx = None;
        for selection in selections {
            let (start, end) = selection.range();
            let hay = self.doc.get_range(start, end).unwrap().to_string();
            self.matches = regex
                .find_iter(&hay)
                .map(|mat| {
                    let start_pos = cursor::end_pos(&start, &hay[..mat.start()]);
                    let end_pos = cursor::end_pos(&start, &hay[..mat.end()]);
                    (start_pos, end_pos)
                })
                .collect();
        }

        if self.matches.is_empty() {
            return BufferResult::Info("No matches found".to_string());
        }

        // Clear existing selections and select result.
        self.clear_selections();

        // Find closest match to cursor.
        self.matches_idx = self
            .matches
            .iter()
            .enumerate()
            .find_map(|(idx, (start, _))| self.doc.cur.le(start).then_some(idx))
            // Or use last match if before current cursor position.
            .or(Some(self.matches.len() - 1));

        // Select the closest match to cursor and jump there.
        let idx = self.matches_idx.unwrap();
        self.selections.push(Selection::new(
            self.matches[idx].0,
            self.matches[idx].1,
            SelectionKind::Normal,
            None,
            None,
        ));
        cursor::move_to(&mut self.doc, self.matches[idx].0);

        BufferResult::Ok
    }

    fn goto(&mut self, args: &str) -> BufferResult {
        let (x, y) = line_column(args);

        let mut pos = self.doc.cur;
        if let Some(x) = x {
            pos.x = x.saturating_sub(1);
        }
        if let Some(y) = y {
            pos.y = y.saturating_sub(1);
        }
        cursor::move_to(&mut self.doc, pos);

        BufferResult::Ok
    }

    /// Applies the command entered during command mode.
    pub fn apply_command(&mut self, input: String) -> Result<BufferResult, String> {
        if input.is_empty() {
            return Ok(BufferResult::Ok);
        }

        let (cmd, args) = match input.split_once(char::is_whitespace) {
            Some((cmd, args)) => (cmd.trim(), args.trim()),
            None => (input.trim(), ""),
        };

        match cmd {
            "q" => Ok(BufferResult::Quit),
            "qq" => Ok(BufferResult::ForceQuit),
            "?" => Ok(BufferResult::Info(format!(
                "Mini - A terminal text-editor (v{})\n\n{INFO_MSG}",
                option_env!("CARGO_PKG_VERSION").or(Some("?.?.?")).unwrap()
            ))),
            "j" => Ok(self.goto(args)),
            "s" => Ok(self.search(args)),
            "cb" => match args.parse::<usize>() {
                Ok(idx) => Ok(BufferResult::Change(idx)),
                Err(err) => Ok(BufferResult::Error(err.to_string())),
            },
            "lb" => Ok(BufferResult::ListBuffers),
            #[allow(clippy::option_if_let_else)]
            "nb" => match BufferKind::from(args) {
                Some(kind) => Ok(BufferResult::NewBuffer(kind)),
                None => Ok(BufferResult::Error(format!(
                    "'{args}' is not a valid buffer kind. Try one of these:\n{}",
                    BufferKind::list()
                ))),
            },
            "log" => Ok(BufferResult::Log),
            _ => Err(input),
        }
    }
}
