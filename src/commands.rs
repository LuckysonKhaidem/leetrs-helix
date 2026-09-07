//! Command handlers extracted from `main.rs`.
//!
//! Each public function here corresponds to one CLI sub-command. Separating
//! them from `main.rs` keeps the binary entry-point thin (CLI parsing only)
//! and makes the handlers independently testable.
use std::{fs, io, process::Command, rc::Rc};

use clap::CommandFactory;
use clap_complete::{
    Shell,
    aot::{Bash, Fish, Zsh},
    generate,
};
use dialoguer::{Select, theme::ColorfulTheme};

use crate::{
    auth::{LeetCodeCredentials, auto_extract_flow, manual_auth_flow},
    client::LeetCodeClient,
    config::CONFIG,
    models::{Identifier, Language, ProblemSummary},
    picker::Picker,
};

/// Builds a [`Picker`] from stored credentials, returning an error if the
/// user has not authenticated yet.
pub async fn make_picker() -> std::result::Result<Picker, String> {
    let creds = LeetCodeCredentials::load()
        .ok_or_else(|| "Not authenticated. Please run `leetrs-helix auth` first.".to_string())?;
    let client =
        LeetCodeClient::new(creds).map_err(|e| format!("Failed to initialize client: {}", e))?;
    Ok(Picker::new(client))
}

/// Interactive authentication flow — prompts the user to choose a method.
pub fn handle_auth() {
    println!("🔒 LeetCode Authentication\n");

    let options = &[
        "Paste tokens manually",
        "Extract from Firefox",
        "Extract from Chrome",
    ];

    let selection = Select::with_theme(&ColorfulTheme::default())
        .with_prompt("How would you like to authenticate?")
        .default(0)
        .items(&options[..])
        .interact()
        .unwrap();

    let credentials_result = match selection {
        0 => manual_auth_flow(),
        1 => auto_extract_flow("firefox"),
        2 => auto_extract_flow("chrome"),
        _ => unreachable!(),
    };

    match credentials_result {
        Ok(creds) => match creds.save() {
            Ok(_) => println!("\n✅ Authentication successful!"),
            Err(e) => eprintln!("\n❌ Failed to save credentials: {}", e),
        },
        Err(e) => {
            eprintln!("\n❌ Authentication failed: {}", e);
            if selection != 0 {
                eprintln!(
                    "Tip: Make sure you are logged into leetcode.com on that browser, or try the manual option."
                );
            }
        }
    }
}

/// Prints current credential status to stdout.
pub fn handle_status() {
    match LeetCodeCredentials::load() {
        Some(creds) => {
            println!("✅ Currently authenticated!");
            println!("🔑 csrftoken:");
            println!("{}\n", creds.csrf_token);
            println!("🔑 LEETCODE_SESSION:");
            println!("{}", creds.session_cookie);
        }
        None => {
            eprintln!("❌ Not authenticated. No valid credentials found.");
            eprintln!("Run `leetrs-helix auth` to set up your account.");
        }
    }
}

/// Resolves and writes the problem files, then opens the configured editor
/// with the description and code template.
///
/// When `preview` is true the Markdown description is printed to stdout
/// instead of opening an editor.
pub async fn handle_pick(
    identifier: &Identifier,
    language: &Option<Language>,
    preview: bool,
) -> std::result::Result<(), String> {
    let picker = make_picker().await?;
    pick_and_open(&picker, identifier, language, preview).await
}

/// Runs the solution against example test cases without recording a submission.
pub async fn handle_test(file: &str) -> std::result::Result<(), String> {
    let picker = make_picker().await?;
    picker.test_submit(file).await;
    Ok(())
}

/// Submits the solution to LeetCode for full judging.
pub async fn handle_submit(file: &str) -> std::result::Result<(), String> {
    let picker = make_picker().await?;
    picker.submit(file).await;
    Ok(())
}

/// Writes shell completion script for `shell` to stdout.
pub fn handle_completion<C: CommandFactory>(shell: &Shell) {
    let mut cmd = C::command();
    match shell {
        Shell::Bash => generate(Bash, &mut cmd, "leetrs-helix", &mut io::stdout()),
        Shell::Zsh => generate(Zsh, &mut cmd, "leetrs-helix", &mut io::stdout()),
        Shell::Fish => generate(Fish, &mut cmd, "leetrs-helix", &mut io::stdout()),
        Shell::Elvish => todo!(),
        Shell::PowerShell => todo!(),
        _ => todo!(),
    }
}

/// Fetches the full problem list and user data, then launches the interactive TUI.
pub async fn open_tui(language: &Option<Language>) {
    let creds = match LeetCodeCredentials::load() {
        Some(c) => c,
        None => {
            eprintln!("❌ Not authenticated. Please run `leetrs-helix auth` first.");
            return;
        }
    };
    let client = match LeetCodeClient::new(creds) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("❌ Failed to initialize client: {}", e);
            return;
        }
    };
    let picker = Picker::new(client);

    let mut problems = match picker.list_problems().await {
        Ok(p) => p,
        Err(e) => {
            eprintln!("❌ Failed to fetch problems: {}", e);
            return;
        }
    };

    // Merge company tags into the in-memory problem list so the company
    // filter works even if the cache was built before companies were added.
    if let Ok(companies) = picker.list_companies().await {
        for company in companies {
            for qid in company.question_ids {
                if let Some(p) = problems.iter_mut().find(|p| p.id == qid) {
                    p.companies.push(company.name.clone());
                    if let Some(freq) = company.frequencies.get(&qid.to_string()) {
                        p.frequency = p.frequency.max(*freq);
                    }
                }
            }
        }
    }

    let problems: Rc<[ProblemSummary]> = Rc::from(problems);
    let user_data = picker.get_user_data().await.ok();
    let languages = match picker.list_languages().await {
        Ok(langs) => Some(langs),
        Err(e) => {
            eprintln!("⚠️ Failed to fetch language list: {}", e);
            None
        }
    };
    if let Err(e) =
        crate::tui::run_tui(Rc::clone(&problems), picker, user_data, language, languages).await
    {
        eprintln!("❌ TUI error: {}", e);
    }
}

/// Resolves and writes the problem files, then opens the configured editor.
///
/// When `preview` is true, the Markdown description is printed to stdout
/// instead of opening an editor.
pub async fn pick_and_open(
    picker: &Picker,
    identifier: &Identifier,
    language: &Option<Language>,
    preview: bool,
) -> std::result::Result<(), String> {
    let (code, desc) = picker
        .pick(identifier, language)
        .await
        .map_err(|e| format!("{}", e))?;

    if !preview {
        launch_editor(&code, &desc).await?;
    } else {
        let content = fs::read_to_string(desc)
            .map_err(|e| format!("Failed to read description file: {}", e))?;
        print!("{}", content);
    }
    Ok(())
}

/// Launches the configured editor for the given code and description files.
///
/// Behaviour is driven by the `editor`, `show_description`, and metadata:
/// * `nvim`/`vim`  -> vertical split: `nvim <desc> -c "vsplit <code>"`
/// * `hx`/`helix`  -> vertical split: `hx --vsplit <desc> <code>`
/// * other editors -> open both files: `<editor> <desc> <code>`
///
/// If `show_description` is disabled, only the code file is opened.
pub async fn launch_editor(code: &str, desc: &str) -> std::result::Result<(), String> {
    let config = CONFIG
        .get()
        .ok_or_else(|| "Failed to initialise config".to_string())?;
    let editor = config.editor.as_deref().unwrap_or("hx");
    let show_description = config.show_description.unwrap_or(true);

    println!("🚀 launching {}...", editor);

    let status = if show_description {
        if is_vim_style_editor(editor) {
            Command::new(editor)
                .arg(desc)
                .arg("-c")
                .arg(format!("vsplit {}", code))
                .status()
        } else if is_helix_editor(editor) {
            // Helix supports CLI splits via --vsplit / --hsplit.
            Command::new(editor)
                .arg("--vsplit")
                .arg(desc)
                .arg(code)
                .status()
        } else {
            // Generic editor: open the description first, then the code file.
            Command::new(editor).arg(desc).arg(code).status()
        }
    } else {
        Command::new(editor).arg(code).status()
    };

    match status {
        Ok(exit_status) if exit_status.success() => {
            println!("\n👋 {} closed.", editor);
            Ok(())
        }
        Ok(exit_status) => Err(format!(
            "⚠️ {} exited with an error code: {}",
            editor, exit_status
        )),
        Err(e) => Err(format!(
            "❌ failed to launch {}. is it installed and in your path? error: {}",
            editor, e
        )),
    }
}

/// Returns `true` for editors that support `-c "vsplit ..."` (Neovim/Vim).
fn is_vim_style_editor(editor: &str) -> bool {
    let lower = editor.to_lowercase();
    lower == "nvim" || lower == "vim" || lower == "vi" || lower.contains("neovim")
}

/// Returns `true` for Helix (`hx` or `helix`).
fn is_helix_editor(editor: &str) -> bool {
    let lower = editor.to_lowercase();
    lower == "hx" || lower == "helix"
}
