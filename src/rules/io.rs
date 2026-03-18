use streaming_iterator::StreamingIterator;
use tree_sitter::{Query, QueryCursor};

use crate::{
    config::Config,
    diagnostic::{Diagnostic, RuleMeta, Severity},
    parser::{
        ParsedFile, has_keyword_arg, is_inside_for_loop, keyword_arg_value, node_text, position,
    },
};

use super::RuleSet;

pub struct IoRules;

const QUERY_SRC: &str = include_str!("../../queries/io.scm");

impl RuleSet for IoRules {
    fn meta() -> Vec<RuleMeta> {
        vec![
            RuleMeta {
                id: "IO001",
                name: "np-save-large-arrays",
                severity: Severity::Hint,
                description: "np.save() used — uncompressed, unchunked; prefer Zarr or HDF5 for large arrays",
            },
            RuleMeta {
                id: "IO002",
                name: "netcdf4-direct-open",
                severity: Severity::Hint,
                description: "netCDF4.Dataset opened directly — bypasses xarray coordinate alignment machinery",
            },
            RuleMeta {
                id: "IO003",
                name: "zarr-open-without-chunks",
                severity: Severity::Warning,
                description: "zarr.open called without chunks= — unchunked Zarr defeats compression and parallel I/O",
            },
            RuleMeta {
                id: "IO004",
                name: "netcdf4-read-in-loop",
                severity: Severity::Warning,
                description: "netCDF4 variable subscripted inside a loop — each read may hit disk; pre-load outside the loop",
            },
            RuleMeta {
                id: "IO005",
                name: "h5py-file-without-swmr",
                severity: Severity::Hint,
                description: "h5py.File opened without swmr=True — consider SWMR mode for concurrent HPC read workflows",
            },
            RuleMeta {
                id: "IO006",
                name: "open-dataset-scipy-engine",
                severity: Severity::Warning,
                description: "xr.open_dataset called with engine='scipy' — loads eagerly without chunking; use 'netcdf4' or 'zarr'",
            },
        ]
    }

    fn check(file: &ParsedFile, path: &str, config: &Config) -> Vec<Diagnostic> {
        let mut diags = Vec::new();
        let source = file.source.as_bytes();
        let lang = tree_sitter_python::LANGUAGE.into();

        // Query compilation errors are a bug in xray itself — fail loudly.
        let query = Query::new(&lang, QUERY_SRC)
            .unwrap_or_else(|e| panic!("xray: BUG — failed to compile io query: {e}"));

        let mut cursor = QueryCursor::new();
        let root = file.tree.root_node();

        let mut matches = cursor.matches(&query, root, source);
        while let Some(m) = matches.next() {
            match m.pattern_index {
                // IO001 — np.save
                0 if !config.is_disabled("IO001") => {
                    if let Some(node) = query
                        .capture_index_for_name("io_npsave_call")
                        .and_then(|i| m.nodes_for_capture_index(i).next())
                    {
                        // Only fire for np.save / numpy.save — not any arbitrary .save() call
                        if let Some(func) = node.child_by_field_name("function") {
                            if func.kind() == "attribute" {
                                if let Some(obj) = func.child_by_field_name("object") {
                                    let obj_name = node_text(&obj, source);
                                    if obj_name != "np" && obj_name != "numpy" {
                                        continue;
                                    }
                                } else {
                                    // attribute with no object — not np.save
                                    continue;
                                }
                            } else {
                                // bare `save(...)` call — skip (ambiguous without import check)
                                continue;
                            }
                        }
                        let (line, col) = position(&node);
                        diags.push(
                            Diagnostic::new(
                                "IO001",
                                Severity::Hint,
                                path,
                                line,
                                col,
                                "`np.save()` stores arrays uncompressed and unchunked — poor for large HPC datasets",
                            )
                            .with_suggestion("Use `zarr.save(path, arr, chunks=(...), compressor=Blosc())` or `h5py` for large scientific arrays")
                            .with_url("https://zarr.readthedocs.io/en/stable/"),
                        );
                    }
                }

                // IO002 — netCDF4.Dataset direct open
                1 if !config.is_disabled("IO002") => {
                    if let Some(node) = query
                        .capture_index_for_name("io_nc4_dataset_call")
                        .and_then(|i| m.nodes_for_capture_index(i).next())
                    {
                        let (line, col) = position(&node);
                        diags.push(
                            Diagnostic::new(
                                "IO002",
                                Severity::Hint,
                                path,
                                line,
                                col,
                                "`netCDF4.Dataset()` bypasses xarray's coordinate alignment, CF metadata, and lazy loading",
                            )
                            .with_suggestion("Use `xr.open_dataset(path, chunks='auto')` unless you specifically need the low-level netCDF4 API")
                            .with_url("https://docs.xarray.dev/en/stable/generated/xarray.open_dataset.html"),
                        );
                    }
                }

                // IO003 — zarr.open without chunks
                // Use AST-based keyword argument check to avoid substring false positives
                // (e.g. a string argument that happens to contain "chunks").
                2 if !config.is_disabled("IO003") => {
                    if let Some(call_node) = query
                        .capture_index_for_name("io_zarr_open_call")
                        .and_then(|i| m.nodes_for_capture_index(i).next())
                    {
                        // Only fire for zarr.open* — not any open() call
                        let is_zarr_call =
                            if let Some(func) = call_node.child_by_field_name("function") {
                                if func.kind() == "attribute" {
                                    func.child_by_field_name("object")
                                        .map(|obj| node_text(&obj, source) == "zarr")
                                        .unwrap_or(false)
                                } else {
                                    // bare open() — only flag if zarr is imported
                                    file.imports.zarr
                                }
                            } else {
                                false
                            };
                        if !is_zarr_call {
                            continue;
                        }
                        if !has_keyword_arg(call_node, source, "chunks") {
                            let (line, col) = position(&call_node);
                            diags.push(
                                Diagnostic::new(
                                    "IO003",
                                    Severity::Warning,
                                    path,
                                    line,
                                    col,
                                    "`zarr.open()` called without `chunks=` — the array is stored as a single chunk, disabling parallel I/O",
                                )
                                .with_suggestion("Set `chunks` to match your access pattern, e.g. `chunks=(time, 256, 256)` for time-series grids")
                                .with_url("https://zarr.readthedocs.io/en/stable/tutorial.html#chunk-optimizations"),
                            );
                        }
                    }
                }

                // IO004 — netCDF4 variable read in loop
                3 if !config.is_disabled("IO004") => {
                    if let Some(node) = query
                        .capture_index_for_name("io_nc_subscript")
                        .and_then(|i| m.nodes_for_capture_index(i).next())
                    {
                        // Only fire inside for loops when netCDF4 is imported
                        if !file.imports.netcdf4 || !is_inside_for_loop(node) {
                            continue;
                        }
                        let (line, col) = position(&node);
                        diags.push(
                            Diagnostic::new(
                                "IO004",
                                Severity::Warning,
                                path,
                                line,
                                col,
                                "netCDF4 variable subscripted inside a for loop — each read may trigger a disk seek",
                            )
                            .with_suggestion("Pre-load the full array outside the loop with `data = nc_var[:]`, then index `data[i]`")
                            .with_url("https://github.com/greensh16/xray/wiki/IO-Rules#io004"),
                        );
                    }
                }

                // IO005 — h5py.File without swmr=True
                4 if !config.is_disabled("IO005") => {
                    if let Some(call_node) = query
                        .capture_index_for_name("io_h5py_file_call")
                        .and_then(|i| m.nodes_for_capture_index(i).next())
                    {
                        // Only fire for h5py.File — not any .File() or File() call
                        let is_h5py_call =
                            if let Some(func) = call_node.child_by_field_name("function") {
                                if func.kind() == "attribute" {
                                    func.child_by_field_name("object")
                                        .map(|obj| node_text(&obj, source) == "h5py")
                                        .unwrap_or(false)
                                } else {
                                    // bare File() — only flag if h5py is imported
                                    file.imports.h5py
                                }
                            } else {
                                false
                            };
                        if !is_h5py_call {
                            continue;
                        }
                        if !has_keyword_arg(call_node, source, "swmr") {
                            let (line, col) = position(&call_node);
                            diags.push(
                                Diagnostic::new(
                                    "IO005",
                                    Severity::Hint,
                                    path,
                                    line,
                                    col,
                                    "`h5py.File()` opened without `swmr=True` — concurrent reads in an HPC job may return stale data",
                                )
                                .with_suggestion("Add `swmr=True` when the file will be read concurrently by multiple processes")
                                .with_url("https://docs.h5py.org/en/stable/swmr.html"),
                            );
                        }
                    }
                }

                // IO006 — xr.open_dataset with engine="scipy"
                5 if !config.is_disabled("IO006") => {
                    if let Some(call_node) = query
                        .capture_index_for_name("io_open_scipy_call")
                        .and_then(|i| m.nodes_for_capture_index(i).next())
                    {
                        // Only fire when engine= is explicitly set to "scipy"
                        let engine = keyword_arg_value(call_node, source, "engine");
                        if engine.map(|v| v.contains("scipy")).unwrap_or(false) {
                            let (line, col) = position(&call_node);
                            diags.push(
                                Diagnostic::new(
                                    "IO006",
                                    Severity::Warning,
                                    path,
                                    line,
                                    col,
                                    "`engine='scipy'` loads the entire file eagerly — no chunking, no lazy access, poor for large HPC datasets",
                                )
                                .with_suggestion("Use `engine='netcdf4'` for standard NetCDF files, or `engine='zarr'` for chunked cloud-native storage")
                                .with_fix_hint("engine=\"netcdf4\"")
                                .with_url("https://docs.xarray.dev/en/stable/generated/xarray.open_dataset.html"),
                            );
                        }
                    }
                }

                _ => {}
            }
        }

        diags
    }
}
