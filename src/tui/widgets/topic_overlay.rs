//! Tag filter overlay rendering (shared by topics and companies).
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout},
    style::Style,
    widgets::{Block, Borders, Clear, List, ListItem, Paragraph},
};

use crate::tui::widgets::filter_state::{TagFilterState, TagInputMode};

/// Renders a tag filter overlay (topics or companies) on top of the frame.
pub fn render_tag_overlay(
    frame: &mut Frame,
    tag_filter: &mut TagFilterState,
    filtered_count: usize,
    title: &str,
) {
    let overlay_area = frame
        .area()
        .centered(Constraint::Percentage(70), Constraint::Percentage(80));

    frame.render_widget(Clear, overlay_area);

    let (search_title, search_style, border_color) = match tag_filter.mode {
        TagInputMode::Editing => (
            format!(
                " Search ({}) — Press Esc to finish search ",
                tag_filter.filtered_tags.len()
            ),
            Style::default().fg(crate::theme::ACCENT),
            crate::theme::ACCENT,
        ),
        TagInputMode::Normal => (
            format!(
                " Search ({}) — Press / to search ",
                tag_filter.filtered_tags.len()
            ),
            Style::default().fg(crate::theme::FG_SUBTLE),
            crate::theme::ACCENT,
        ),
    };

    let selected_count = tag_filter.selected_tags.len();
    let title = if selected_count == 0 {
        format!(
            " {} — j/k: navigate  /: search  Space/Enter: toggle  c: clear  Esc: close ",
            title
        )
    } else {
        format!(
            " {} ({} selected) — j/k: navigate  /: search  Space/Enter: toggle  c: clear  Esc: close ",
            title, selected_count
        )
    };

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(border_color))
        .title(title.as_str());

    let inner = block.inner(overlay_area);
    frame.render_widget(block, overlay_area);

    let layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(1),
            Constraint::Length(1),
        ])
        .split(inner);

    let search_widget = Paragraph::new(tag_filter.search_input.value())
        .style(search_style)
        .block(Block::default().borders(Borders::ALL).title(search_title));
    frame.render_widget(search_widget, layout[0]);

    if let TagInputMode::Editing = tag_filter.mode {
        frame.set_cursor_position((
            layout[0].x + tag_filter.search_input.visual_cursor() as u16 + 1,
            layout[0].y + 1,
        ));
    }

    if tag_filter.all_tags.is_empty() {
        frame.render_widget(
            Paragraph::new("No tags available — ensure the problem list is fully loaded.")
                .style(Style::default().fg(crate::theme::FG_SUBTLE)),
            layout[1],
        );
    } else if tag_filter.filtered_tags.is_empty() {
        frame.render_widget(
            Paragraph::new(format!(
                "No tags match \"{}\"",
                tag_filter.search_input.value()
            ))
            .style(Style::default().fg(crate::theme::FG_SUBTLE)),
            layout[1],
        );
    } else {
        let items: Vec<ListItem> = tag_filter
            .filtered_tags
            .iter()
            .map(|t| {
                let (prefix, color) = if tag_filter.selected_tags.contains(t) {
                    ("[x] ", crate::theme::EASY)
                } else {
                    ("[ ] ", crate::theme::FG)
                };
                ListItem::new(format!("{}{}", prefix, t)).style(Style::default().fg(color))
            })
            .collect();

        let list = List::new(items)
            .highlight_style(
                Style::default()
                    .bg(crate::theme::SELECTION)
                    .fg(crate::theme::FG),
            )
            .highlight_symbol(">> ");

        frame.render_stateful_widget(list, layout[1], &mut tag_filter.list_state);
    }

    let hint = if filtered_count == 0 {
        Paragraph::new("No problems match current filters")
            .style(Style::default().fg(crate::theme::HARD))
    } else {
        Paragraph::new(format!("{} problems match", filtered_count))
            .style(Style::default().fg(crate::theme::FG_SUBTLE))
    };
    frame.render_widget(hint, layout[2]);
}
