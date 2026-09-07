//! Language enum and associated helpers.
use clap::ValueEnum;

use crate::models::LeetCodeLanguage;

/// Supported submission languages accepted by LeetCode.
///
/// The `ValueEnum` derive lets Clap accept these directly as CLI arguments.
/// Internally this enum is kept only for CLI parsing and for inferring a
/// language from a file extension during `test`/`submit`. The TUI and `pick`
/// flows use LeetCode's live language list instead of this enum so every
/// language LeetCode supports (C++, Java, Go, TypeScript, etc.) works.
#[derive(Debug, Clone, Copy, ValueEnum, PartialEq, Eq)]
pub enum Language {
    Cpp,
    Java,
    Python,
    Python2,
    CSharp,
    JavaScript,
    TypeScript,
    C,
    Go,
    Kotlin,
    Swift,
    Rust,
    Ruby,
    PHP,
    Dart,
    Scala,
    Elixir,
    Erlang,
    Racket,
    Bash,
    Mysql,
    Pandas,
    MsSql,
    Postgres,
    OracleSql,
}

impl Language {
    /// An ordered fallback list used when LeetCode's live language list is
    /// unavailable (offline mode or failed fetch).
    pub fn fallback() -> &'static [Language] {
        &[
            Language::Rust,
            Language::Python,
            Language::Pandas,
            Language::Mysql,
            Language::Postgres,
        ]
    }

    /// Returns a human-friendly display name for the language.
    pub fn display_name(&self) -> &'static str {
        match self {
            Language::Cpp => "C++",
            Language::Java => "Java",
            Language::Python => "Python",
            Language::Python2 => "Python2",
            Language::Pandas => "Pandas",
            Language::CSharp => "C#",
            Language::JavaScript => "JavaScript",
            Language::TypeScript => "TypeScript",
            Language::C => "C",
            Language::Go => "Go",
            Language::Kotlin => "Kotlin",
            Language::Swift => "Swift",
            Language::Rust => "Rust",
            Language::Ruby => "Ruby",
            Language::PHP => "PHP",
            Language::Dart => "Dart",
            Language::Scala => "Scala",
            Language::Elixir => "Elixir",
            Language::Erlang => "Erlang",
            Language::Racket => "Racket",
            Language::Bash => "Bash",
            Language::Mysql => "MySQL",
            Language::MsSql => "MS SQL",
            Language::Postgres => "PostgreSQL",
            Language::OracleSql => "Oracle SQL",
        }
    }

    /// Maps a LeetCode language slug to one of our internal variants.
    pub fn from_leetcode_slug(slug: &str) -> Option<Self> {
        match slug {
            "cpp" => Some(Self::Cpp),
            "java" => Some(Self::Java),
            "python" => Some(Self::Python),
            "python3" => Some(Self::Python),
            "pythondata" => Some(Self::Pandas),
            "csharp" => Some(Self::CSharp),
            "javascript" => Some(Self::JavaScript),
            "typescript" => Some(Self::TypeScript),
            "c" => Some(Self::C),
            "golang" => Some(Self::Go),
            "kotlin" => Some(Self::Kotlin),
            "swift" => Some(Self::Swift),
            "rust" => Some(Self::Rust),
            "ruby" => Some(Self::Ruby),
            "php" => Some(Self::PHP),
            "dart" => Some(Self::Dart),
            "scala" => Some(Self::Scala),
            "elixir" => Some(Self::Elixir),
            "erlang" => Some(Self::Erlang),
            "racket" => Some(Self::Racket),
            "bash" => Some(Self::Bash),
            "mysql" => Some(Self::Mysql),
            "mssql" => Some(Self::MsSql),
            "postgresql" => Some(Self::Postgres),
            "oraclesql" => Some(Self::OracleSql),
            _ => None,
        }
    }

    /// Maps a [`Language`] variant to LeetCode's internal language slug string.
    pub fn to_lang_slug(&self) -> &'static str {
        match self {
            Language::Cpp => "cpp",
            Language::Java => "java",
            Language::Python | Language::Python2 => "python3",
            Language::Pandas => "pythondata",
            Language::CSharp => "csharp",
            Language::JavaScript => "javascript",
            Language::TypeScript => "typescript",
            Language::C => "c",
            Language::Go => "golang",
            Language::Kotlin => "kotlin",
            Language::Swift => "swift",
            Language::Rust => "rust",
            Language::Ruby => "ruby",
            Language::PHP => "php",
            Language::Dart => "dart",
            Language::Scala => "scala",
            Language::Elixir => "elixir",
            Language::Erlang => "erlang",
            Language::Racket => "racket",
            Language::Bash => "bash",
            Language::Mysql => "mysql",
            Language::MsSql => "mssql",
            Language::Postgres => "postgresql",
            Language::OracleSql => "oraclesql",
        }
    }

    /// Infers the language from a file extension. Falls back to C++ for unknown extensions.
    pub fn from_extension(ext: &str) -> Self {
        match ext {
            "py" => Language::Python,
            "rs" => Language::Rust,
            "cpp" | "cc" | "cxx" | "c++" => Language::Cpp,
            "java" => Language::Java,
            "cs" => Language::CSharp,
            "js" => Language::JavaScript,
            "ts" => Language::TypeScript,
            "c" => Language::C,
            "go" => Language::Go,
            "kt" => Language::Kotlin,
            "swift" => Language::Swift,
            "rb" => Language::Ruby,
            "php" => Language::PHP,
            "dart" => Language::Dart,
            "scala" | "sc" => Language::Scala,
            "ex" | "exs" => Language::Elixir,
            "erl" | "hrl" => Language::Erlang,
            "rkt" => Language::Racket,
            "sh" | "bash" => Language::Bash,
            "sql" => Language::Mysql,
            _ => Language::Cpp,
        }
    }

    /// Returns the file extension used for solution files in this language.
    pub fn code_extension(&self) -> &'static str {
        match self {
            Language::Cpp => "cpp",
            Language::Java => "java",
            Language::Python | Language::Pandas | Language::Python2 => "py",
            Language::CSharp => "cs",
            Language::JavaScript => "js",
            Language::TypeScript => "ts",
            Language::C => "c",
            Language::Go => "go",
            Language::Kotlin => "kt",
            Language::Swift => "swift",
            Language::Rust => "rs",
            Language::Ruby => "rb",
            Language::PHP => "php",
            Language::Dart => "dart",
            Language::Scala => "scala",
            Language::Elixir => "ex",
            Language::Erlang => "erl",
            Language::Racket => "rkt",
            Language::Bash => "sh",
            Language::Mysql | Language::MsSql | Language::Postgres | Language::OracleSql => "sql",
        }
    }

    /// Returns the single-line comment prefix used in this language.
    pub fn meta_comment_prefix(&self) -> &'static str {
        match self {
            Language::Python
            | Language::Pandas
            | Language::Python2
            | Language::Bash
            | Language::Mysql
            | Language::MsSql
            | Language::OracleSql => "#",
            Language::C
            | Language::Cpp
            | Language::Rust
            | Language::Java
            | Language::CSharp
            | Language::JavaScript
            | Language::TypeScript
            | Language::Go
            | Language::Kotlin
            | Language::Swift
            | Language::Dart
            | Language::Scala
            | Language::Ruby
            | Language::PHP => "//",
            Language::Elixir | Language::Erlang | Language::Racket => "%",
            Language::Postgres => "--",
        }
    }
}

impl From<&String> for Language {
    fn from(value: &String) -> Self {
        Language::from_leetcode_slug(value).unwrap_or(Language::Cpp)
    }
}

impl From<String> for Language {
    fn from(value: String) -> Self {
        Language::from_leetcode_slug(&value).unwrap_or(Language::Cpp)
    }
}

impl From<LeetCodeLanguage> for Language {
    fn from(lang: LeetCodeLanguage) -> Self {
        Language::from(&lang)
    }
}

impl From<&LeetCodeLanguage> for Language {
    fn from(lang: &LeetCodeLanguage) -> Self {
        Language::from_leetcode_slug(&lang.name).unwrap_or(Self::Python)
    }
}

/// Problem-file metadata parsed from the metadata header line.
impl Language {
    /// Returns the display name for use in UI status strings.
    pub fn short_name(&self) -> &'static str {
        match self {
            Language::Python | Language::Python2 => "Python",
            Language::Postgres => "Postgres",
            Language::OracleSql => "Oracle SQL",
            Language::MsSql => "MS SQL",
            Language::Bash => "Bash",
            l => l.display_name(),
        }
    }
}

/// A problem identifier supplied on the command line — either a numeric ID or a slug.
#[derive(Debug, Clone)]
pub enum Identifier {
    Number(u64),
    String(String),
}
