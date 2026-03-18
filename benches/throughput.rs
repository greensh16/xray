//! Throughput benchmarks for xray — tracks lint latency per file and
//! lines-of-code per second across the full rule pipeline.
//!
//! Run with:
//!   cargo bench
//!   cargo bench -- --save-baseline main   # save a named baseline
//!   cargo bench -- --baseline main        # compare against saved baseline

use criterion::{BenchmarkId, Criterion, Throughput, criterion_group, criterion_main};
use xray::{config::Config, parser, rules};

// ── per-file benchmarks ───────────────────────────────────────────────────────

/// Benchmark the full lint pipeline (parse + all rules) for each fixture.
fn bench_lint_fixture(c: &mut Criterion) {
    let config = Config::default();

    let fixtures: &[(&str, &str)] = &[
        ("xarray_bad", "tests/fixtures/xarray_bad.py"),
        ("dask_bad", "tests/fixtures/dask_bad.py"),
        ("numpy_bad", "tests/fixtures/numpy_bad.py"),
        ("io_bad", "tests/fixtures/io_bad.py"),
        ("clean", "tests/fixtures/clean.py"),
    ];

    let mut group = c.benchmark_group("lint_fixture");

    for (label, path) in fixtures {
        // Measure throughput in lines of code so that Criterion shows LOC/sec
        let source =
            std::fs::read_to_string(path).unwrap_or_else(|_| panic!("fixture not found: {path}"));
        let loc = source.lines().count() as u64;
        group.throughput(Throughput::Elements(loc));

        group.bench_with_input(BenchmarkId::new("file", label), path, |b, p| {
            b.iter(|| {
                let parsed = parser::parse_file(p).expect("fixture should parse");
                rules::run_all(&parsed, p, &config)
            });
        });
    }

    group.finish();
}

// ── parse-only benchmark ──────────────────────────────────────────────────────

/// Isolate the tree-sitter parse cost from the rule-check cost.
fn bench_parse_only(c: &mut Criterion) {
    let fixtures: &[(&str, &str)] = &[
        ("xarray_bad", "tests/fixtures/xarray_bad.py"),
        ("dask_bad", "tests/fixtures/dask_bad.py"),
        ("numpy_bad", "tests/fixtures/numpy_bad.py"),
        ("io_bad", "tests/fixtures/io_bad.py"),
        ("clean", "tests/fixtures/clean.py"),
    ];

    let mut group = c.benchmark_group("parse_only");

    for (label, path) in fixtures {
        let source =
            std::fs::read_to_string(path).unwrap_or_else(|_| panic!("fixture not found: {path}"));
        let loc = source.lines().count() as u64;
        group.throughput(Throughput::Elements(loc));

        group.bench_with_input(BenchmarkId::new("file", label), path, |b, p| {
            b.iter(|| parser::parse_file(p).expect("fixture should parse"));
        });
    }

    group.finish();
}

// ── aggregate benchmark ───────────────────────────────────────────────────────

/// Simulate linting an entire "project" (all fixtures in sequence) to measure
/// wall-clock cost of a typical `xray` invocation.
fn bench_all_fixtures(c: &mut Criterion) {
    let config = Config::default();
    let paths = [
        "tests/fixtures/xarray_bad.py",
        "tests/fixtures/dask_bad.py",
        "tests/fixtures/numpy_bad.py",
        "tests/fixtures/io_bad.py",
        "tests/fixtures/clean.py",
    ];

    let total_loc: u64 = paths
        .iter()
        .map(|p| {
            std::fs::read_to_string(p)
                .map(|s| s.lines().count() as u64)
                .unwrap_or(0)
        })
        .sum();

    let mut group = c.benchmark_group("lint_all_fixtures");
    group.throughput(Throughput::Elements(total_loc));

    group.bench_function("all_files", |b| {
        b.iter(|| {
            paths.iter().for_each(|p| {
                let parsed = parser::parse_file(p).expect("fixture should parse");
                let _ = rules::run_all(&parsed, p, &config);
            });
        });
    });

    group.finish();
}

criterion_group!(
    benches,
    bench_lint_fixture,
    bench_parse_only,
    bench_all_fixtures
);
criterion_main!(benches);
