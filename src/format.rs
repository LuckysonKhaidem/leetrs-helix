//! CLI result formatting for submission and test results.
//!
//! Keeps all `println!` / `eprintln!` calls out of business logic so that
//! [`SubmissionResult`] can later be displayed in the TUI as well.
use crate::services::submission::{SubmissionResult, SubmissionStatus};

/// Prints a human-readable summary of a submission or test result to stdout.
pub fn format_result(result: &SubmissionResult) {
    print!("{}", result_to_string(result));
}

/// Builds a human-readable summary of a submission or test result as a String.
///
/// Used by the CLI [`format_result`] and by the TUI results pane.
pub fn result_to_string(result: &SubmissionResult) -> String {
    let mut out = String::new();
    out.push_str("\n────────────────────────────────────────────────────\n");

    if result.is_test {
        let passed = result.correct_answer.unwrap_or(false);
        if passed {
            out.push_str("  ✅ All test cases passed\n");
        } else {
            out.push_str("  ❌ Test Failed\n");
        }
    } else {
        match &result.status {
            SubmissionStatus::Accepted => out.push_str("  ✅ Accepted\n"),
            SubmissionStatus::WrongAnswer => out.push_str("  ❌ Wrong Answer\n"),
            SubmissionStatus::CompileError => out.push_str("  ❌ Compile Error\n"),
            SubmissionStatus::RuntimeError => out.push_str("  ❌ Runtime Error\n"),
            SubmissionStatus::TimeLimitExceeded => out.push_str("  ❌ Time Limit Exceeded\n"),
            SubmissionStatus::Unknown(msg) => out.push_str(&format!("  ❌ {}\n", msg)),
        }
    }
    out.push_str("────────────────────────────────────────────────────\n");

    if let (Some(correct), Some(total)) = (result.total_correct, result.total_testcases) {
        out.push_str(&format!("🧪 Testcases: {} / {} passed\n", correct, total));
    }
    if let Some(runtime) = &result.runtime {
        out.push_str(&format!("⏱️ Runtime: {}\n", runtime));
    }
    if let Some(memory) = &result.memory {
        out.push_str(&format!("💾 Memory: {}\n", memory));
    }
    if let Some(mp) = result.memory_percentile {
        out.push_str(&format!("📝 Memory Percentile: {:.2}%\n", mp));
    }
    if let Some(rp) = result.runtime_percentile {
        out.push_str(&format!("⏰ Runtime Percentile: {:.2}%\n", rp));
    }

    // Error-specific detail.
    match &result.status {
        SubmissionStatus::CompileError => {
            if let Some(err) = &result.compile_error {
                out.push_str("\n💥 Compiler Output:\n");
                for line in err.lines() {
                    out.push_str(&format!("  {}\n", line));
                }
            }
        }
        SubmissionStatus::RuntimeError => {
            if let Some(err) = &result.full_runtime_error {
                out.push_str("\n❌ Error:\n");
                for line in err.lines() {
                    out.push_str(&format!("  {}\n", line));
                }
            }
        }
        SubmissionStatus::TimeLimitExceeded => {
            out.push_str("\n⏱️ Your solution exceeded the time limit.\n");
        }
        SubmissionStatus::Unknown(msg) => {
            out.push_str(&format!("\n{msg}\n"));
        }
        _ => {}
    }

    // stdin / stdout / expected, shown whenever available.
    if let Some(input) = result.input.clone() {
        out.push_str("\n📥 stdin\n");
        for line in input.split('\n') {
            out.push_str(&format!("  {}\n", line));
        }
    }

    let expected = result
        .expected_output
        .clone()
        .or_else(|| result.expected_code_answer.as_ref().map(|v| v.join("\n")));
    if let Some(expected) = expected {
        out.push_str("\n✅ expected\n");
        for line in expected.split('\n') {
            out.push_str(&format!("  {}\n", line));
        }
    }

    let stdout = result
        .code_output
        .clone()
        .or_else(|| result.code_answer.as_ref().map(|v| v.join("\n")));
    if let Some(stdout) = stdout {
        out.push_str("\n📤 stdout\n");
        for line in stdout.split('\n') {
            out.push_str(&format!("  {}\n", line));
        }
    }

    out
}
