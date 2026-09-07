//! Help screen showing all keybindings in a formatted two-column list.
use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout},
    style::Style,
    widgets::{Block, Borders, List, ListItem, Paragraph},
};

use crate::tui::{Action, screen::Screen, utils::create_split_item};
pub enum InputMode {
    Editing,
    Normal,
}

pub struct HelpScreen;

impl Screen for HelpScreen {
    fn render(&mut self, frame: &mut Frame) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .margin(1)
            .constraints([Constraint::Min(1), Constraint::Length(2)])
            .split(frame.area());

        let help_area = chunks[0];

        let items: Vec<ListItem> = vec![
            create_split_item("Global", "", crate::theme::FG, help_area.width),
            create_split_item(
                "Tab",
                "Switch between Problems and Help",
                crate::theme::ACCENT,
                help_area.width,
            ),
            create_split_item(
                "q / Esc",
                "Quit the application (from any tab)",
                crate::theme::ACCENT,
                help_area.width,
            ),
            create_split_item("", "", crate::theme::FG, help_area.width),
            create_split_item("Problems", "", crate::theme::FG, help_area.width),
            create_split_item(
                "/",
                "Start searching problems",
                crate::theme::EASY,
                help_area.width,
            ),
            create_split_item(
                "t",
                "Open topic-filter overlay (multi-select)",
                crate::theme::EASY,
                help_area.width,
            ),
            create_split_item(
                "c",
                "Open company-filter overlay (multi-select)",
                crate::theme::EASY,
                help_area.width,
            ),
            create_split_item(
                "j / k or ↓ / ↑",
                "Move selection down / up",
                crate::theme::EASY,
                help_area.width,
            ),
            create_split_item(
                "Ctrl+d / Ctrl+u",
                "Scroll down / up 10 items (list & topic filter)",
                crate::theme::EASY,
                help_area.width,
            ),
            create_split_item(
                "Enter",
                "Select the highlighted problem",
                crate::theme::EASY,
                help_area.width,
            ),
            create_split_item(
                "1 / 2 / 3 / 4",
                "Filter by difficulty (Easy / Med / Hard / All)",
                crate::theme::EASY,
                help_area.width,
            ),
            create_split_item(
                "Ctrl+j / Ctrl+k",
                "Move selection while searching",
                crate::theme::EASY,
                help_area.width,
            ),
            create_split_item(
                "l",
                "Open language picker (↑/↓ navigate, Enter select)",
                crate::theme::EASY,
                help_area.width,
            ),
            create_split_item("", "", crate::theme::FG, help_area.width),
            create_split_item("Language Picker", "", crate::theme::FG, help_area.width),
            create_split_item(
                "↑/↓ or j/k",
                "Navigate the language list",
                crate::theme::ACCENT,
                help_area.width,
            ),
            create_split_item(
                "Enter",
                "Select the highlighted language",
                crate::theme::ACCENT,
                help_area.width,
            ),
            create_split_item(
                "Esc",
                "Close without changing",
                crate::theme::ACCENT,
                help_area.width,
            ),
            create_split_item("", "", crate::theme::FG, help_area.width),
            create_split_item(
                "Topic Filter Overlay",
                "",
                crate::theme::FG,
                help_area.width,
            ),
            create_split_item(
                "Type text",
                "Filter topics in real-time",
                crate::theme::ACCENT,
                help_area.width,
            ),
            create_split_item(
                "Ctrl+j/k or ↓/↑",
                "Navigate topics while searching",
                crate::theme::ACCENT,
                help_area.width,
            ),
            create_split_item(
                "Enter / Space",
                "Toggle selected topic",
                crate::theme::ACCENT,
                help_area.width,
            ),
            create_split_item(
                "Esc",
                "Clear search query / close overlay",
                crate::theme::ACCENT,
                help_area.width,
            ),
        ];

        let help_list = List::new(items).block(
            Block::default()
                .borders(Borders::ALL)
                .title(" Help - Key Bindings ")
                .border_style(Style::default().fg(crate::theme::FG_SUBTLE)),
        );

        frame.render_widget(help_list, help_area);

        // Bottom hint bar
        let hint = Paragraph::new("Press Tab to return to Problems tab.")
            .style(Style::default().fg(crate::theme::FG_SUBTLE));
        frame.render_widget(hint, chunks[1]);
    }

    fn event_loop(&mut self, key_event: &KeyEvent) -> Option<Action> {
        match key_event.code {
            KeyCode::Char('q') | KeyCode::Esc => Some(Action::Quit),
            _ => None,
        }
    }
}

impl HelpScreen {
    pub fn new() -> Self {
        Self
    }
}

impl Default for HelpScreen {
    fn default() -> Self {
        Self
    }
}
