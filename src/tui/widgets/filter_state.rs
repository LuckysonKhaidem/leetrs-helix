//! Filter state — difficulty, topic, company, and fuzzy-search filter logic.
use std::collections::HashSet;

use fuzzy_matcher::{FuzzyMatcher, skim::SkimMatcherV2};
use ratatui::widgets::ListState;

use crossterm::event::KeyEvent;
use tui_input::{Input, backend::crossterm::EventHandler};

use crate::models::ProblemSummary;

/// Input mode for a tag-filter overlay.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum TagInputMode {
    #[default]
    Normal,
    Editing,
}

/// A filter overlay over a list of string tags (topics or companies).
///
/// Tracks the full tag list, the currently selected subset, and a search
/// filter so the user can navigate a large list with arrow keys or typing.
pub struct TagFilterState {
    pub all_tags: Vec<String>,
    pub selected_tags: HashSet<String>,
    pub list_state: ListState,
    pub search_input: Input,
    pub filtered_tags: Vec<String>,
    pub mode: TagInputMode,
}

impl TagFilterState {
    /// Builds a new tag filter from a set of tags (deduped and sorted).
    pub fn new(tags: Vec<String>) -> Self {
        let mut all_tags: HashSet<String> = tags.into_iter().collect();
        let mut all_tags: Vec<String> = all_tags.drain().collect();
        all_tags.sort();

        let mut list_state = ListState::default();
        if !all_tags.is_empty() {
            list_state.select(Some(0));
        }

        let filtered_tags = all_tags.clone();

        Self {
            all_tags,
            selected_tags: HashSet::new(),
            list_state,
            search_input: Input::default(),
            filtered_tags,
            mode: TagInputMode::Normal,
        }
    }

    pub fn cursor(&self) -> usize {
        self.list_state.selected().unwrap_or(0)
    }

    pub fn update_filter(&mut self) {
        let query = self.search_input.value().to_lowercase();
        if query.is_empty() {
            self.filtered_tags = self.all_tags.clone();
        } else {
            self.filtered_tags = self
                .all_tags
                .iter()
                .filter(|t| t.to_lowercase().contains(&query))
                .cloned()
                .collect();
        }

        if self.filtered_tags.is_empty() {
            self.list_state.select(None);
        } else {
            let curr = self.list_state.selected().unwrap_or(0);
            if curr >= self.filtered_tags.len() || self.list_state.selected().is_none() {
                self.list_state.select(Some(0));
            }
        }
    }

    pub fn handle_key(&mut self, key_event: &KeyEvent) {
        self.search_input
            .handle_event(&crossterm::event::Event::Key(*key_event));
        self.update_filter();
    }

    pub fn clear_search(&mut self) {
        self.search_input = Input::default();
        self.update_filter();
    }

    pub fn next(&mut self) {
        if self.filtered_tags.is_empty() {
            return;
        }
        let i = self.cursor();
        let next = if i >= self.filtered_tags.len() - 1 {
            0
        } else {
            i + 1
        };
        self.list_state.select(Some(next));
    }

    pub fn previous(&mut self) {
        if self.filtered_tags.is_empty() {
            return;
        }
        let i = self.cursor();
        let prev = if i == 0 {
            self.filtered_tags.len() - 1
        } else {
            i - 1
        };
        self.list_state.select(Some(prev));
    }

    pub fn scroll_down(&mut self, n: usize) {
        if self.filtered_tags.is_empty() {
            return;
        }
        let max_idx = self.filtered_tags.len() - 1;
        let next = (self.cursor() + n).min(max_idx);
        self.list_state.select(Some(next));
    }

    pub fn scroll_up(&mut self, n: usize) {
        if self.filtered_tags.is_empty() {
            return;
        }
        let prev = self.cursor().saturating_sub(n);
        self.list_state.select(Some(prev));
    }

    pub fn toggle_current(&mut self) {
        let cursor = self.cursor();
        if let Some(tag) = self.filtered_tags.get(cursor).cloned() {
            if self.selected_tags.contains(&tag) {
                self.selected_tags.remove(&tag);
            } else {
                self.selected_tags.insert(tag);
            }
        }
    }

    pub fn clear(&mut self) {
        self.selected_tags.clear();
    }
}

impl Default for TagFilterState {
    fn default() -> Self {
        Self::new(Vec::new())
    }
}

/// Encapsulates difficulty, topic, company, and search filters.
///
/// Call [`FilterState::apply`] to get the set of problem indices that pass all
/// active filters in one efficient pass.
pub struct FilterState {
    pub difficulty: Option<u8>,
    pub topics: TagFilterState,
    pub companies: TagFilterState,
}

impl FilterState {
    /// Builds a filter state whose topic and company lists are derived from the
    /// problem data, so the overlay always shows exactly the tags that exist.
    pub fn new(problems: &[ProblemSummary]) -> Self {
        let topics: Vec<String> = problems
            .iter()
            .flat_map(|p| p.topics.iter().cloned())
            .collect();
        let companies: Vec<String> = problems
            .iter()
            .flat_map(|p| p.companies.iter().cloned())
            .collect();

        Self {
            difficulty: None,
            topics: TagFilterState::new(topics),
            companies: TagFilterState::new(companies),
        }
    }

    /// Returns the sorted list of indices into `problems` that survive all
    /// active filters and the given search query.
    pub fn apply(&self, problems: &[ProblemSummary], query: &str) -> Vec<usize> {
        let has_topics = !self.topics.selected_tags.is_empty();
        let has_companies = !self.companies.selected_tags.is_empty();

        let candidates = problems.iter().enumerate().filter(|(_, p)| {
            if let Some(diff) = self.difficulty
                && p.difficulty != diff
            {
                return false;
            }
            if has_topics {
                let matches_topic = p
                    .topics
                    .iter()
                    .any(|t| self.topics.selected_tags.contains(t));
                if !matches_topic {
                    return false;
                }
            }
            if has_companies {
                let matches_company = p
                    .companies
                    .iter()
                    .any(|c| self.companies.selected_tags.contains(c));
                if !matches_company {
                    return false;
                }
            }
            true
        });

        let mut result: Vec<usize> = if query.is_empty() {
            candidates.map(|(idx, _)| idx).collect()
        } else {
            let matcher = SkimMatcherV2::default();
            let mut scored: Vec<(i64, usize)> = Vec::with_capacity(problems.len());
            for (idx, p) in candidates {
                if let Some(score) = matcher
                    .fuzzy_match(&p.title, query)
                    .or_else(|| matcher.fuzzy_match(&p.id.to_string(), query))
                {
                    scored.push((score, idx));
                }
            }
            scored.sort_unstable_by_key(|b| std::cmp::Reverse(b.0));
            scored.into_iter().map(|(_, idx)| idx).collect()
        };

        // When filtering by company, sort by "recency" (frequency) descending.
        if has_companies {
            result.sort_by(|&a, &b| {
                problems[b]
                    .frequency
                    .partial_cmp(&problems[a].frequency)
                    .unwrap_or(std::cmp::Ordering::Equal)
            });
        }

        result
    }

    pub fn set_difficulty(&mut self, difficulty: u8) {
        if difficulty > 0 && difficulty < 4 {
            self.difficulty = Some(difficulty);
        } else {
            self.difficulty = None;
        }
    }
}

impl Default for FilterState {
    fn default() -> Self {
        Self::new(&[])
    }
}
