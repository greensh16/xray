# xray

A fast, self-contained Rust linter for scientific Python workflows on HPC systems.
Targets **xarray**, **dask**, **NumPy**, **pandas**, and **scientific I/O** patterns
that general-purpose linters (ruff, pylint) don't cover.

**Zero Python runtime required** — ships as a single binary. Runs on Gadi, Setonix,
or any HPC cluster without loading a Python module.

---

## Installation

Download a pre-built binary from the [releases page](https://github.com/greensh16/xray/releases/latest):

```bash
curl -L https://github.com/greensh16/xray/releases/download/v1.0.0/xray-linux-x86_64 \
  -o ~/.local/bin/xray && chmod +x ~/.local/bin/xray
```

---

## Usage

```bash
xray                          # lint all .py files in the project
xray src/analysis.py          # single file
xray analysis.ipynb           # Jupyter notebook — each code cell linted independently
xray --min-severity warning   # warnings and errors only
xray --format json src/ > report.json
xray --diff HEAD~1            # only files changed since last commit
xray --watch                  # re-lint on save
xray explain XR001            # show rationale and fix examples for a rule
xray init                     # write an annotated xray.toml to the current directory
```

Exit codes: `0` clean · `1` diagnostics found · `2` fatal error.

---

## Rules

32 rules across four domains. All IDs are stable.

### XR — xarray

| ID | Default | Description |
|----|---------|-------------|
| XR001 | warning | `open_dataset` / `open_mfdataset` without `chunks=` — eager load |
| XR002 | warning | `.values` on a lazy DataArray — forces full compute |
| XR003 | hint | `for` loop iterating over a dimension attribute |
| XR004 | warning | `.sel()` called with a float literal |
| XR005 | error | `.compute()` called inside a `for` loop |
| XR006 | warning | `ds.to_array()` without `dim=` — unnamed concat dimension |
| XR007 | error | `xr.concat` inside a loop — O(n²) memory growth |
| XR008 | warning | `open_mfdataset` without `parallel=True` — serial file open |
| XR009 | warning | `apply_ufunc` with `dask="allowed"` — silent serial fallback |
| XR010 | warning | `xr.merge` inside a loop — O(n²) alignment cost |
| XR011 | hint | `to_netcdf()` without `encoding=` — no compression |

### DK — Dask

| ID | Default | Description |
|----|---------|-------------|
| DK001 | error | `.compute()` inside a `for` loop |
| DK002 | error | `dask.compute()` inside a `for` loop |
| DK003 | warning | Excessive `.compute()` calls — consider `.persist()` |
| DK004 | hint | Immediate `.compute()` on a just-constructed dask object |
| DK005 | warning | `.persist()` result not assigned — graph cost wasted |
| DK006 | warning | `.persist().compute()` chain — redundant round-trip |
| DK007 | warning | `da.from_array()` without `chunks=` — single monolithic chunk |
| DK008 | warning | `.rechunk()` inside a `for` loop |
| DK009 | error | `da.concatenate()` inside a loop — O(n²) intermediate copies |

### NP — NumPy / pandas

| ID | Default | Description |
|----|---------|-------------|
| NP001 | warning | `np.loadtxt` on large files — use `pd.read_csv` or `np.fromfile` |
| NP002 | warning | `np.append` inside a loop — O(n²) copies |
| NP003 | hint | `np.array` / `np.zeros` / `np.ones` without `dtype=` |
| NP004 | warning | `math.*` scalar function inside a loop — use NumPy ufunc |
| NP005 | warning | `np.where` with a Python callable — both branches evaluated eagerly |
| NP006 | warning | `np.matrix` usage — deprecated, use `np.ndarray` and `@` |
| NP007 | warning | `DataFrame.applymap` / `Series.apply(lambda)` inside a loop |

### IO — Scientific I/O

| ID | Default | Description |
|----|---------|-------------|
| IO001 | hint | NetCDF4 opened without explicit `mode=` |
| IO002 | warning | `xr.open_dataset` inside a loop — use `open_mfdataset` |
| IO003 | hint | Zarr store opened without `consolidated=True` |
| IO004 | warning | `h5py` dataset sliced without checking chunk layout |
| IO005 | warning | `h5py.File` without `swmr=True` in a parallel context |
| IO006 | warning | `xr.open_dataset(engine="scipy")` on large files |

---

## Configuration

`xray.toml` is discovered by walking up from the project root:

```toml
disable = ["IO001"]
min_severity = "hint"

[severity_overrides]
XR001 = "error"   # promote to error
NP003 = "hint"    # demote to hint

[paths]
include = ["src/**/*.py", "notebooks/**/*.ipynb"]
exclude = ["tests/fixtures/**"]
```

Per-line suppression: `# xray: disable=XR001`
Per-file suppression: `# xray: disable-file=XR001`

Environment variables: `XRAY_CONFIG`, `XRAY_FORMAT`, `XRAY_MIN_SEVERITY`, `XRAY_DISABLE`.

---

## Output Formats

| Flag | Use case |
|------|----------|
| `--format text` | Human-readable terminal output (default) |
| `--format json` | Versioned JSON envelope — see [JSON schema docs](https://github.com/greensh16/xray/wiki/JSON-Output-Schema) |
| `--format sarif` | GitHub Code Scanning / any SARIF 2.1.0 consumer |
| `--format gitlab-codequality` | GitLab CI Code Quality report |

---

## Documentation

Full documentation lives on the [GitHub Wiki](https://github.com/greensh16/xray/wiki):

- [Rule reference](https://github.com/greensh16/xray/wiki/Rule-Reference) — rationale, examples, and fix hints for all 32 rules
- [Configuration guide](https://github.com/greensh16/xray/wiki/Configuration) — full `xray.toml` schema
- [JSON output schema](https://github.com/greensh16/xray/wiki/JSON-Output-Schema) — stable v1 field reference
- [HPC deployment cookbook](https://github.com/greensh16/xray/wiki/HPC-Deployment-Cookbook) — Gadi, Setonix, PBS, Slurm
- [Case studies](https://github.com/greensh16/xray/wiki/Case-Studies) — real-world performance regressions caught by xray

---

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md) for a step-by-step guide to proposing and
implementing new rules, including the tree-sitter query authoring workflow.

To request a new rule, use the [rule request issue template](.github/ISSUE_TEMPLATE/rule-request.md).

## Scope

xray uses syntactic analysis — it reads source text without executing it or
resolving types. Rules fire based on API names and the presence of relevant imports.
It won't catch issues that require runtime shape or dtype information. For general
Python quality, run **ruff** alongside xray.
