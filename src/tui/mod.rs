//! ratatui TUI runtime for leetrs.
//!
//! Provides the interactive problem browser. The top-level entry point is
//! [`run_tui`], which owns the terminal setup/teardown and re-opens the editor
//! after it closes so the user can pick another problem without restarting.
pub mod renderers;
pub mod screen;
mod syntax;
mod utils;
pub mod widgets;

use crate::config::CONFIG;
use crate::models::{Identifier, Language, ProblemSummary, UserDetail};
use crate::picker::Picker;
use crate::tui::screen::{
    editor_screen::EditorScreen, help_screen::HelpScreen, selection_screen::SelectionScreen,
};
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEventKind},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::layout::{Constraint, Layout};
use ratatui::widgets::{Block, Clear, Paragraph};
use ratatui::{
    Terminal,
    backend::{Backend, CrosstermBackend},
    widgets::ListState,
};
use screen::Screen;
use std::{io, rc::Rc};

/// Which tab is currently displayed.
#[derive(Default, PartialEq, Eq)]
pub enum Tab {
    #[default]
    Selection,
    Help,
    Editor,
}

/// Holds the state of the application
/// Top-level application state shared across a single TUI session.
pub struct App {
    pub should_quit: bool,
    /// The full, shared problem list (reference-counted to avoid copying).
    pub problems: Rc<[ProblemSummary]>,
    pub tab: Tab,
    pub selection_screen: SelectionScreen,
    pub help_screen: HelpScreen,
    pub editor_screen: EditorScreen,
    /// Slug of the problem the user pressed Enter on, if any.
    pub selected_problem: Option<String>,
    pub user_detail: Option<UserDetail>,
    /// One-shot message shown in a modal popup until dismissed.
    pub popup_message: Option<String>,
    /// Language currently selected for code-stub generation (LeetCode slug).
    pub language: String,
    /// Languages returned by LeetCode, ordered by LeetCode's sort key.
    pub languages: Vec<crate::models::LeetCodeLanguage>,
    /// The picker used to load problems and run submissions.
    pub picker: Picker,
    /// Slugs of the problems in the filtered list when the editor was opened.
    pub nav_slugs: Vec<String>,
    /// Index of the currently open problem within `nav_slugs`.
    pub nav_index: usize,
}

/// Actions that a [`Screen`] can return to the main event loop.
pub enum Action {
    Quit,
    /// The user selected a problem; carries its slug.
    Select(String),
    /// Display a one-shot modal popup with the given message.
    ShowMessage(String),
    /// Dismiss the currently active popup.
    DismissPopup,
    /// Open the given URL in the system browser.
    Open(String),
    /// Set the active solution language to the given LeetCode slug.
    SetLanguage(String),
    /// Run the example test cases against the current code.
    RunTests,
    /// Submit the current code for full judging.
    Submit,
    /// Return to the problem-selection screen.
    BackToSelection,
    /// Load the next problem in the filtered list.
    NextProblem,
    /// Load the previous problem in the filtered list.
    PrevProblem,
}

impl App {
    pub fn new(
        problems: Rc<[ProblemSummary]>,
        user_detail: Option<UserDetail>,
        language: String,
        languages: Vec<crate::models::LeetCodeLanguage>,
        picker: Picker,
    ) -> Self {
        let mut list_state = ListState::default();
        if !problems.is_empty() {
            list_state.select(Some(0)); // Start by highlighting the first item
        }

        Self {
            should_quit: false,
            selection_screen: SelectionScreen::new(
                Rc::clone(&problems),
                user_detail.clone(),
                language.clone(),
                languages.clone(),
            ),
            editor_screen: EditorScreen::new(),
            problems,
            tab: Tab::default(),
            selected_problem: None,
            help_screen: HelpScreen::new(),
            user_detail,
            popup_message: None,
            language,
            languages,
            picker,
            nav_slugs: Vec::new(),
            nav_index: 0,
        }
    }

    pub fn switch(&mut self) {
        self.tab = match self.tab {
            Tab::Help => Tab::Selection,
            _ => Tab::Help,
        }
    }

    /// Sets the active language slug and updates the selection screen.
    pub fn set_language(&mut self, slug: &str) {
        self.language = slug.to_string();
        self.selection_screen.set_language(slug);
    }

    /// Handles an [`Action`] returned by the active screen.
    async fn handle_action(&mut self, action: Action) {
        match action {
            Action::Quit => self.should_quit = true,
            Action::Select(problem) => {
                // Record the filtered problem list so the editor can navigate
                // between problems.
                let slugs: Vec<String> = self
                    .selection_screen
                    .filtered_problems
                    .iter()
                    .map(|&i| self.selection_screen.all_problems[i].slug.clone())
                    .collect();
                self.nav_slugs = slugs;
                self.nav_index = self
                    .nav_slugs
                    .iter()
                    .position(|s| *s == problem)
                    .unwrap_or(0);
                self.load_problem(&problem).await;
            }
            Action::ShowMessage(msg) => self.popup_message = Some(msg),
            Action::DismissPopup => self.popup_message = None,
            Action::Open(url) => {
                let _ = open::that(url);
            }
            Action::SetLanguage(slug) => {
                self.set_language(&slug);
                self.popup_message = Some(format!("Language: {}", slug));
            }
            Action::RunTests => self.editor_screen.start_tests(&self.picker),
            Action::Submit => self.editor_screen.start_submit(&self.picker),
            Action::BackToSelection => self.tab = Tab::Selection,
            Action::NextProblem => {
                if !self.nav_slugs.is_empty() {
                    self.nav_index = (self.nav_index + 1) % self.nav_slugs.len();
                    let slug = self.nav_slugs[self.nav_index].clone();
                    self.load_problem(&slug).await;
                }
            }
            Action::PrevProblem => {
                if !self.nav_slugs.is_empty() {
                    self.nav_index =
                        (self.nav_index + self.nav_slugs.len() - 1) % self.nav_slugs.len();
                    let slug = self.nav_slugs[self.nav_index].clone();
                    self.load_problem(&slug).await;
                }
            }
        }
    }

    /// Loads the given problem slug into the editor screen.
    async fn load_problem(&mut self, problem: &str) {
        let lang = Language::from_leetcode_slug(&self.language).or_else(|| {
            CONFIG
                .get()
                .and_then(|c| c.language.as_ref().map(Language::from))
        });
        match self
            .editor_screen
            .load(
                &self.picker,
                &Identifier::String(problem.to_string()),
                &lang,
            )
            .await
        {
            Ok(()) => self.tab = Tab::Editor,
            Err(e) => self.popup_message = Some(e),
        }
    }
}

/// RAII guard for the terminal alternate screen / raw mode.
///
/// Entering the terminal and restoring it are always done as a pair. This
/// guard ensures the cleanup code runs even if the TUI panics mid-session,
/// preventing the user's shell from being left in raw mode.
struct TerminalGuard {
    terminal: Terminal<CrosstermBackend<io::Stdout>>,
}

impl TerminalGuard {
    fn enter() -> anyhow::Result<Self> {
        enable_raw_mode().map_err(anyhow::Error::from)?;
        let mut stdout = io::stdout();
        execute!(stdout, EnterAlternateScreen).map_err(anyhow::Error::from)?;
        execute!(stdout, EnableMouseCapture).map_err(anyhow::Error::from)?;
        let backend = CrosstermBackend::new(stdout);
        let mut terminal = Terminal::new(backend)?;
        terminal.clear()?;
        Ok(Self { terminal })
    }

    fn terminal_mut(&mut self) -> &mut Terminal<CrosstermBackend<io::Stdout>> {
        &mut self.terminal
    }
}

impl Drop for TerminalGuard {
    fn drop(&mut self) {
        // Best-effort restoration — ignore errors during panic unwinding.
        let _ = disable_raw_mode();
        let _ = execute!(self.terminal.backend_mut(), LeaveAlternateScreen);
        let _ = execute!(self.terminal.backend_mut(), DisableMouseCapture);
        let _ = self.terminal.show_cursor();
    }
}

/// The main entry point for the TUI.
///
/// Initialises [`App`], then enters the TUI event loop. Selecting a problem
/// loads it into the native editor tab instead of launching an external editor.
pub async fn run_tui(
    problems: Rc<[ProblemSummary]>,
    picker: Picker,
    user_detail: Option<UserDetail>,
    language: &Option<Language>,
    languages: Option<Vec<crate::models::LeetCodeLanguage>>,
) -> anyhow::Result<()> {
    let initial_language = (*language)
        .map(|l| l.to_lang_slug().to_string())
        .unwrap_or_else(|| {
            CONFIG
                .get()
                .and_then(|c| c.language.clone())
                .unwrap_or_else(|| "python3".to_string())
        });
    let languages = languages.unwrap_or_else(|| {
        Language::fallback()
            .iter()
            .map(|&l| crate::models::LeetCodeLanguage {
                id: 0,
                name: l.to_lang_slug().to_string(),
            })
            .collect()
    });
    let mut app = App::new(problems, user_detail, initial_language, languages, picker);
    // Suppress stdout/stderr logging while the TUI owns the alternate screen,
    // so progress messages don't corrupt the rendered frame.
    crate::log::set_quiet(true);
    let mut guard = TerminalGuard::enter()?;
    let result = run_app(guard.terminal_mut(), &mut app).await;
    // Guard drops here, restoring the terminal
    drop(guard);
    crate::log::set_quiet(false);
    match result {
        Ok(_) => Ok(()),
        Err(e) => Err(anyhow::Error::from(e)),
    }
}

/// The Event Loop
/// Drives rendering and keyboard events for a single TUI session.
///
/// Returns `Ok(None)` when the user quits, and `Err` on I/O failure.
async fn run_app<B: Backend>(
    terminal: &mut Terminal<B>,
    app: &mut App,
) -> io::Result<Option<String>> {
    loop {
        // Poll any background submission tasks so the results pane updates.
        app.editor_screen.tick();

        {
            let screen: &mut dyn Screen = match app.tab {
                Tab::Selection => &mut app.selection_screen,
                Tab::Help => &mut app.help_screen,
                Tab::Editor => &mut app.editor_screen,
            };

            let _ = terminal.draw(|f| {
                // Fill the whole screen with the theme background first.
                f.render_widget(
                    ratatui::widgets::Paragraph::new("")
                        .style(ratatui::style::Style::default().bg(crate::theme::BG)),
                    f.area(),
                );
                screen.render(f);
                if let Some(popup_message) = &app.popup_message {
                    let centered_area = f
                        .area()
                        .centered(Constraint::Percentage(60), Constraint::Percentage(20));
                    f.render_widget(Clear, centered_area);
                    let layout = Layout::default()
                        .direction(ratatui::layout::Direction::Vertical)
                        .constraints(
                            [Constraint::Percentage(80), Constraint::Percentage(20)].as_ref(),
                        )
                        .split(centered_area);
                    let popup_block = Block::bordered()
                        .border_style(ratatui::style::Style::default().fg(crate::theme::BORDER))
                        .title("Alert");
                    let paragraph = Paragraph::new(popup_message.as_str())
                        .block(popup_block)
                        .style(
                            ratatui::style::Style::default()
                                .fg(crate::theme::FG)
                                .bg(crate::theme::BG_OVERLAY),
                        );
                    f.render_widget(paragraph, layout[0]);
                    let hint = Paragraph::new("Press Enter or Esc to close")
                        .style(ratatui::style::Style::default().fg(crate::theme::FG_MUTED));
                    f.render_widget(hint, layout[1]);
                }
            });
        }

        // Poll for events (non-blocking)
        if event::poll(std::time::Duration::from_millis(50))? {
            let event = event::read()?;
            match event {
                Event::Mouse(mouse) => {
                    if app.popup_message.is_none() {
                        let screen: &mut dyn Screen = match app.tab {
                            Tab::Selection => &mut app.selection_screen,
                            Tab::Help => &mut app.help_screen,
                            Tab::Editor => &mut app.editor_screen,
                        };
                        screen.handle_mouse(&mouse);
                    }
                }
                Event::Key(key) if key.kind == KeyEventKind::Press => {
                    if app.popup_message.is_some() {
                        match key.code {
                            KeyCode::Enter | KeyCode::Esc => {
                                app.popup_message = None;
                            }
                            _ => {}
                        }
                        continue;
                    }

                    let action = match key.code {
                        // In the editor, Tab is passed to the textarea (indent)
                        // unless Ctrl is held, which toggles code/button focus.
                        KeyCode::Tab if app.tab == Tab::Editor => {
                            let screen: &mut dyn Screen = match app.tab {
                                Tab::Editor => &mut app.editor_screen,
                                _ => unreachable!(),
                            };
                            screen.event_loop(&key)
                        }
                        KeyCode::Tab => {
                            app.switch();
                            None
                        }
                        KeyCode::Char('?') => {
                            app.tab = Tab::Help;
                            None
                        }
                        _ => {
                            let screen: &mut dyn Screen = match app.tab {
                                Tab::Selection => &mut app.selection_screen,
                                Tab::Help => &mut app.help_screen,
                                Tab::Editor => &mut app.editor_screen,
                            };
                            screen.event_loop(&key)
                        }
                    };

                    if let Some(action) = action {
                        app.handle_action(action).await;
                    }
                }
                _ => {}
            }
        }

        if app.should_quit {
            return Ok(None);
        }
    }
}
