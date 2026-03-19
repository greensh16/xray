# Changelog

All notable changes to xray are documented in this file.

The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/).
xray uses [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

---

## [Unreleased]

---

## [1.0.0] — 2026-03-19

### Added

- **Jupyter notebook support** — `.ipynb` files are now linted directly without
  any conversion step. Diagnostics include the cell number and per-cell
  line/column (e.g. `analysis.ipynb:cell[3]:2:5`). IPython magic lines
  (`%`/`!`) are stripped before parsing so they don't cause syntax errors, while
  preserving per-cell line numbers. Import context is accumulated across all
  cells so `import xarray` in cell 1 correctly gates xarray rules in cell 5.
- **XR008** — `open_mfdataset` without `parallel=True` (Warning): flags calls
  that open multi-file datasets without concurrent file-open via `dask.delayed`.
- **XR009** — `apply_ufunc` with `dask="allowed"` (Warning): flags the silent
  serial fallback mode; recommends `dask="parallelized"`.
- **XR010** — `xr.merge` inside a `for` loop (Warning): O(n²) coordinate
  alignment; collect datasets first then merge once.
- **XR011** — `to_netcdf()` without `encoding=` (Hint): variables written as
  float64 with no compression; suggests dtype + zlib encoding.
- **DK007** — `da.from_array()` without `chunks=` (Warning): single monolithic
  chunk defeats all Dask parallelism.
- **DK008** — `.rechunk()` inside a `for` loop (Warning): O(n) full
  re-partitions on an ever-growing array.
- **DK009** — `da.concatenate()` inside a `for` loop (Error): same O(n²)
  anti-pattern as XR007 / NP002 but for Dask arrays.
- Integration tests and `ExplainEntry` entries for all 7 new rules.
- Rule count updated to 32 across all docs and README.

---

## [0.9.0] — 2026-03-17

### Added

- **Stable JSON output schema** — `--format json` now emits a versioned envelope
  object with `schema_version: "1"`, a `diagnostics` array, and a `summary` object
  (`total`, `errors`, `warnings`, `hints`). Documented in `docs/json-schema.md`.
  The `build_json()` function is now public for consumers of the Rust library.
- **CRLF line-ending normalisation** — `parse_file()` and `parse_source()` now
  normalise `\r\n` to `\n` before parsing, so diagnostic line numbers are
  correct on files created on Windows or checked out with `core.autocrlf=true`.
- **Non-UTF-8 source hardening** — `parse_file()` reads bytes with
  `String::from_utf8_lossy` rather than `read_to_string`, replacing invalid
  bytes with the replacement character instead of returning `Err`. Non-ASCII
  paths are now handled correctly on all platforms.
- **Cross-platform CI matrix** — `.github/workflows/ci.yml` builds and tests
  on Linux x86-64, Linux aarch64 (via `cross`), macOS arm64, and Windows x86-64
  on every push and pull request. Release workflow creates GitHub releases and
  publishes to crates.io.
- **crates.io publish metadata** — `Cargo.toml` now includes `license`,
  `repository`, `homepage`, `documentation`, `keywords`, `categories`, `readme`,
  and `exclude` fields so `cargo install xray` works after release.
- **`collect_paths_pub()`** — public wrapper around the internal glob helper,
  exposed for integration testing and advanced library consumers.

### Changed

- Stable rule IDs declared: all rule IDs from XR001–XR007, DK001–DK006,
  NP001–NP007, and IO001–IO006 are frozen. No renumbering before v2.0.
- Stable config schema declared: `xray.toml` keys are frozen; additions only,
  no removals until v2.0.
- VS Code extension bumped to 0.9.0.

### Tests Added

- CRLF source parses without error and produces correct line/column numbers.
- Unicode multi-byte characters in source (CJK, accented, combining) do not
  shift line numbers or cause panics.
- Non-UTF-8 bytes (Latin-1) in source produce diagnostics via lossy conversion.
- `Config::from_file` returns `Err` for malformed TOML and missing files.
- Valid TOML config round-trips all fields correctly.
- CLI-level disable overrides config-level enables.
- Zero-match glob patterns return an empty vec without error.
- Literal file paths (non-glob) are collected as-is.
- Deeply nested `tests/**/*.py` glob matches all fixtures.
- JSON schema version field, diagnostics array, and summary counts are verified.

---

## [0.8.0] — 2026-03-17

### Added

- **Hosted documentation site** — full rule reference under `docs/rules/`, configuration
  guide at `docs/configuration.md`, HPC deployment cookbook at `docs/hpc-cookbook.md`,
  and per-rule "why this pattern is slow" explainers.
- **`CONTRIBUTING.md`** — step-by-step walkthrough for proposing, implementing, and
  testing a new rule, including the tree-sitter query authoring workflow.
- **Rule request issue template** — `.github/ISSUE_TEMPLATE/rule-request.md` with
  triage criteria and a structured proposal format for community-submitted rules.
- **Case studies** — two documented real-world examples of xray catching performance
  regressions on Gadi and Setonix (`docs/case-studies/`).
- **`CHANGELOG.md`** — this file; machine-readable history in Keep a Changelog format,
  maintained from this release onward.

### Changed

- `authors` field in `Cargo.toml` updated to `xray-hpc contributors`.
- VS Code extension bumped to 0.8.0.

---

## [0.7.0] — 2026-02-17

### Added

- **LSP server mode** — `xray lsp` runs a synchronous JSON-RPC 2.0 Language Server
  over stdin/stdout; no async runtime required.
  - Handles `initialize`, `initialized`, `textDocument/didOpen`, `textDocument/didSave`,
    `textDocument/didClose`, `shutdown`, and `exit`.
  - Publishes `textDocument/publishDiagnostics` after every open/save event.
  - `codeDescription.href` populated from each rule's docs URL.
- **VS Code extension** — `editors/vscode/` contains `package.json` and `extension.js`.
  - Spawns `xray lsp` as a subprocess; communicates via vscode-languageclient.
  - Settings: `xray.serverPath`, `xray.configFile`, `xray.minSeverity`, `xray.enabled`,
    `xray.trace.server`.
  - Commands: `xray.restartServer`, `xray.showOutput`.
  - Watches `xray.toml` and `.xrayignore` for workspace changes.
- **Watch mode** — `xray --watch` re-lints changed `.py` files on save using
  `notify::RecommendedWatcher`; 50 ms debounce avoids double-fire on atomic writes.
  - Respects `.xrayignore` and `[paths]` config excludes.
  - Prints a separator-bordered change summary to stderr on each lint cycle.
- **Diagnostic URLs** — all 25 rules now carry a `url` pointing to
  `https://github.com/greensh16/xray/rules/<RULE_ID>` for in-editor "more info" links.
  Five previously missing URLs added: DK002, NP002, NP003, NP004, IO004.

### Changed

- `SERVER_VERSION` in `lsp.rs` derives from `CARGO_PKG_VERSION` at compile time.

---

## [0.6.0] — 2026-01-17

### Added

- **GitHub Actions composite action** — `action.yml`; inputs: `paths`, `min-severity`,
  `fail-on`; downloads the binary from releases, runs xray, uploads SARIF to Code
  Scanning.
- **pre-commit hook** — `.pre-commit-hooks.yaml` with two hooks: `xray` (blocking on
  warnings) and `xray-warn-only` (always passes, warnings to stdout).
- **SARIF 2.1.0 output** — `--format sarif` emits a full `tool.driver.rules` array,
  per-result locations, fix objects, and `helpUri` from the rule's docs URL.
- **GitLab Code Quality report** — `--format gitlab-codequality` emits the JSON array
  format expected by GitLab CI; severity mapped to `critical`/`major`/`info`;
  fingerprints computed via FNV-1a (no extra dependency).
- **Diff-aware mode** — `xray --diff <REF>` lints only files changed since the given
  git ref (`--diff-filter=ACMR`; deleted files excluded).
- **Benchmark suite** — `benches/throughput.rs` using Criterion 0.5; tracks
  `bench_lint_fixture`, `bench_parse_only`, and `bench_all_fixtures`.

### Changed

- `OutputFormat` enum extended with `Sarif` and `GitlabCodequality` variants.
- `runner::RunResults` gains `paths: Vec<String>` for SARIF/GitLab consumers.

---

## [0.5.0] — 2025-12-17

### Added

- **`.xrayignore` file** — gitignore-style patterns; bare names expand to `**/name`,
  directory patterns append `/**`. File is discovered by walking up from the project
  root.
- **Per-rule severity overrides** — `[severity_overrides]` section in `xray.toml`
  maps rule IDs to `"hint"`, `"warning"`, or `"error"`.
- **`[paths]` config section** — `include` and `exclude` glob lists; default include
  is `["**/*.py"]`.
- **Environment variable support** — `XRAY_CONFIG`, `XRAY_FORMAT`,
  `XRAY_MIN_SEVERITY`, `XRAY_DISABLE` as fallbacks for all major CLI options.
- **Config validation** — `Config::validate()` emits clear errors for unknown rule
  IDs in `disable`, bad severity strings, or zero threshold values.

### Changed

- `Config` struct extended with `severity_overrides: HashMap<String, String>` and
  `paths: PathsConfig`.
- `xray init` template now includes commented `[severity_overrides]` and `[paths]`
  sections.

---

## [0.4.0] — 2025-11-17

### Added

- **`xray explain <RULE_ID>`** — prints rule rationale, bad/good code examples, and
  a link to relevant documentation; implemented for all 25 rules.
- **`xray init`** — scaffolds an annotated `xray.toml` in the current directory with
  all options commented out.
- **Auto-fix suggestions** — `fix_hint: Option<String>` field on `Diagnostic`;
  populated for mechanical fixes (e.g. add `chunks=` to `open_dataset`, replace
  `math.sqrt` with `np.sqrt`); surfaced in text and JSON output.
- **`--stats` flag** — per-rule and per-file summary table printed after linting.
- **Shell completions** — `xray completions <SHELL>` generates completion scripts
  for bash, zsh, and fish via clap_complete.
- **Exit code documentation** — codes 0 (clean), 1 (diagnostics found), 2 (fatal
  error) stabilised.

### Changed

- CLI refactored to clap subcommands: `explain`, `init`, `completions`; bare
  invocation still lints.
- `Cli` struct replaces `Args`; `XrayCommand` enum added.

---

## [0.3.0] — 2025-10-17

### Added

- **XR006** — `ds.to_array()` without `dim=` creates an unnamed concatenation
  dimension, causing silent downstream breakage.
- **XR007** — `xr.concat` inside a loop (O(n²) memory growth, same class of issue
  as NP002).
- **DK005** — `.persist()` result is never reused in the same scope; the persist
  call is wasted work.
- **DK006** — `.compute()` and `.persist()` mixed on the same graph in the same
  scope; graph is materialised twice.
- **NP006** — `np.matrix` usage flagged as deprecated; `np.ndarray` recommended.
- **NP007** — `DataFrame.applymap` / `Series.apply` with a Python lambda inside a
  loop; vectorised alternatives recommended.
- **IO005** — `h5py.File` opened without `swmr=True` in a parallel context.
- **IO006** — `xr.open_dataset(engine="scipy")` on files whose size exceeds a
  configurable threshold.

### Changed

- Total rule count: 17 → 25.
- Rule coverage documentation expanded with one worked example per new rule.

---

## [0.2.0] — 2025-09-17

### Added

- **Inline suppression comments** — `# xray: disable=XR001` suppresses the
  diagnostic on that line; `# xray: disable-file=XR001` suppresses the rule for
  the entire file.
- **AST-based import detection** — replaced substring matching with proper
  tree-sitter import-node traversal; eliminates false triggers from string literals
  and comments that mention library names.
- **Scope-aware XR002** — `.values` method-call guard; `dict.values()` and
  `set.values()` no longer trigger; only bare property access is flagged.
- **NP003 dtype detection hardening** — `dtype=` matched as an actual keyword
  argument AST node rather than a substring of the argument list.
- **NP004 scope expansion** — `math.*` scalar functions flagged everywhere; warning
  inside a for loop, hint when called outside one.
- **Fatal query compilation** — `.scm` syntax errors panic at startup rather than
  silently returning empty results.

---

## [0.1.0] — 2025-08-17

### Added

- **Core linting engine** — tree-sitter AST parsing with zero Python runtime
  dependency.
- **17 rules** across four domains:
  - xarray: XR001–XR005
  - dask: DK001–DK004
  - NumPy/pandas: NP001–NP005
  - scientific I/O: IO001–IO004
- **CLI** with `--format` (text/json), `--min-severity`, `--disable`, `--list-rules`.
- **TOML configuration** (`xray.toml`) with per-domain knobs and threshold settings.
- **Parallel file processing** via rayon.
- **Integration test suite** with clean/bad fixture files for all rule domains.

---

[Unreleased]: https://github.com/greensh16/xray/compare/v1.0.0...HEAD
[1.0.0]: https://github.com/greensh16/xray/compare/v0.9.0...v1.0.0
[0.9.0]: https://github.com/greensh16/xray/compare/v0.8.0...v0.9.0
[0.8.0]: https://github.com/greensh16/xray/compare/v0.7.0...v0.8.0
[0.7.0]: https://github.com/greensh16/xray/compare/v0.6.0...v0.7.0
[0.6.0]: https://github.com/greensh16/xray/compare/v0.5.0...v0.6.0
[0.5.0]: https://github.com/greensh16/xray/compare/v0.4.0...v0.5.0
[0.4.0]: https://github.com/greensh16/xray/compare/v0.3.0...v0.4.0
[0.3.0]: https://github.com/greensh16/xray/compare/v0.2.0...v0.3.0
[0.2.0]: https://github.com/greensh16/xray/compare/v0.1.0...v0.2.0
[0.1.0]: https://github.com/greensh16/xray/releases/tag/v0.1.0
