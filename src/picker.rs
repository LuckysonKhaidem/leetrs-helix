//! Problem picker — fetches questions, writes local files, and drives submission.
//!
//! [`Picker`] is the main orchestrator used by both the CLI commands (`pick`,
//! `submit`, `test`) and the TUI. It wraps [`LeetCodeClient`] and adds local
//! file I/O and a disk cache for the problem list.
use crate::cache::CacheService;
use crate::client::LeetCodeClient;
use crate::config::CONFIG;
use crate::error::EngineError;
use crate::format::format_result;
use crate::models::{Identifier, Language, ProblemSummary, UserDetail};
use crate::services::submission::{SubmissionResult, SubmissionService};
use std::fs;
use std::path::Path;

/// Orchestrates problem fetching, file generation, submission, and caching.
///
/// Constructed once per command invocation and shared across async tasks via
/// clone (the inner [`LeetCodeClient`] is `Clone`).
#[derive(Clone)]
pub struct Picker {
    pub client: LeetCodeClient,
}

impl Picker {
    pub fn new(client: LeetCodeClient) -> Self {
        Picker { client }
    }

    /// Fetches a problem and returns it fully in memory (no files written).
    ///
    /// This is the core of both [`Picker::pick`] (which additionally writes the
    /// files for the CLI flow) and the native TUI editor.
    pub async fn fetch_in_memory(
        &self,
        identifier: &Identifier,
        language: &Option<Language>,
    ) -> crate::error::Result<crate::models::InMemoryProblem> {
        let mut language = match language {
            Some(lang) => *lang,
            None => {
                let config = CONFIG.get().expect("Config not initialised");
                if let Some(lang) = &config.language {
                    Language::from(lang)
                } else {
                    crate::log::info("🔤 No language specified, defaulting to Python.");
                    Language::Python
                }
            }
        };

        let question = match identifier {
            Identifier::Number(num) => {
                crate::log::info(format!("🔍 Fetching problem ID: {}...", num));
                self.client.get_question_by_id(*num).await?
            }
            Identifier::String(identifier) => {
                crate::log::info(format!("🔍 Fetching problem: {}...", identifier));
                self.client.get_question_by_slug(identifier).await?
            }
        };

        // Convert LeetCode's raw HTML into wrapped terminal text.
        let formatted_content = html2md::parse_html(&question.content);
        let md_content = format!("# {}\n\n{}", question.title, formatted_content);

        let snippet = question
            .code_snippets
            .iter()
            .find(|s| s.lang_slug == language.to_lang_slug());

        let snippet = match snippet {
            Some(s) => s,
            None => {
                let snippet = question.code_snippets.first().ok_or_else(|| {
                    EngineError::Other("LeetCode problem has no code snippets".to_string())
                })?;
                language = Language::from(snippet.lang_slug.clone());
                snippet
            }
        };

        let meta = format!(
            "{} id={} slug={} lang={}",
            language.meta_comment_prefix(),
            question.question_id,
            question.title_slug,
            language.to_lang_slug()
        );

        Ok(crate::models::InMemoryProblem {
            code: format!("{}\n\n{}", meta, snippet.code),
            description: md_content,
            slug: question.title_slug.clone(),
            question_id: question.question_id.clone(),
            language,
        })
    }

    /// Resolves a problem by [`Identifier`], writes the Markdown description
    /// and language-specific code stub to disk, and returns their paths.
    ///
    /// Used by the CLI `pick` flow that opens an external editor. The native
    /// TUI editor uses [`Picker::fetch_in_memory`] instead and never touches disk.
    ///
    /// # Returns
    /// `(code_file_path, description_file_path)` on success.
    pub async fn pick(
        &self,
        identifier: &Identifier,
        language: &Option<Language>,
    ) -> crate::error::Result<(String, String)> {
        let mut resolved_language = match language {
            Some(lang) => *lang,
            None => {
                let config = CONFIG.get().expect("Config not initialised");
                if let Some(lang) = &config.language {
                    Language::from(lang)
                } else {
                    crate::log::info("🔤 No language specified, defaulting to Python.");
                    Language::Python
                }
            }
        };

        //TODO: If language is specified, must open that file
        // else open the file with matching slug.
        if let Identifier::String(ident) = identifier {
            let snake_slug = ident.replace("-", "_");
            let code_filename = format!("{}.{}", snake_slug, resolved_language.code_extension());
            let desc_filename = format!("{}.md", snake_slug);
            if Path::new(&code_filename).exists() && Path::new(&desc_filename).exists() {
                return Ok((code_filename, desc_filename));
            }
        }

        let problem = self.fetch_in_memory(identifier, language).await?;
        resolved_language = problem.language;

        let snake_slug = problem.slug.replace("-", "_");
        let code_filename = format!("{}.{}", snake_slug, resolved_language.code_extension());
        let desc_filename = format!("{}.md", snake_slug);

        if let Err(e) = fs::write(&code_filename, format!("{}\n", problem.code)) {
            crate::log::error(format!("❌ failed to write code file: {}", e));
            return Err(EngineError::System);
        }
        if let Err(e) = fs::write(&desc_filename, problem.description) {
            crate::log::error(format!("❌ failed to write description file: {}", e));
            return Err(EngineError::System);
        }
        crate::log::info("✅ files generated successfully.");

        Ok((code_filename, desc_filename))
    }

    /// Runs the solution file against the problem's built-in example test cases
    /// and prints the result, but **does not** record it as an official submission.
    pub async fn test_submit(&self, file: &str) {
        let service = SubmissionService::new(self.client.clone());
        match service.submit_or_test(file, true).await {
            Ok(result) => format_result(&result),
            Err(e) => eprintln!("❌ {}", e),
        }
    }

    /// Runs the solution against the example test cases and returns the
    /// structured result (does not print).
    pub async fn run_tests(&self, file: &str) -> crate::error::Result<SubmissionResult> {
        let service = SubmissionService::new(self.client.clone());
        service.submit_or_test(file, true).await
    }

    /// Submits the solution for full judging and returns the structured result
    /// (does not print).
    pub async fn submit_solution(&self, file: &str) -> crate::error::Result<SubmissionResult> {
        let service = SubmissionService::new(self.client.clone());
        service.submit_or_test(file, false).await
    }

    /// Runs the example test cases against an in-memory code string (no file).
    pub async fn run_tests_memory(
        &self,
        code: &str,
        slug: &str,
        language: Language,
    ) -> crate::error::Result<SubmissionResult> {
        let service = SubmissionService::new(self.client.clone());
        service
            .submit_or_test_code(code, slug, language, true)
            .await
    }

    /// Submits an in-memory code string for full judging (no file).
    pub async fn submit_solution_memory(
        &self,
        code: &str,
        slug: &str,
        language: Language,
    ) -> crate::error::Result<SubmissionResult> {
        let service = SubmissionService::new(self.client.clone());
        service
            .submit_or_test_code(code, slug, language, false)
            .await
    }

    /// Submits the solution file to LeetCode for full judging and prints the
    /// verdict, test-case counts, and performance percentiles.
    pub async fn submit(&self, file: &str) {
        let service = SubmissionService::new(self.client.clone());
        match service.submit_or_test(file, false).await {
            Ok(result) => format_result(&result),
            Err(e) => eprintln!("❌ {}", e),
        }
    }

    /// Returns the cached user profile, refreshing it in the background.
    ///
    /// **Cache-aside strategy:**
    /// 1. Read `user.json` from disk.
    /// 2. If found, return it immediately and spawn a background task that
    ///    fetches the latest data from the API and overwrites the file.
    /// 3. If not found, block on the API fetch, write the file, then return.
    pub async fn get_user_data(&self) -> crate::error::Result<UserDetail> {
        let cache = CacheService::new();
        let user_path = cache.user_path();
        let data = match fs::read_to_string(&user_path) {
            Ok(v) => {
                let client = self.client.clone();
                let user_path_bg = user_path.clone();
                tokio::spawn(async move {
                    let result: Result<(), Box<dyn std::error::Error>> = async {
                        let user_detail = client.get_user_detail().await?;
                        let data = serde_json::to_string(&user_detail)?;
                        let _ = fs::write(&user_path_bg, &data);
                        Ok(())
                    }
                    .await;

                    let _ = result;
                });
                v
            }
            Err(_) => {
                let user_detail = self.client.get_user_detail().await?;
                let data = serde_json::to_string(&user_detail)?;
                let _ = fs::write(&user_path, &data);
                data
            }
        };
        let user_detail: UserDetail = serde_json::from_str(&data).map_err(|e| {
            eprintln!("Failed to parse user details: {}", e);
            eprintln!("Try running `leetrs-helix tui` again to refresh the cache.");
            if let Err(err) = fs::remove_file(&user_path) {
                eprintln!("Failed to remove corrupted cache file: {}", err);
            }
            e
        })?;
        Ok(user_detail)
    }

    /// Returns the list of languages supported by LeetCode.
    ///
    /// Uses the same cache-aside strategy as the problem list.
    pub async fn list_languages(
        &self,
    ) -> crate::error::Result<Vec<crate::models::LeetCodeLanguage>> {
        let cache = CacheService::new();
        let path = cache.path("languages.json");
        let data = match fs::read_to_string(&path) {
            Ok(v) => {
                let client = self.client.clone();
                let path_bg = path.clone();
                tokio::spawn(async move {
                    let result: Result<(), Box<dyn std::error::Error + Send + Sync>> = async {
                        let languages = client.get_languages().await?;
                        let data = serde_json::to_string(&languages)?;
                        let _ = fs::write(&path_bg, &data);
                        Ok(())
                    }
                    .await;
                    let _ = result;
                });
                v
            }
            Err(_) => {
                let languages = self.client.get_languages().await?;
                let data = serde_json::to_string(&languages)?;
                let _ = fs::write(&path, &data);
                data
            }
        };
        let languages: Vec<crate::models::LeetCodeLanguage> =
            serde_json::from_str(&data).map_err(|e| {
                eprintln!("Failed to parse language list: {}", e);
                eprintln!("Try running `leetrs-helix tui` again to refresh the cache.");
                if let Err(err) = fs::remove_file(&path) {
                    eprintln!("Failed to remove corrupted cache file: {}", err);
                }
                e
            })?;
        Ok(languages)
    }

    /// Returns the list of company tags (with their question IDs) from LeetCode.
    ///
    /// Uses the same cache-aside strategy as the problem list.
    pub async fn list_companies(
        &self,
    ) -> crate::error::Result<Vec<crate::models::LeetCodeCompany>> {
        let cache = CacheService::new();
        let path = cache.path("companies.json");
        let data = match fs::read_to_string(&path) {
            Ok(v) => {
                let client = self.client.clone();
                let path_bg = path.clone();
                tokio::spawn(async move {
                    let result: Result<(), Box<dyn std::error::Error + Send + Sync>> = async {
                        let companies = client.get_company_tags().await?;
                        let data = serde_json::to_string(&companies)?;
                        let _ = fs::write(&path_bg, &data);
                        Ok(())
                    }
                    .await;
                    let _ = result;
                });
                v
            }
            Err(_) => {
                let companies = self.client.get_company_tags().await?;
                let data = serde_json::to_string(&companies)?;
                let _ = fs::write(&path, &data);
                data
            }
        };
        let companies: Vec<crate::models::LeetCodeCompany> =
            serde_json::from_str(&data).map_err(|e| {
                eprintln!("Failed to parse company list: {}", e);
                eprintln!("Try running `leetrs-helix tui` again to refresh the cache.");
                if let Err(err) = fs::remove_file(&path) {
                    eprintln!("Failed to remove corrupted cache file: {}", err);
                }
                e
            })?;
        Ok(companies)
    }

    /// Fetches the most recent submission for a (slug, language) from LeetCode.
    pub async fn get_last_submission(
        &self,
        slug: &str,
        lang: &str,
    ) -> crate::error::Result<Option<crate::models::LastSubmission>> {
        self.client.get_last_submission(slug, lang).await
    }

    /// Returns the full problem list, enriched with topic tags.
    ///
    /// **Cache-aside strategy:**
    /// 1. Read `data.json` from disk.
    /// 2. If found, return it immediately and spawn a background task that
    ///    fetches a fresh list (problems + tags) and overwrites the cache.
    /// 3. If not found, block on both API calls, write the cache, then return.
    pub async fn list_problems(&self) -> crate::error::Result<Vec<ProblemSummary>> {
        let cache = CacheService::new();
        let data_path = cache.problems_path();
        let user_path = cache.user_path();
        let data = match fs::read_to_string(&data_path) {
            Ok(v) => {
                // Fetch data in the background and update data.json for next time
                let client_clone = self.client.clone();
                let data_path_bg = data_path.clone();
                let user_path_bg = user_path.clone();
                tokio::spawn(async move {
                    let res: Result<(), Box<dyn std::error::Error + Send + Sync>> = async {
                        let user_detail = client_clone.get_user_detail().await?;
                        let data = serde_json::to_string(&user_detail)?;
                        let _ = fs::write(&user_path_bg, data);
                        let mut problems = client_clone.get_problem_list().await?;
                        let question_tags = client_clone.get_topics_question_list().await?;
                        for question_tag in question_tags {
                            question_tag.question_ids.iter().for_each(|question_id| {
                                if let Some(problem) =
                                    problems.iter_mut().find(|p| p.id == *question_id)
                                {
                                    problem.topics.push(question_tag.name.clone());
                                }
                            });
                        }
                        if let Ok(companies) = client_clone.get_company_tags().await {
                            for company in companies {
                                company.question_ids.iter().for_each(|question_id| {
                                    if let Some(problem) =
                                        problems.iter_mut().find(|p| p.id == *question_id)
                                    {
                                        problem.companies.push(company.name.clone());
                                        if let Some(freq) =
                                            company.frequencies.get(&question_id.to_string())
                                        {
                                            problem.frequency = problem.frequency.max(*freq);
                                        }
                                    }
                                });
                            }
                        }
                        let data = serde_json::to_string(&problems)?;
                        let _ = fs::write(&data_path_bg, data);
                        Ok(())
                    }
                    .await;

                    let _ = res;
                });
                v
            }
            Err(_) => {
                let mut problems = self.client.get_problem_list().await?;
                let question_tags = self.client.get_topics_question_list().await?;
                for question_tag in question_tags {
                    question_tag.question_ids.iter().for_each(|question_id| {
                        if let Some(problem) = problems.iter_mut().find(|p| p.id == *question_id) {
                            problem.topics.push(question_tag.name.clone());
                        }
                    });
                }
                if let Ok(companies) = self.client.get_company_tags().await {
                    for company in companies {
                        company.question_ids.iter().for_each(|question_id| {
                            if let Some(problem) =
                                problems.iter_mut().find(|p| p.id == *question_id)
                            {
                                problem.companies.push(company.name.clone());
                                if let Some(freq) =
                                    company.frequencies.get(&question_id.to_string())
                                {
                                    problem.frequency = problem.frequency.max(*freq);
                                }
                            }
                        });
                    }
                }
                let data = serde_json::to_string(&problems)?;
                let _ = fs::write(&data_path, &data);
                data
            }
        };
        let problems: Vec<ProblemSummary> = serde_json::from_str(&data).map_err(|e| {
            eprintln!("Failed to parse problem list: {}", e);
            eprintln!("Try running `leetrs-helix tui` again to refresh the cache.");
            if let Err(err) = fs::remove_file(&data_path) {
                eprintln!("Failed to remove corrupted cache file: {}", err);
            }
            e
        })?;
        Ok(problems)
    }
}
