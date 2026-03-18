//! `.xrayignore` — gitignore-style path exclusion for xray.
//!
//! Rules:
//!   - Lines starting with `#` are comments; blank lines are ignored.
//!   - A pattern **without** a `/` (e.g. `vendor`) is treated as `**/vendor`
//!     so it matches anywhere in the directory tree.
//!   - A pattern **with** a `/` is project-relative (e.g. `tests/fixtures/`).
//!   - A trailing `/` marks a directory pattern; it is stripped before
//!     matching, and the pattern is anchored with a trailing `/**` so all
//!     contents of that directory are excluded.
//!   - `*` matches within a single path component; `**` matches across
//!     multiple components.
//!
//! xray walks upward from the current working directory to find
//! `.xrayignore`, honouring the same discovery rule as `xray.toml`.

use glob::{MatchOptions, Pattern};

const MATCH_OPTS: MatchOptions = MatchOptions {
    case_sensitive: true,
    require_literal_separator: false,
    require_literal_leading_dot: false,
};

#[derive(Default)]
pub struct IgnorePatterns {
    patterns: Vec<Pattern>,
}

impl IgnorePatterns {
    /// Walk upward from `start_dir` looking for `.xrayignore`.
    /// Returns an empty set if no file is found.
    pub fn load(start_dir: &str) -> Self {
        let patterns = try_load(start_dir).unwrap_or_default();
        Self { patterns }
    }

    /// Build from a raw multi-line string (useful for tests and `xray init`
    /// preview).
    pub fn parse(contents: &str) -> Self {
        Self {
            patterns: parse_patterns(contents),
        }
    }

    /// Returns `true` if `path` matches any ignore pattern.
    pub fn is_ignored(&self, path: &str) -> bool {
        let p = std::path::Path::new(path);
        self.patterns
            .iter()
            .any(|pat| pat.matches_path_with(p, MATCH_OPTS))
    }
}

// ── internal helpers ──────────────────────────────────────────────────────────

fn try_load(start: &str) -> Option<Vec<Pattern>> {
    let mut dir = std::fs::canonicalize(start).ok()?;
    loop {
        let candidate = dir.join(".xrayignore");
        if candidate.exists() {
            let contents = std::fs::read_to_string(&candidate).ok()?;
            return Some(parse_patterns(&contents));
        }
        if !dir.pop() {
            break;
        }
    }
    None
}

fn parse_patterns(contents: &str) -> Vec<Pattern> {
    contents
        .lines()
        .map(str::trim)
        .filter(|l| !l.is_empty() && !l.starts_with('#'))
        .flat_map(build_patterns)
        .collect()
}

/// Returns one or more glob patterns for a single ignore line.
/// Bare names like `vendor` produce two patterns: `**/vendor` (the item itself)
/// and `**/vendor/**` (all contents), mirroring gitignore semantics.
fn build_patterns(raw: &str) -> Vec<Pattern> {
    // Strip leading '/' (means project-root-relative — handled identically
    // since we match against paths that start from the project root)
    let l = raw.trim_start_matches('/');

    let is_dir_pattern = l.ends_with('/');
    let l = l.trim_end_matches('/');

    if l.is_empty() {
        return Vec::new();
    }

    let mut strs = Vec::new();

    if is_dir_pattern {
        // Explicit directory pattern: match contents
        if l.contains('/') {
            strs.push(format!("{l}/**"));
        } else {
            strs.push(format!("**/{l}/**"));
        }
    } else if l.contains('/') {
        // Relative path like src/generated/*.py — use as-is
        strs.push(l.to_string());
    } else {
        // Bare name: match the item itself AND its contents
        strs.push(format!("**/{l}"));
        strs.push(format!("**/{l}/**"));
    }

    strs.iter().filter_map(|s| Pattern::new(s).ok()).collect()
}

// ── unit tests ────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn pat(s: &str) -> IgnorePatterns {
        IgnorePatterns::parse(s)
    }

    #[test]
    fn bare_name_matches_anywhere() {
        let ig = pat("vendor");
        assert!(ig.is_ignored("vendor/lib.py"));
        assert!(ig.is_ignored("src/vendor/lib.py"));
        assert!(!ig.is_ignored("src/not_vendor/lib.py"));
    }

    #[test]
    fn directory_pattern_matches_all_children() {
        let ig = pat("tests/fixtures/");
        assert!(ig.is_ignored("tests/fixtures/clean.py"));
        assert!(ig.is_ignored("tests/fixtures/nested/bad.py"));
        assert!(!ig.is_ignored("tests/other/bad.py"));
    }

    #[test]
    fn glob_wildcard_in_pattern() {
        let ig = pat("**/generated_*.py");
        assert!(ig.is_ignored("src/generated_models.py"));
        assert!(ig.is_ignored("deep/dir/generated_schema.py"));
        assert!(!ig.is_ignored("src/main.py"));
    }

    #[test]
    fn comments_and_blank_lines_are_ignored() {
        let ig = pat("# this is a comment\n\nvendor");
        assert!(ig.is_ignored("vendor/foo.py"));
    }

    #[test]
    fn leading_slash_stripped() {
        let ig = pat("/scripts/setup.py");
        assert!(ig.is_ignored("scripts/setup.py"));
    }

    #[test]
    fn empty_patterns_return_nothing_ignored() {
        let ig = IgnorePatterns::default();
        assert!(!ig.is_ignored("anything.py"));
    }
}
