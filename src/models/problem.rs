//! Problem summary, question detail, and topic models.
use std::collections::HashMap;

use serde::{Deserialize, Serialize};

/// A GraphQL request body sent to `https://leetcode.com/graphql`.
#[derive(Serialize, Debug)]
pub struct GraphQLQuery {
    pub query: String,
    pub variables: Option<serde_json::Value>,
    #[serde(rename = "operationName")]
    pub operation_name: Option<String>,
}

/// A single code snippet returned by LeetCode's GraphQL API for one language.
#[derive(Deserialize, Debug)]
pub struct QuestionSnippet {
    #[serde(rename = "langSlug")]
    pub lang_slug: String,
    pub code: String,
}

/// Minimal user profile returned by the `userStatus` GraphQL query.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct UserDetail {
    pub username: Option<String>,
    #[serde(rename = "isPremium")]
    pub is_premium: Option<bool>,
    #[serde(rename = "isVerified")]
    pub is_verified: bool,
}

/// A programming language supported by LeetCode's judge.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct LeetCodeLanguage {
    pub id: i64,
    pub name: String,
}

/// A fully-fetched problem held in memory, without writing files to disk.
#[derive(Debug, Clone)]
pub struct InMemoryProblem {
    /// The code snippet (with metadata header) ready for editing.
    pub code: String,
    /// The rendered Markdown problem description.
    pub description: String,
    /// LeetCode title slug, e.g. `two-sum`.
    pub slug: String,
    /// LeetCode problem ID, e.g. `1`.
    pub question_id: String,
    /// The resolved language.
    pub language: crate::models::Language,
}

/// A saved solution: the last code edited/submitted for a problem in a language.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct SavedSolution {
    /// The code that was last submitted/tested.
    pub code: String,
    /// The last result status, e.g. "Accepted", "Wrong Answer", ...
    pub last_status: String,
}

/// The last submission LeetCode has for a problem/language.
#[derive(Debug, Clone)]
pub struct LastSubmission {
    pub code: String,
    /// Whether the last submission was Accepted.
    pub accepted: bool,
}

/// A company tag and the set of problem IDs tagged under it.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct LeetCodeCompany {
    pub id: String,
    pub name: String,
    pub slug: String,
    #[serde(rename = "questionIds")]
    pub question_ids: Vec<u64>,
    /// Map of question ID -> 6-month "recency" frequency for this company.
    #[serde(default)]
    pub frequencies: HashMap<String, f64>,
}

/// Full problem details fetched from the `questionData` GraphQL query.
#[derive(Deserialize, Debug)]
pub struct Question {
    #[serde(rename = "questionId")]
    pub question_id: String,
    #[serde(rename = "titleSlug")]
    pub title_slug: String,
    pub title: String,
    pub content: String,
    #[serde(rename = "exampleTestcases")]
    pub example_test_cases: String,
    #[serde(rename = "codeSnippets")]
    pub code_snippets: Vec<QuestionSnippet>,
}

/// Lightweight problem summary used to populate the TUI problem list.
///
/// These are deserialized from the cached `data.json` file and from the
/// `/api/problems/all/` REST endpoint.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ProblemSummary {
    pub id: u64,
    pub acceptance: f64,
    pub accepted: u64,
    /// Difficulty level: `1` = Easy, `2` = Medium, `3` = Hard.
    pub difficulty: u8,
    pub slug: String,
    /// `"ac"` if solved, `"notac"` if attempted but not solved, `None` if untouched.
    pub status: Option<String>,
    pub submitted: u64,
    pub title: String,
    pub is_paid: bool,
    pub topics: Vec<String>,
    /// Company names the problem has been tagged with.
    pub companies: Vec<String>,
    /// Maximum per-company "recency" frequency across the problem's companies.
    #[serde(default)]
    pub frequency: f64,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Topic {
    pub name: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct QuestionTopics {
    pub name: String,
    pub id: String,
    pub slug: String,
    #[serde(rename = "translatedName")]
    pub translated_name: Option<String>,
    #[serde(rename = "questionIds")]
    pub question_ids: Vec<u64>,
}
