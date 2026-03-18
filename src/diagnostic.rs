use serde::Serialize;

/// Rule identifier, e.g. "XR001"
pub type RuleId = &'static str;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Severity {
    Hint,
    Warning,
    Error,
}

impl std::fmt::Display for Severity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Severity::Hint => write!(f, "hint"),
            Severity::Warning => write!(f, "warning"),
            Severity::Error => write!(f, "error"),
        }
    }
}

/// A single diagnostic emitted for a source span.
#[derive(Debug, Clone, Serialize)]
pub struct Diagnostic {
    pub rule_id: &'static str,
    pub severity: Severity,
    pub file: String,
    /// 1-based line number
    pub line: usize,
    /// 1-based column
    pub column: usize,
    pub message: String,
    pub suggestion: Option<String>,
    /// Concrete, copy-paste-ready code fix (auto-fix eligible rules only).
    /// e.g. `chunks="auto"` or `np.sqrt(arr)`.  Omitted from JSON when None.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fix_hint: Option<String>,
    pub url: Option<&'static str>,
    /// For Jupyter notebook cells the file path is a display label like
    /// `notebook.ipynb:cell[3]` that cannot be read from disk.  When set,
    /// the renderer uses this source text instead of reading `file`.
    #[serde(skip)]
    pub source_override: Option<String>,
}

impl Diagnostic {
    pub fn new(
        rule_id: &'static str,
        severity: Severity,
        file: impl Into<String>,
        line: usize,
        column: usize,
        message: impl Into<String>,
    ) -> Self {
        Self {
            rule_id,
            severity,
            file: file.into(),
            line,
            column,
            message: message.into(),
            suggestion: None,
            fix_hint: None,
            url: None,
            source_override: None,
        }
    }

    pub fn with_suggestion(mut self, s: impl Into<String>) -> Self {
        self.suggestion = Some(s.into());
        self
    }

    pub fn with_fix_hint(mut self, hint: impl Into<String>) -> Self {
        self.fix_hint = Some(hint.into());
        self
    }

    pub fn with_url(mut self, url: &'static str) -> Self {
        self.url = Some(url);
        self
    }
}

/// Static metadata about a rule — used for --list-rules and xray explain
pub struct RuleMeta {
    pub id: RuleId,
    pub name: &'static str,
    pub severity: Severity,
    pub description: &'static str,
}

/// All results for a single file
#[derive(Default)]
pub struct FileResults {
    pub diagnostics: Vec<Diagnostic>,
}

impl FileResults {
    pub fn push(&mut self, d: Diagnostic) {
        self.diagnostics.push(d);
    }
}

/// Aggregated results across the whole run
#[derive(Default)]
pub struct RunResults {
    pub files: Vec<FileResults>,
    /// Paths that were linted (parallel to `files`)
    pub paths: Vec<String>,
}

impl RunResults {
    pub fn has_errors(&self) -> bool {
        self.files
            .iter()
            .any(|f| f.diagnostics.iter().any(|d| d.severity == Severity::Error))
    }

    pub fn all_diagnostics(&self) -> impl Iterator<Item = &Diagnostic> {
        self.files.iter().flat_map(|f| f.diagnostics.iter())
    }

    pub fn total(&self) -> usize {
        self.files.iter().map(|f| f.diagnostics.len()).sum()
    }
}
