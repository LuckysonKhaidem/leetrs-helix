//! Lightweight syntax highlighter for the code pane.
//!
//! `ratatui-textarea` only supports a single global style, so to color-code the
//! code we tokenize each line and build styled [`Line`]s ourselves. The editor
//! keeps using `TextArea` for editing state; this module is only for rendering.
//!
//! The capture names and base16 color palette mirror Helix's default theme
//! (keyword = magenta, string = green, comment = dark-gray italic,
//! function = blue, type = yellow, numeric = orange, operator = cyan).
use ratatui::{
    style::{Modifier, Style},
    text::{Line, Span},
};

/// A single highlight rule set for a language.
struct LangDef<'a> {
    /// Line-comment prefixes, e.g. `//`, `#`, `--`.
    line_comments: &'a [&'a str],
    /// Block comment delimiters, e.g. `/*`, `*/` (C-like languages).
    block_comments: Option<(&'a str, &'a str)>,
    keywords: &'a [&'a str],
}

fn lang_def(lang: &str) -> LangDef<'static> {
    // Language slugs used by LeetCode (rust, python3, cpp, java, golang, ...).
    let c_like = LangDef {
        line_comments: &["//"],
        block_comments: Some(("/*", "*/")),
        keywords: &[
            "fn",
            "let",
            "mut",
            "const",
            "pub",
            "use",
            "mod",
            "impl",
            "struct",
            "enum",
            "trait",
            "if",
            "else",
            "match",
            "for",
            "while",
            "loop",
            "return",
            "break",
            "continue",
            "true",
            "false",
            "None",
            "Some",
            "Ok",
            "Err",
            "self",
            "super",
            "type",
            "static",
            "async",
            "await",
            "where",
            "ref",
            "move",
            "box",
            "as",
            "in",
            "is",
            "dyn",
            "extern",
            "unsafe",
            "class",
            "public",
            "private",
            "protected",
            "void",
            "int",
            "long",
            "double",
            "float",
            "char",
            "boolean",
            "bool",
            "string",
            "new",
            "this",
            "extends",
            "implements",
            "package",
            "import",
            "func",
            "go",
            "var",
            "range",
            "defer",
            "chan",
            "interface",
            "map",
            "nil",
            "err",
            "select",
            "case",
            "default",
            "switch",
            "fallthrough",
            "goroutine",
            "typeof",
            "console",
            "export",
            "require",
            "module",
            "function",
            "delete",
            "yield",
        ],
    };
    let python = LangDef {
        line_comments: &["#"],
        block_comments: None,
        keywords: &[
            "def", "class", "return", "if", "elif", "else", "for", "while", "break", "continue",
            "pass", "import", "from", "as", "with", "try", "except", "finally", "raise", "lambda",
            "yield", "global", "nonlocal", "in", "is", "not", "and", "or", "True", "False", "None",
            "self", "print", "len", "range", "assert", "del", "async", "await",
        ],
    };
    let sql = LangDef {
        line_comments: &["--"],
        block_comments: Some(("/*", "*/")),
        keywords: &[
            "SELECT",
            "FROM",
            "WHERE",
            "INSERT",
            "INTO",
            "VALUES",
            "UPDATE",
            "SET",
            "DELETE",
            "CREATE",
            "TABLE",
            "DROP",
            "ALTER",
            "JOIN",
            "LEFT",
            "RIGHT",
            "INNER",
            "OUTER",
            "ON",
            "GROUP",
            "BY",
            "ORDER",
            "HAVING",
            "LIMIT",
            "OFFSET",
            "AS",
            "AND",
            "OR",
            "NOT",
            "NULL",
            "PRIMARY",
            "KEY",
            "FOREIGN",
            "REFERENCES",
            "INDEX",
            "DISTINCT",
            "COUNT",
            "SUM",
            "AVG",
            "MIN",
            "MAX",
            "WHERE",
            "IN",
            "BETWEEN",
            "LIKE",
            "IS",
            "ASC",
            "DESC",
            "UNION",
            "ALL",
        ],
    };
    let shell = LangDef {
        line_comments: &["#"],
        block_comments: None,
        keywords: &[
            "echo", "if", "then", "else", "elif", "fi", "for", "do", "done", "while", "until",
            "case", "esac", "function", "return", "exit", "export", "local", "read", "set",
            "unset", "cd", "source", "shift", "in",
        ],
    };

    match lang {
        "python" | "python3" | "pythondata" | "py" => python,
        "mysql" | "mssql" | "postgresql" | "oraclesql" | "sql" => sql,
        "bash" | "sh" => shell,
        _ => c_like,
    }
}

// Helix GitHub Dark Dimmed palette.
use crate::theme;

const KW_STYLE: Style = Style::new().fg(theme::KEYWORD).add_modifier(Modifier::BOLD);
const TYPE_STYLE: Style = Style::new().fg(theme::TYPE);
const STRING_STYLE: Style = Style::new().fg(theme::STRING);
const NUMBER_STYLE: Style = Style::new().fg(theme::NUMBER);
const COMMENT_STYLE: Style = Style::new()
    .fg(theme::COMMENT)
    .add_modifier(Modifier::ITALIC);
const FUNCTION_STYLE: Style = Style::new().fg(theme::FUNCTION);
const OPERATOR_STYLE: Style = Style::new().fg(theme::OPERATOR);
const DEFAULT_STYLE: Style = Style::new().fg(theme::FG);

/// Returns whether a word looks like a type name (starts uppercase or is a
/// known primitive type).
fn is_type(word: &str) -> bool {
    word.chars()
        .next()
        .map(|c| c.is_uppercase())
        .unwrap_or(false)
        || matches!(
            word,
            "String"
                | "Vec"
                | "Option"
                | "Result"
                | "HashMap"
                | "i32"
                | "i64"
                | "u32"
                | "u64"
                | "usize"
                | "isize"
                | "f32"
                | "f64"
                | "bool"
                | "char"
                | "int"
                | "float"
                | "double"
                | "long"
                | "short"
                | "byte"
                | "void"
                | "None"
                | "Some"
        )
}

/// Returns whether a char is an operator character.
fn is_operator(c: char) -> bool {
    matches!(
        c,
        '+' | '-'
            | '*'
            | '/'
            | '%'
            | '='
            | '<'
            | '>'
            | '&'
            | '|'
            | '!'
            | '^'
            | '~'
            | '?'
            | ':'
            | '@'
    )
}

/// Highlights a single source line into a [`Line`] of styled spans.
fn highlight_line(line: &str, def: &LangDef<'_>, in_block_comment: &mut bool) -> Line<'static> {
    let mut spans: Vec<Span<'static>> = Vec::new();
    let bytes = line.as_bytes();
    let mut i = 0usize;

    while i < bytes.len() {
        let rest = &line[i..];

        // Block comment.
        if let Some((open, close)) = def.block_comments {
            if *in_block_comment {
                if let Some(pos) = rest.find(close) {
                    spans.push(Span::styled(
                        line[i..i + pos + close.len()].to_string(),
                        COMMENT_STYLE,
                    ));
                    i += pos + close.len();
                    *in_block_comment = false;
                } else {
                    spans.push(Span::styled(line[i..].to_string(), COMMENT_STYLE));
                    i = bytes.len();
                }
                continue;
            }
            if rest.starts_with(open) {
                let end = rest
                    .find(close)
                    .map(|p| i + p + close.len())
                    .unwrap_or(bytes.len());
                spans.push(Span::styled(line[i..end].to_string(), COMMENT_STYLE));
                *in_block_comment = rest.find(close).is_none();
                i = end;
                continue;
            }
        }

        // Line comment.
        if def.line_comments.iter().any(|c| rest.starts_with(c)) {
            spans.push(Span::styled(line[i..].to_string(), COMMENT_STYLE));
            break;
        }

        let c = bytes[i] as char;

        // String literals.
        if c == '"' || c == '\'' || c == '`' {
            let mut end = i + 1;
            let mut escaped = false;
            while end < bytes.len() {
                let ch = bytes[end] as char;
                if escaped {
                    escaped = false;
                } else if ch == '\\' {
                    escaped = true;
                } else if ch == c {
                    end += 1;
                    break;
                }
                end += 1;
            }
            spans.push(Span::styled(line[i..end].to_string(), STRING_STYLE));
            i = end;
            continue;
        }

        // Identifiers / keywords / types / functions.
        if c.is_ascii_alphabetic() || c == '_' {
            let start = i;
            while i < bytes.len() && (bytes[i].is_ascii_alphanumeric() || bytes[i] == b'_') {
                i += 1;
            }
            let word = &line[start..i];
            let after = &line[i..];
            let trimmed = after.trim_start();
            if def.keywords.contains(&word) {
                spans.push(Span::styled(word.to_string(), KW_STYLE));
            } else if trimmed.starts_with('(') {
                spans.push(Span::styled(word.to_string(), FUNCTION_STYLE));
            } else if is_type(word) {
                spans.push(Span::styled(word.to_string(), TYPE_STYLE));
            } else {
                spans.push(Span::styled(word.to_string(), DEFAULT_STYLE));
            }
            continue;
        }

        // Numbers.
        if c.is_ascii_digit() {
            let start = i;
            while i < bytes.len()
                && (bytes[i].is_ascii_digit() || bytes[i] == b'.' || bytes[i] == b'x')
            {
                i += 1;
            }
            spans.push(Span::styled(line[start..i].to_string(), NUMBER_STYLE));
            continue;
        }

        // Operators.
        if is_operator(c) {
            let start = i;
            while i < bytes.len() && is_operator(bytes[i] as char) {
                i += 1;
            }
            spans.push(Span::styled(line[start..i].to_string(), OPERATOR_STYLE));
            continue;
        }

        // Default char.
        spans.push(Span::styled(c.to_string(), DEFAULT_STYLE));
        i += 1;
    }

    if spans.is_empty() {
        spans.push(Span::styled(" ".to_string(), DEFAULT_STYLE));
    }
    Line::from(spans)
}

/// Highlights a whole source buffer into a vector of styled [`Line`]s.
pub fn highlight(code: &str, lang: &str) -> Vec<Line<'static>> {
    let def = lang_def(lang);
    let mut in_block = false;
    code.lines()
        .map(|l| highlight_line(l, &def, &mut in_block))
        .collect()
}
