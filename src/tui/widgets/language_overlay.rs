//! Language picker overlay rendering.
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout},
    style::Style,
    widgets::{Block, Borders, Clear, List, ListItem, ListState, Paragraph},
};

use crate::models::LeetCodeLanguage;

/// Holds the state for the language picker overlay.
///
/// Lists every language returned by LeetCode and tracks the cursor position so
/// the user can scroll through them with the arrow keys / `j` / `k`.
pub struct LanguageOverlay {
    pub languages: Vec<LeetCodeLanguage>,
    pub list_state: ListState,
}

impl LanguageOverlay {
    pub fn new(languages: Vec<LeetCodeLanguage>) -> Self {
        let mut list_state = ListState::default();
        if !languages.is_empty() {
            list_state.select(Some(0));
        }
        Self {
            languages,
            list_state,
        }
    }

    pub fn cursor(&self) -> usize {
        self.list_state.selected().unwrap_or(0)
    }

    pub fn next(&mut self) {
        if self.languages.is_empty() {
            return;
        }
        let i = self.cursor();
        let next = if i >= self.languages.len() - 1 {
            0
        } else {
            i + 1
        };
        self.list_state.select(Some(next));
    }

    pub fn previous(&mut self) {
        if self.languages.is_empty() {
            return;
        }
        let i = self.cursor();
        let prev = if i == 0 {
            self.languages.len() - 1
        } else {
            i - 1
        };
        self.list_state.select(Some(prev));
    }

    pub fn scroll_down(&mut self, n: usize) {
        if self.languages.is_empty() {
            return;
        }
        let max_idx = self.languages.len() - 1;
        let next = (self.cursor() + n).min(max_idx);
        self.list_state.select(Some(next));
    }

    pub fn scroll_up(&mut self, n: usize) {
        if self.languages.is_empty() {
            return;
        }
        let prev = self.cursor().saturating_sub(n);
        self.list_state.select(Some(prev));
    }

    /// Returns the LeetCode slug of the currently highlighted language.
    pub fn selected_slug(&self) -> Option<String> {
        self.list_state
            .selected()
            .and_then(|i| self.languages.get(i))
            .map(|l| l.name.clone())
    }
}

/// Renders the language picker overlay on top of the current frame.
pub fn render_language_overlay(
    frame: &mut Frame,
    overlay: &mut LanguageOverlay,
    current_language: &str,
) {
    let overlay_area = frame
        .area()
        .centered(Constraint::Percentage(50), Constraint::Percentage(75));

    frame.render_widget(Clear, overlay_area);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(crate::theme::ACCENT))
        .title(" Language — ↑/↓: navigate  Enter: select  Esc: close ");
    let inner = block.inner(overlay_area);
    frame.render_widget(block, overlay_area);

    let layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(1), Constraint::Length(1)])
        .split(inner);

    if overlay.languages.is_empty() {
        frame.render_widget(
            Paragraph::new("No languages available")
                .style(Style::default().fg(crate::theme::FG_SUBTLE)),
            layout[0],
        );
    } else {
        let items: Vec<ListItem> = overlay
            .languages
            .iter()
            .map(|l| {
                let (prefix, color) = if l.name == current_language {
                    ("[✓] ", crate::theme::EASY)
                } else {
                    ("[  ] ", crate::theme::FG)
                };
                ListItem::new(format!("{}{}", prefix, l.name)).style(Style::default().fg(color))
            })
            .collect();

        let list = List::new(items)
            .highlight_style(
                Style::default()
                    .bg(crate::theme::SELECTION)
                    .fg(crate::theme::FG),
            )
            .highlight_symbol(">> ");

        frame.render_stateful_widget(list, layout[0], &mut overlay.list_state);
    }

    let hint = Paragraph::new(format!("Current language: {}", current_language))
        .style(Style::default().fg(crate::theme::FG_SUBTLE));
    frame.render_widget(hint, layout[1]);
}
