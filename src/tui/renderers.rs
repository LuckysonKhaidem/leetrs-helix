//! Shared rendering utilities for the problem-selection TUI.
use ratatui::{
    style::Style,
    text::Span,
    widgets::{Cell, Row},
};

use crate::models::ProblemSummary;

/// Converts a [`ProblemSummary`] into a [`Row`] for the problems table.
///
/// Extracted from the inline lambda in `SelectionScreen::render()` so it can
/// be unit-tested and reused across views.
pub fn render_problem_row(p: &ProblemSummary) -> Row<'static> {
    let diff_color = match p.difficulty {
        1 => crate::theme::EASY,
        2 => crate::theme::MEDIUM,
        _ => crate::theme::HARD,
    };

    let id_cell = Cell::from(Span::styled(
        format!("[{}]", p.id),
        Style::default().fg(diff_color),
    ));
    let name_cell = Cell::from(Span::styled(
        p.title.clone(),
        Style::default().fg(diff_color),
    ));
    let acceptance_text = format!("{:.1}%", p.acceptance * 100.0);
    let acceptance_cell = Cell::from(acceptance_text);

    let done_text = if let Some(status) = &p.status {
        match status.as_str() {
            "ac" => "\u{f00c}",
            "notac" => "\u{eabc}",
            _ => "",
        }
    } else {
        ""
    };
    let done_cell = match done_text {
        "\u{f00c}" => Cell::from(done_text).style(Style::default().fg(crate::theme::EASY)),
        _ => Cell::from(done_text).style(Style::default().fg(crate::theme::FG)),
    };

    let premium_text = if p.is_paid { "󰌾" } else { "" };
    let premium_cell = Cell::from(premium_text).style(Style::default().fg(crate::theme::HARD));

    let topics_text = p
        .topics
        .first()
        .map(|t| t.as_str())
        .unwrap_or("")
        .to_string();
    let topics_cell = Cell::from(topics_text);

    Row::new(vec![
        id_cell,
        name_cell,
        acceptance_cell,
        topics_cell,
        premium_cell,
        done_cell,
    ])
}
