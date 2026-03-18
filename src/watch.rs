//! File-watch mode for `xray --watch`.
//!
//! On startup, performs a full lint of all matched Python files.  Then it
//! watches the file system for changes and re-lints any modified `.py` file
//! as soon as the change is detected.
//!
//! Uses the `notify` crate which selects the best OS-level watcher available
//! (inotify on Linux, kqueue on macOS, FSEvents on macOS 10.7+,
//! ReadDirectoryChangesW on Windows) and falls back gracefully when those
//! mechanisms are unavailable (e.g. over NFS/Lustre on HPC nodes).
//!
//! Usage:
//!   xray --watch                # watch all Python files recursively
//!   xray --watch src/           # watch a specific directory
//!   xray --watch analysis.py    # watch a single file

use anyhow::Result;
use notify::{
    Config as NotifyConfig, Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher,
};
use std::{
    collections::HashSet,
    path::{Path, PathBuf},
    sync::mpsc,
    time::{Duration, Instant},
};

use crate::{cli::Cli, config::Config, ignore::IgnorePatterns, parser, rules};

/// Run the watch loop.  Lints all matching files on start, then re-lints on
/// every `.py` file-save event.  Blocks until the user presses Ctrl-C.
pub fn run_watch(cli: &Cli, config: &Config) -> Result<()> {
    // ── Initial lint ──────────────────────────────────────────────────────────
    eprintln!("xray: starting watch mode (Ctrl-C to stop)");
    eprintln!("{}", "─".repeat(72));
    lint_and_print_paths(&collect_watch_paths(cli, config)?, config);
    eprintln!("{}", "─".repeat(72));

    // ── Set up file watcher ───────────────────────────────────────────────────
    let (tx, rx) = mpsc::channel::<notify::Result<Event>>();
    let mut watcher = RecommendedWatcher::new(tx, NotifyConfig::default())?;

    // Determine which paths to watch.  If the user supplied explicit file
    // paths, watch their parent directories; otherwise watch the roots.
    let watch_roots = watch_roots(cli);
    for root in &watch_roots {
        let mode = if Path::new(root).is_file() {
            RecursiveMode::NonRecursive
        } else {
            RecursiveMode::Recursive
        };
        if let Err(e) = watcher.watch(Path::new(root), mode) {
            eprintln!("xray: cannot watch {root}: {e}");
        }
    }

    let ignore = IgnorePatterns::load(".");

    // ── Event loop ─────────────────────────────────────────────────────────
    // Debounce: collect events for up to 50 ms so that editors that write
    // in multiple steps (write temp, rename) trigger only one lint cycle.
    let debounce = Duration::from_millis(50);
    let mut pending: HashSet<PathBuf> = HashSet::new();
    let mut last_event = Instant::now();

    loop {
        // Try to drain events with a timeout
        match rx.recv_timeout(debounce) {
            Ok(Ok(event)) => {
                if is_modify_event(&event) {
                    for path in event.paths {
                        if is_python_file(&path) && !ignore.is_ignored(path.to_str().unwrap_or(""))
                        {
                            pending.insert(path);
                            last_event = Instant::now();
                        }
                    }
                }
            }
            Ok(Err(e)) => eprintln!("xray: watch error: {e}"),
            Err(mpsc::RecvTimeoutError::Timeout) => {}
            Err(mpsc::RecvTimeoutError::Disconnected) => break,
        }

        // Flush pending paths once the debounce window has passed
        if !pending.is_empty() && last_event.elapsed() >= debounce {
            let paths: Vec<String> = pending
                .drain()
                .filter_map(|p| p.to_str().map(String::from))
                .collect();

            eprintln!();
            eprintln!(
                "xray: {} file{} changed — re-linting...",
                paths.len(),
                if paths.len() == 1 { "" } else { "s" }
            );
            eprintln!("{}", "─".repeat(72));
            lint_and_print_paths(&paths, config);
            eprintln!("{}", "─".repeat(72));
        }
    }

    Ok(())
}

// ── internal helpers ──────────────────────────────────────────────────────────

fn watch_roots(cli: &Cli) -> Vec<String> {
    if cli.paths.is_empty() {
        vec![".".to_string()]
    } else {
        cli.paths.clone()
    }
}

fn collect_watch_paths(cli: &Cli, config: &Config) -> Result<Vec<String>> {
    use glob::glob;

    let raw_patterns: Vec<String> = if cli.paths.is_empty() {
        config.paths.include.clone()
    } else {
        cli.paths.clone()
    };

    let mut paths = Vec::new();
    for pattern in &raw_patterns {
        if Path::new(pattern).is_file() {
            paths.push(pattern.clone());
            continue;
        }
        for entry in glob(pattern)
            .map_err(|e| anyhow::anyhow!("invalid glob: {e}"))?
            .flatten()
        {
            if let Some(s) = entry.to_str() {
                paths.push(s.to_string());
            }
        }
    }

    // Apply excludes and .xrayignore
    let ignore = IgnorePatterns::load(".");
    paths.retain(|p| {
        !config.paths.exclude.iter().any(|ex| {
            glob::Pattern::new(ex)
                .map(|pat| pat.matches_path(Path::new(p)))
                .unwrap_or(false)
        }) && !ignore.is_ignored(p)
    });

    Ok(paths)
}

fn lint_and_print_paths(paths: &[String], config: &Config) {
    let mut total = 0usize;
    for path in paths {
        match parser::parse_file(path) {
            Ok(parsed) => {
                let diags = rules::run_all(&parsed, path, config);
                total += diags.len();
                for d in &diags {
                    eprintln!("  {}:{}: [{}] {}", d.file, d.line, d.rule_id, d.message);
                    if let Some(ref fix) = d.fix_hint {
                        eprintln!("    fix: {fix}");
                    }
                }
            }
            Err(e) => eprintln!("xray: could not parse {path}: {e}"),
        }
    }
    if total == 0 {
        eprintln!(
            "  ✓ no issues found in {} file{}",
            paths.len(),
            if paths.len() == 1 { "" } else { "s" }
        );
    } else {
        eprintln!(
            "  {} issue{} found",
            total,
            if total == 1 { "" } else { "s" }
        );
    }
}

fn is_python_file(path: &Path) -> bool {
    path.extension().is_some_and(|ext| ext == "py")
}

fn is_modify_event(event: &Event) -> bool {
    matches!(
        event.kind,
        EventKind::Create(_) | EventKind::Modify(_) | EventKind::Remove(_)
    )
}
