//! In-TUI code editor screen.
//!
//! Replaces the external editor with a native ratatui-based editor that shows
//! the problem description, an editable code pane, and inline action buttons
//! for running the example test cases and submitting the solution. Everything
//! is held in memory — no files are written to disk.
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers, MouseButton, MouseEvent, MouseEventKind};
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Position, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Paragraph},
};
use ratatui_textarea::{CursorMove, TextArea};

use crate::format::result_to_string;
use crate::models::{Identifier, Language};
use crate::picker::Picker;
use crate::services::submission::SubmissionResult;
use crate::tui::{Action, screen::Screen};

/// Which pane has keyboard focus in the editor.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EditorFocus {
    Code,
    Buttons,
    Results,
}

/// Lifecycle status shown in the status bar.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EditorStatus {
    Idle,
    Loading,
    Testing,
    Submitting,
    Done,
}

impl EditorStatus {
    fn label(&self) -> &'static str {
        match self {
            EditorStatus::Idle => "Ready",
            EditorStatus::Loading => "Loading problem...",
            EditorStatus::Testing => "Running tests...",
            EditorStatus::Submitting => "Submitting...",
            EditorStatus::Done => "Done",
        }
    }
}

/// Vim-like modal editing modes for the code editor.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VimMode {
    Normal,
    Insert,
    Visual,
}

impl VimMode {
    fn label(&self) -> &'static str {
        match self {
            VimMode::Normal => "NORMAL",
            VimMode::Insert => "INSERT",
            VimMode::Visual => "VISUAL",
        }
    }

    fn color(&self) -> Color {
        match self {
            VimMode::Normal => crate::theme::FG_MUTED,
            VimMode::Insert => crate::theme::EASY,
            VimMode::Visual => crate::theme::VISUAL_BG,
        }
    }
}

/// Motion target for a pending operator (`d`/`y`/`c`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum OpMotion {
    Line,
    Word,
    WordEnd,
    ToEnd,
    ToHead,
}

/// The native code editor screen.
pub struct EditorScreen {
    pub description: String,
    pub slug: String,
    pub question_id: String,
    pub language: String,
    pub editor: TextArea<'static>,
    pub results: Vec<String>,
    pub status: EditorStatus,
    pub focus: EditorFocus,
    pub button_index: usize,
    pub busy: bool,
    pub result_rx: Option<tokio::sync::oneshot::Receiver<Result<SubmissionResult, String>>>,
    /// Vim-like modal editing mode for the code pane.
    pub mode: VimMode,
    /// Tracks a pending `g` for the `gg` (jump to top) key sequence.
    pub pending_g: bool,
    /// Pending operator (`d`/`y`/`c`) waiting for a motion key.
    pub pending_operator: Option<char>,
    /// When true, the next character replaces the char under the cursor (`r`).
    pub replace_next: bool,
    /// Rect of the code editor pane (set during render) for mouse mapping.
    pub code_area: Rect,
    /// Rect of the action bar (set during render) for mouse button clicks.
    pub action_area: Rect,
    /// Rect of the results pane (set during render) for mouse scrolling.
    pub results_area: Rect,
    /// Vertical scroll offset for the results pane.
    pub results_scroll: u16,
    /// Top visible code line (our own viewport, not the TextArea's).
    pub scroll_row: usize,
    /// Whether a mouse drag selection is in progress.
    pub mouse_selecting: bool,
}

impl EditorScreen {
    pub fn new() -> Self {
        let mut editor = TextArea::default();
        editor.set_line_number_style(Style::default().fg(crate::theme::FG_SUBTLE));
        editor.set_cursor_line_style(Style::default().add_modifier(Modifier::BOLD));
        Self {
            description: String::new(),
            slug: String::new(),
            question_id: String::new(),
            language: "python3".to_string(),
            editor,
            results: Vec::new(),
            status: EditorStatus::Idle,
            focus: EditorFocus::Code,
            button_index: 0,
            busy: false,
            result_rx: None,
            mode: VimMode::Normal,
            pending_g: false,
            pending_operator: None,
            replace_next: false,
            code_area: Rect::default(),
            action_area: Rect::default(),
            results_area: Rect::default(),
            results_scroll: 0,
            scroll_row: 0,
            mouse_selecting: false,
        }
    }

    /// Fetches the problem fully into memory and loads the code + description
    /// into the editor without writing any files to disk.
    ///
    /// If a previous solution for this (slug, language) was saved and its last
    /// submission was not accepted, it resumes from that code instead of the
    /// fresh stub.
    pub async fn load(
        &mut self,
        picker: &Picker,
        identifier: &Identifier,
        language: &Option<Language>,
    ) -> Result<(), String> {
        self.status = EditorStatus::Loading;
        let problem = picker
            .fetch_in_memory(identifier, language)
            .await
            .map_err(|e| format!("{}", e))?;

        let slug = problem.slug.clone();
        let lang_slug = problem.language.to_lang_slug().to_string();
        let mut code = problem.code.clone();
        // Resume from the last submission if it was unsuccessful.
        if let Ok(Some(last)) = picker.get_last_submission(&slug, &lang_slug).await
            && !last.accepted
        {
            code = last.code;
        }

        let lines: Vec<String> = code.lines().map(|s| s.to_string()).collect();
        self.editor = TextArea::new(lines);
        self.editor
            .set_line_number_style(Style::default().fg(crate::theme::FG_SUBTLE));
        self.editor
            .set_cursor_line_style(Style::default().add_modifier(Modifier::BOLD));

        self.slug = slug;
        self.question_id = problem.question_id.clone();
        self.language = lang_slug;
        self.description = problem.description;
        self.results.clear();
        self.status = EditorStatus::Idle;
        self.busy = false;
        self.result_rx = None;
        self.mode = VimMode::Normal;
        self.pending_g = false;
        self.pending_operator = None;
        self.replace_next = false;
        self.scroll_row = 0;
        Ok(())
    }

    /// Returns the display filename for the code pane title (in memory only).
    fn display_filename(&self) -> String {
        let lang = Language::from_leetcode_slug(&self.language).unwrap_or(Language::Rust);
        format!("{}.{}", self.slug.replace('-', "_"), lang.code_extension())
    }

    /// Spawns a background task to run the example test cases against the
    /// in-memory code, keeping the TUI responsive while LeetCode judges.
    pub fn start_tests(&mut self, picker: &Picker) {
        if self.busy || self.slug.is_empty() {
            return;
        }
        self.busy = true;
        self.status = EditorStatus::Testing;
        self.results.clear();
        self.results.push("Running test cases...".to_string());

        let picker = picker.clone();
        let code = self.editor.lines().join("\n");
        let slug = self.slug.clone();
        let language = Language::from_leetcode_slug(&self.language).unwrap_or(Language::Rust);
        let (tx, rx) = tokio::sync::oneshot::channel();
        self.result_rx = Some(rx);
        tokio::spawn(async move {
            let res = picker
                .run_tests_memory(&code, &slug, language)
                .await
                .map_err(|e| e.to_string());
            let _ = tx.send(res);
        });
    }

    /// Spawns a background task to submit the in-memory code for full judging.
    pub fn start_submit(&mut self, picker: &Picker) {
        if self.busy || self.slug.is_empty() {
            return;
        }
        self.busy = true;
        self.status = EditorStatus::Submitting;
        self.results.clear();
        self.results.push("Submitting...".to_string());

        let picker = picker.clone();
        let code = self.editor.lines().join("\n");
        let slug = self.slug.clone();
        let language = Language::from_leetcode_slug(&self.language).unwrap_or(Language::Rust);
        let (tx, rx) = tokio::sync::oneshot::channel();
        self.result_rx = Some(rx);
        tokio::spawn(async move {
            let res = picker
                .submit_solution_memory(&code, &slug, language)
                .await
                .map_err(|e| e.to_string());
            let _ = tx.send(res);
        });
    }

    /// Polls the background submission task and updates the results pane when
    /// it completes. Called once per event-loop iteration.
    pub fn tick(&mut self) {
        if let Some(rx) = &mut self.result_rx {
            match rx.try_recv() {
                Ok(result) => {
                    self.result_rx = None;
                    self.busy = false;
                    self.status = EditorStatus::Done;
                    match result {
                        Ok(r) => {
                            self.results = result_to_string(&r)
                                .lines()
                                .map(|s| s.to_string())
                                .collect();
                            self.results_scroll = 0;
                        }
                        Err(e) => {
                            self.results = vec![format!("Error: {}", e)];
                            self.results_scroll = 0;
                        }
                    }
                }
                Err(tokio::sync::oneshot::error::TryRecvError::Empty) => {}
                Err(tokio::sync::oneshot::error::TryRecvError::Closed) => {
                    self.result_rx = None;
                    self.busy = false;
                    self.status = EditorStatus::Done;
                    self.results.push("Task ended unexpectedly.".to_string());
                }
            }
        }
    }

    fn toggle_focus(&mut self) {
        self.focus = match self.focus {
            EditorFocus::Code => EditorFocus::Buttons,
            EditorFocus::Buttons => EditorFocus::Results,
            EditorFocus::Results => EditorFocus::Code,
        };
    }

    /// Handles keys in Vim normal mode (navigation + mode switching).
    fn handle_normal_key(&mut self, key: &KeyEvent) {
        // Ctrl-based shortcuts (redo).
        if key.modifiers.contains(KeyModifiers::CONTROL) {
            if key.code == KeyCode::Char('r') {
                self.editor.redo();
            }
            return;
        }
        // Any key other than `g` clears a pending `gg` sequence.
        if key.code != KeyCode::Char('g') {
            self.pending_g = false;
        }

        // Replace mode: the next character replaces the char under the cursor.
        if self.replace_next {
            self.replace_next = false;
            if let KeyCode::Char(c) = key.code
                && !self.editor.is_empty()
            {
                self.editor.delete_next_char();
                self.editor.insert_char(c);
            }
            return;
        }

        // Operator pending: `d`/`y`/`c` followed by a motion.
        if let Some(op) = self.pending_operator {
            self.pending_operator = None;
            self.handle_operator(op, key);
            return;
        }

        match key.code {
            // --- Mode switches ---
            KeyCode::Char('i') => self.mode = VimMode::Insert,
            KeyCode::Char('a') => {
                self.editor.move_cursor(CursorMove::Forward);
                self.mode = VimMode::Insert;
            }
            KeyCode::Char('A') => {
                self.editor.move_cursor(CursorMove::End);
                self.mode = VimMode::Insert;
            }
            KeyCode::Char('I') => {
                self.editor.move_cursor(CursorMove::Head);
                self.mode = VimMode::Insert;
            }
            KeyCode::Char('o') => {
                self.editor.move_cursor(CursorMove::End);
                self.editor.insert_newline();
                self.mode = VimMode::Insert;
            }
            KeyCode::Char('O') => {
                self.editor.move_cursor(CursorMove::Head);
                self.editor.insert_newline();
                self.editor.move_cursor(CursorMove::Up);
                self.mode = VimMode::Insert;
            }
            KeyCode::Char('v') => {
                self.mode = VimMode::Visual;
                self.editor.start_selection();
            }
            KeyCode::Char('V') => {
                self.mode = VimMode::Visual;
                self.editor.move_cursor(CursorMove::Head);
                self.editor.start_selection();
                self.editor.move_cursor(CursorMove::End);
            }

            // --- Operators (dd, yy, dw, yw, d$, etc.) ---
            KeyCode::Char('d') => self.pending_operator = Some('d'),
            KeyCode::Char('y') => self.pending_operator = Some('y'),
            KeyCode::Char('c') => self.pending_operator = Some('c'),

            // --- Motions ---
            KeyCode::Char('h') | KeyCode::Left => self.editor.move_cursor(CursorMove::Back),
            KeyCode::Char('l') | KeyCode::Right => self.editor.move_cursor(CursorMove::Forward),
            KeyCode::Char('j') | KeyCode::Down => self.editor.move_cursor(CursorMove::Down),
            KeyCode::Char('k') | KeyCode::Up => self.editor.move_cursor(CursorMove::Up),
            KeyCode::Char('w') => self.editor.move_cursor(CursorMove::WordForward),
            KeyCode::Char('b') => self.editor.move_cursor(CursorMove::WordBack),
            KeyCode::Char('e') => self.editor.move_cursor(CursorMove::WordEnd),
            KeyCode::Char('0') | KeyCode::Char('^') => self.editor.move_cursor(CursorMove::Head),
            KeyCode::Char('$') => self.editor.move_cursor(CursorMove::End),
            KeyCode::Char('g') => {
                if self.pending_g {
                    self.editor.move_cursor(CursorMove::Top);
                    self.pending_g = false;
                } else {
                    self.pending_g = true;
                }
            }
            KeyCode::Char('G') => self.editor.move_cursor(CursorMove::Bottom),
            KeyCode::PageDown => {
                for _ in 0..10 {
                    self.editor.move_cursor(CursorMove::Down);
                }
            }
            KeyCode::PageUp => {
                for _ in 0..10 {
                    self.editor.move_cursor(CursorMove::Up);
                }
            }

            // --- Edits ---
            KeyCode::Char('x') => {
                self.editor.delete_next_char();
            }
            KeyCode::Char('X') => {
                self.editor.delete_char();
            }
            KeyCode::Char('u') => {
                self.editor.undo();
            }
            KeyCode::Char('p') => {
                self.editor.paste();
            }
            KeyCode::Char('P') => {
                self.editor.move_cursor(CursorMove::Back);
                self.editor.paste();
            }
            KeyCode::Char('D') => {
                self.editor.delete_line_by_end();
            }
            KeyCode::Char('C') => {
                self.editor.delete_line_by_end();
                self.mode = VimMode::Insert;
            }
            KeyCode::Char('r') => self.replace_next = true,
            KeyCode::BackTab => {
                self.outdent_line();
            }
            _ => {}
        }
    }

    /// Applies a pending operator (`d`/`y`/`c`) to the motion given by `key`.
    fn handle_operator(&mut self, op: char, key: &KeyEvent) {
        let motion = match key.code {
            KeyCode::Char('d') | KeyCode::Char('y') | KeyCode::Char('c') => OpMotion::Line,
            KeyCode::Char('w') => OpMotion::Word,
            KeyCode::Char('e') => OpMotion::WordEnd,
            KeyCode::Char('$') => OpMotion::ToEnd,
            KeyCode::Char('0') | KeyCode::Char('^') => OpMotion::ToHead,
            _ => return,
        };

        match motion {
            OpMotion::Line => {
                self.editor.move_cursor(CursorMove::Head);
                self.editor.start_selection();
                let start = self.editor.cursor();
                self.editor.move_cursor(CursorMove::Down);
                if start == self.editor.cursor() {
                    self.editor.move_cursor(CursorMove::End);
                }
                self.apply_operator(op);
                self.editor.cancel_selection();
            }
            OpMotion::Word => {
                self.editor.start_selection();
                self.editor.move_cursor(CursorMove::WordForward);
                self.apply_operator(op);
                self.editor.cancel_selection();
            }
            OpMotion::WordEnd => {
                self.editor.start_selection();
                self.editor.move_cursor(CursorMove::WordEnd);
                self.apply_operator(op);
                self.editor.cancel_selection();
            }
            OpMotion::ToEnd => {
                self.editor.start_selection();
                self.editor.move_cursor(CursorMove::End);
                self.apply_operator(op);
                self.editor.cancel_selection();
            }
            OpMotion::ToHead => {
                self.editor.start_selection();
                self.editor.move_cursor(CursorMove::Head);
                self.apply_operator(op);
                self.editor.cancel_selection();
            }
        }
    }

    fn apply_operator(&mut self, op: char) {
        match op {
            'y' => {
                self.editor.copy();
            }
            'd' => {
                self.editor.cut();
            }
            'c' => {
                self.editor.cut();
                self.mode = VimMode::Insert;
            }
            _ => {}
        }
    }

    /// Handles keys in Vim insert mode (text editing; Esc returns to normal).
    fn handle_insert_key(&mut self, key: &KeyEvent) {
        if key.code == KeyCode::Esc {
            self.mode = VimMode::Normal;
            return;
        }
        if key.code == KeyCode::BackTab {
            self.outdent_line();
            return;
        }
        if key.code == KeyCode::Enter {
            self.insert_newline_indented();
            return;
        }
        self.editor.input(*key);
    }

    /// Inserts a newline and auto-indents it based on the code's structure
    /// (block depth from delimiters, or keyword blocks for Python).
    fn insert_newline_indented(&mut self) {
        let cursor = self.editor.cursor();
        let row = cursor.0;
        let lines = self.editor.lines().to_vec();
        let indent_str = self.editor.indent().to_string();

        let is_python = matches!(self.language.as_str(), "python" | "python3" | "pythondata");

        let new_indent = if is_python {
            // Python uses keyword blocks (`def`, `if`, ...:`), not braces.
            let current = lines.get(row).cloned().unwrap_or_default();
            let leading: String = current.chars().take_while(|c| c.is_whitespace()).collect();
            if current.trim_end().ends_with(':') {
                format!("{}{}", leading, indent_str)
            } else {
                leading
            }
        } else {
            // Brace-based languages: compute the block depth from the buffer,
            // then adjust for openers/closers on the current line.
            let depth = Self::net_depth(&lines, row, &self.language);
            let current = lines.get(row).cloned().unwrap_or_default();
            let trimmed_end = current.trim_end();
            let trimmed_start = current.trim_start();

            let mut level = depth;
            let ends_opener = trimmed_end.ends_with('{')
                || trimmed_end.ends_with('(')
                || trimmed_end.ends_with('[');
            let starts_closer = trimmed_start.starts_with('}')
                || trimmed_start.starts_with(')')
                || trimmed_start.starts_with(']');

            if ends_opener {
                level += 1;
            } else if starts_closer {
                level = level.saturating_sub(1);
            }
            indent_str.repeat(level)
        };

        self.editor.insert_newline();
        if !new_indent.is_empty() {
            self.editor.insert_str(&new_indent);
        }
    }

    /// Counts the net number of open delimiters (`{`, `(`, `[`) on the lines
    /// before `row`, ignoring strings and comments. This is the block depth the
    /// current line starts at.
    fn net_depth(lines: &[String], up_to: usize, lang: &str) -> usize {
        let (line_comments, block_comments) = Self::comment_markers(lang);
        let mut depth: isize = 0;
        let mut in_block = false;

        for line in lines.iter().take(up_to) {
            let bytes = line.as_bytes();
            let mut i = 0;
            while i < bytes.len() {
                let rest = &line[i..];
                if in_block
                    && let Some((_, close)) = block_comments
                    && let Some(pos) = rest.find(close)
                {
                    i += pos + close.len();
                    in_block = false;
                    continue;
                }
                if in_block {
                    break;
                }
                if let Some((open, close)) = block_comments
                    && rest.starts_with(open)
                {
                    in_block = true;
                    if let Some(pos) = rest.find(close) {
                        i += pos + close.len();
                        in_block = false;
                        continue;
                    }
                    break;
                }
                if line_comments.iter().any(|c| rest.starts_with(c)) {
                    break;
                }
                let ch = bytes[i] as char;
                if ch == '"' || ch == '\'' || ch == '`' {
                    i += 1;
                    let mut esc = false;
                    while i < bytes.len() {
                        let c2 = bytes[i] as char;
                        if esc {
                            esc = false;
                        } else if c2 == '\\' {
                            esc = true;
                        } else if c2 == ch {
                            i += 1;
                            break;
                        }
                        i += 1;
                    }
                    continue;
                }
                match ch {
                    '{' | '(' | '[' => depth += 1,
                    '}' | ')' | ']' => depth = depth.saturating_sub(1),
                    _ => {}
                }
                i += 1;
            }
        }
        depth.max(0) as usize
    }

    /// Returns the comment markers for a language.
    fn comment_markers(
        lang: &str,
    ) -> (
        &'static [&'static str],
        Option<(&'static str, &'static str)>,
    ) {
        match lang {
            "python" | "python3" | "pythondata" | "bash" | "sh" => (&["#"], None),
            "mysql" | "mssql" | "postgresql" | "oraclesql" | "sql" => (&["--"], Some(("/*", "*/"))),
            _ => (&["//"], Some(("/*", "*/"))),
        }
    }

    /// Removes one level of indentation from the start of the current line.
    fn outdent_line(&mut self) {
        let cursor = self.editor.cursor();
        let row = cursor.0;
        let lines = self.editor.lines().to_vec();
        if let Some(line) = lines.get(row) {
            let remove = if line.starts_with('\t') {
                1
            } else {
                let tab_len = self.editor.tab_length() as usize;
                let leading = line.chars().take_while(|c| *c == ' ').count();
                leading.min(tab_len)
            };
            if remove > 0 {
                self.editor.move_cursor(CursorMove::Head);
                self.editor.delete_str(remove);
            }
        }
    }

    /// Handles keys in Vim visual mode (selection; y/d/c operate on the selection).
    fn handle_visual_key(&mut self, key: &KeyEvent) {
        match key.code {
            KeyCode::Esc | KeyCode::Char('v') => {
                self.mode = VimMode::Normal;
                self.editor.cancel_selection();
            }
            KeyCode::Char('h') | KeyCode::Left => self.editor.move_cursor(CursorMove::Back),
            KeyCode::Char('l') | KeyCode::Right => self.editor.move_cursor(CursorMove::Forward),
            KeyCode::Char('j') | KeyCode::Down => self.editor.move_cursor(CursorMove::Down),
            KeyCode::Char('k') | KeyCode::Up => self.editor.move_cursor(CursorMove::Up),
            KeyCode::Char('w') => self.editor.move_cursor(CursorMove::WordForward),
            KeyCode::Char('b') => self.editor.move_cursor(CursorMove::WordBack),
            KeyCode::Char('y') => {
                // Move forward so the char under the cursor is included (the
                // selection range is exclusive at the end).
                self.editor.move_cursor(CursorMove::Forward);
                self.editor.copy();
                self.mode = VimMode::Normal;
                self.editor.cancel_selection();
            }
            KeyCode::Char('d') => {
                self.editor.move_cursor(CursorMove::Forward);
                self.editor.cut();
                self.mode = VimMode::Normal;
            }
            KeyCode::Char('c') => {
                self.editor.move_cursor(CursorMove::Forward);
                self.editor.cut();
                self.mode = VimMode::Insert;
            }
            _ => {}
        }
    }
}

impl Screen for EditorScreen {
    fn render(&mut self, frame: &mut Frame) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .margin(1)
            .constraints([
                Constraint::Length(1),
                Constraint::Min(1),
                Constraint::Length(3),
                Constraint::Length(12),
            ])
            .split(frame.area());

        // Status bar with a prominent, colored mode indicator.
        let mode_span = ratatui::text::Span::styled(
            format!(" {} ", self.mode.label()),
            Style::default().fg(crate::theme::BG).bg(self.mode.color()),
        );
        let status_line = ratatui::text::Line::from(vec![
            ratatui::text::Span::raw(format!(
                " {}{}  |  Language: {}  |  Status: {}  |  ",
                self.slug,
                if self.busy { " (busy)" } else { "" },
                self.language,
                self.status.label(),
            )),
            mode_span,
            ratatui::text::Span::raw("  |  F5: Test  F6: Submit  F7: Back  F8: Next  F9: Prev"),
        ]);
        frame.render_widget(
            Paragraph::new(status_line).style(Style::default().fg(crate::theme::FG)),
            chunks[0],
        );

        // Main split: description (left) + code editor (right)
        let main = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(40), Constraint::Percentage(60)])
            .split(chunks[1]);

        let desc_block = Block::bordered().title(" Problem Description ");
        let desc = Paragraph::new(self.description.as_str())
            .block(desc_block)
            .wrap(ratatui::widgets::Wrap { trim: true });
        frame.render_widget(desc, main[0]);

        // The editor border reflects the active mode for a strong visual cue.
        self.render_code_pane(frame, main[1]);

        // Action bar — border brightens when focused.
        let action_focus = self.focus == EditorFocus::Buttons;
        let action_block = Block::bordered()
            .border_style(if action_focus {
                Style::default().fg(crate::theme::MEDIUM)
            } else {
                Style::default().fg(crate::theme::FG_SUBTLE)
            })
            .title(" Actions ");
        let inner = action_block.inner(chunks[2]);
        self.action_area = inner;
        frame.render_widget(action_block, chunks[2]);

        let buttons = ["Run Tests (F5)", "Submit (F6)"];
        let mut bar = Vec::new();
        for (i, label) in buttons.iter().enumerate() {
            let selected = self.focus == EditorFocus::Buttons && self.button_index == i;
            let style = if selected {
                Style::default()
                    .fg(crate::theme::BG)
                    .bg(crate::theme::ACCENT)
            } else {
                Style::default().fg(crate::theme::FG_SUBTLE)
            };
            bar.push(ratatui::text::Span::styled(format!(" [{}] ", label), style));
            bar.push(ratatui::text::Span::raw("   "));
        }
        frame.render_widget(Paragraph::new(ratatui::text::Line::from(bar)), inner);

        // Results pane (interactive: scrollable when focused or with the wheel)
        let results_focus = self.focus == EditorFocus::Results;
        let results_block = Block::bordered()
            .border_style(if results_focus {
                Style::default()
                    .fg(crate::theme::FG)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(crate::theme::FG_SUBTLE)
            })
            .title(format!(
                " Results ({} lines){} ",
                self.results.len(),
                if results_focus { " ●" } else { "" }
            ));
        let results_text = self.results.join("\n");
        let results = Paragraph::new(results_text)
            .block(results_block)
            .scroll((self.results_scroll, 0))
            .wrap(ratatui::widgets::Wrap { trim: true });
        self.results_area = chunks[3];
        frame.render_widget(results, chunks[3]);
    }

    fn event_loop(&mut self, key_event: &KeyEvent) -> Option<Action> {
        // Global shortcuts that work regardless of focus.
        match key_event.code {
            KeyCode::F(5) => return Some(Action::RunTests),
            KeyCode::F(6) => return Some(Action::Submit),
            KeyCode::F(7) => return Some(Action::BackToSelection),
            KeyCode::F(8) => return Some(Action::NextProblem),
            KeyCode::F(9) => return Some(Action::PrevProblem),
            KeyCode::Tab if key_event.modifiers.contains(KeyModifiers::CONTROL) => {
                self.toggle_focus();
                return None;
            }
            KeyCode::Char('c') if key_event.modifiers.contains(KeyModifiers::CONTROL) => {
                return Some(Action::Quit);
            }
            _ => {}
        }

        match self.focus {
            EditorFocus::Code => match self.mode {
                VimMode::Normal => self.handle_normal_key(key_event),
                VimMode::Insert => self.handle_insert_key(key_event),
                VimMode::Visual => self.handle_visual_key(key_event),
            },
            EditorFocus::Buttons => match key_event.code {
                KeyCode::Tab | KeyCode::Esc => {
                    self.toggle_focus();
                }
                KeyCode::Left | KeyCode::Char('h') => self.button_index = 0,
                KeyCode::Right | KeyCode::Char('l') => self.button_index = 1,
                KeyCode::Enter => {
                    return if self.button_index == 0 {
                        Some(Action::RunTests)
                    } else {
                        Some(Action::Submit)
                    };
                }
                KeyCode::Char('j') | KeyCode::Down => self.button_index = 0,
                KeyCode::Char('k') | KeyCode::Up => self.button_index = 1,
                _ => {}
            },
            EditorFocus::Results => match key_event.code {
                KeyCode::Esc | KeyCode::Tab => {
                    self.toggle_focus();
                }
                KeyCode::Down | KeyCode::Char('j') => {
                    self.results_scroll = self.results_scroll.saturating_add(1);
                }
                KeyCode::Up | KeyCode::Char('k') => {
                    self.results_scroll = self.results_scroll.saturating_sub(1);
                }
                KeyCode::PageDown => {
                    self.results_scroll = self.results_scroll.saturating_add(10);
                }
                KeyCode::PageUp => {
                    self.results_scroll = self.results_scroll.saturating_sub(10);
                }
                KeyCode::Home => {
                    self.results_scroll = 0;
                }
                KeyCode::End => {
                    self.results_scroll = self.results.len().saturating_sub(1) as u16;
                }
                _ => {}
            },
        }
        None
    }

    fn handle_mouse(&mut self, event: &MouseEvent) {
        // Click/drag on a button.
        if self.action_area.contains(Position {
            x: event.column,
            y: event.row,
        }) {
            if let MouseEventKind::Down(MouseButton::Left) = event.kind {
                let mid = self.action_area.x + self.action_area.width / 2;
                self.button_index = if event.column < mid { 0 } else { 1 };
            }
            return;
        }

        // Click/scroll on the results pane.
        if self.results_area.contains(Position {
            x: event.column,
            y: event.row,
        }) {
            match event.kind {
                MouseEventKind::ScrollUp => {
                    self.results_scroll = self.results_scroll.saturating_sub(3);
                }
                MouseEventKind::ScrollDown => {
                    self.results_scroll = self.results_scroll.saturating_add(3);
                }
                MouseEventKind::Down(MouseButton::Left) => {
                    self.focus = EditorFocus::Results;
                }
                _ => {}
            }
            return;
        }

        if !self.code_area.contains(Position {
            x: event.column,
            y: event.row,
        }) {
            return;
        }

        match event.kind {
            MouseEventKind::ScrollUp => {
                for _ in 0..3 {
                    self.editor.move_cursor(CursorMove::Up);
                }
            }
            MouseEventKind::ScrollDown => {
                for _ in 0..3 {
                    self.editor.move_cursor(CursorMove::Down);
                }
            }
            MouseEventKind::Down(MouseButton::Left) => {
                self.mouse_selecting = true;
                let (row, col) = self.mouse_to_text(event);
                self.editor
                    .move_cursor(CursorMove::Jump(row as u16, col as u16));
                self.editor.start_selection();
            }
            MouseEventKind::Drag(MouseButton::Left) if self.mouse_selecting => {
                // Dragging creates a text selection; switch to visual mode so
                // `y`/`d`/`c` operate on it.
                if self.mode != VimMode::Visual {
                    self.mode = VimMode::Visual;
                }
                let (row, col) = self.mouse_to_text(event);
                self.editor
                    .move_cursor(CursorMove::Jump(row as u16, col as u16));
            }
            MouseEventKind::Up(MouseButton::Left) => {
                self.mouse_selecting = false;
            }
            _ => {}
        }
    }
}

impl EditorScreen {
    /// Translates a mouse position in the code pane into a text (row, col).
    ///
    /// Accounts for the block border, the line-number gutter, and our own
    /// viewport scroll so a click lands on the correct character.
    fn mouse_to_text(&self, event: &MouseEvent) -> (usize, usize) {
        let content_top = self.code_area.y as usize + 1;
        let gutter = self.line_number_width();
        let content_left = self.code_area.x as usize + 1 + gutter + 1;

        let row = self.scroll_row + (event.row as usize).saturating_sub(content_top);
        let col = (event.column as usize).saturating_sub(content_left);
        (row, col)
    }

    fn line_number_width(&self) -> usize {
        self.editor.lines().len().max(1).to_string().len()
    }

    /// Renders the code pane with syntax highlighting, line numbers, the cursor
    /// line, and a selection highlight. The `TextArea` keeps the editing state;
    /// we manage our own viewport (`scroll_row`) so scrolling is stable.
    fn render_code_pane(&mut self, frame: &mut Frame, area: Rect) {
        self.code_area = area;
        let lines = self.editor.lines().to_vec();
        let cursor = self.editor.cursor();
        let selection = self.editor.selection_range();
        let gutter = self.line_number_width();
        let highlighted = crate::tui::syntax::highlight(&lines.join("\n"), &self.language);
        let height = area.height.saturating_sub(2) as usize;

        // Keep the cursor within the visible viewport.
        if cursor.0 < self.scroll_row {
            self.scroll_row = cursor.0;
        } else if height > 0 && cursor.0 >= self.scroll_row + height {
            self.scroll_row = cursor.0 + 1 - height;
        }

        let top_row = self.scroll_row;
        let mut output: Vec<Line<'static>> = Vec::new();
        for i in 0..height {
            let row = top_row + i;
            if row >= lines.len() {
                break;
            }
            let line_num = format!("{:>width$}", row + 1, width = gutter);
            let mut spans: Vec<Span<'static>> = vec![
                Span::styled(line_num, Style::default().fg(crate::theme::FG_SUBTLE)),
                Span::raw(" "),
            ];
            if let Some(hl_line) = highlighted.get(row) {
                spans.extend(hl_line.spans.clone());
            }

            let is_selected = selection
                .map(|((r1, _), (r2, _))| row >= r1 && row < r2)
                .unwrap_or(false);
            let line_style = if is_selected {
                Style::default().bg(crate::theme::SELECTION)
            } else {
                Style::default()
            };

            output.push(Line::from(spans).style(line_style));
        }

        let block = Block::bordered()
            .border_style(Style::default().fg(self.mode.color()))
            .title(format!(" {} ", self.display_filename()));
        frame.render_widget(Paragraph::new(output).block(block), area);

        // Place the cursor within the visible viewport.
        let screen_x = area.x + 1 + gutter as u16 + 1 + cursor.1 as u16;
        let screen_y = area.y + 1 + cursor.0.saturating_sub(top_row) as u16;
        frame.set_cursor_position((screen_x, screen_y));
    }
}

impl Default for EditorScreen {
    fn default() -> Self {
        Self::new()
    }
}
