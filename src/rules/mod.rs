pub mod dask;
pub mod io;
pub mod numpy;
pub mod xarray;

use crate::{
    config::Config,
    diagnostic::{Diagnostic, RuleMeta},
    parser::ParsedFile,
};

/// Every rule set implements this trait.
pub trait RuleSet {
    fn meta() -> Vec<RuleMeta>
    where
        Self: Sized;

    fn check(file: &ParsedFile, path: &str, config: &Config) -> Vec<Diagnostic>
    where
        Self: Sized;
}

/// Run all rule sets against a single parsed file.
pub fn run_all(file: &ParsedFile, path: &str, config: &Config) -> Vec<Diagnostic> {
    let mut out = Vec::new();

    if file.imports.xarray {
        out.extend(xarray::XarrayRules::check(file, path, config));
    }
    if file.imports.dask {
        out.extend(dask::DaskRules::check(file, path, config));
    }
    if file.imports.numpy || file.imports.pandas {
        out.extend(numpy::NumpyRules::check(file, path, config));
    }
    // IO rules fire whenever any relevant library is imported
    if file.imports.netcdf4 || file.imports.zarr || file.imports.numpy || file.imports.h5py {
        out.extend(io::IoRules::check(file, path, config));
    }

    // Apply inline suppressions
    out.retain(|d| !file.suppressions.is_suppressed(d.rule_id, d.line));

    // Sort by line number for readable output
    out.sort_by_key(|d| d.line);
    out
}

/// All rule metadata for --list-rules
pub fn all_meta() -> Vec<RuleMeta> {
    let mut meta = Vec::new();
    meta.extend(xarray::XarrayRules::meta());
    meta.extend(dask::DaskRules::meta());
    meta.extend(numpy::NumpyRules::meta());
    meta.extend(io::IoRules::meta());
    meta
}
