use anyhow::Result;
use std::collections::{HashMap, HashSet};
use tree_sitter::{Node, Parser, Tree};

/// A parsed Python source file ready for rule inspection.
pub struct ParsedFile {
    pub source: String,
    pub tree: Tree,
    pub imports: ImportContext,
    pub suppressions: Suppressions,
}

/// Which scientific libraries are imported in this file — used to
/// gate rules so we only flag e.g. xarray patterns if xarray is present.
#[derive(Debug, Default, Clone)]
pub struct ImportContext {
    pub xarray: bool,
    pub dask: bool,
    pub numpy: bool,
    pub pandas: bool,
    pub netcdf4: bool,
    pub zarr: bool,
    pub h5py: bool,
}

impl ImportContext {
    /// Scan the top-level import statements using AST node traversal to build
    /// the context. This avoids false positives from string literals or
    /// comments that happen to contain library names.
    fn from_tree(root: Node<'_>, source: &[u8]) -> Self {
        let mut ctx = ImportContext::default();

        let mut cursor = root.walk();
        for child in root.children(&mut cursor) {
            match child.kind() {
                // `import xarray` or `import xarray as xr` or `import dask.array as da`
                "import_statement" => {
                    let mut c = child.walk();
                    for name_node in child.children(&mut c) {
                        let module_root = match name_node.kind() {
                            // bare import: `import xarray`
                            "dotted_name" => name_node,
                            // aliased: `import xarray as xr`
                            "aliased_import" => {
                                if let Some(n) = name_node.child_by_field_name("name") {
                                    n
                                } else {
                                    continue;
                                }
                            }
                            _ => continue,
                        };
                        // Only look at the leading identifier of the dotted path
                        if let Some(first) = module_root.child(0) {
                            if first.kind() == "identifier" {
                                let name = node_text(&first, source);
                                Self::mark_by_name(&mut ctx, name);
                            }
                        }
                    }
                }
                // `from xarray import DataArray` or `from dask.array import from_delayed`
                "import_from_statement" => {
                    if let Some(module_node) = child.child_by_field_name("module_name") {
                        // First identifier in the dotted module path
                        if let Some(first) = module_node.child(0) {
                            if first.kind() == "identifier" {
                                let name = node_text(&first, source);
                                Self::mark_by_name(&mut ctx, name);
                            }
                        }
                    }
                }
                _ => {}
            }
        }
        ctx
    }

    fn mark_by_name(ctx: &mut Self, module: &str) {
        match module {
            "xarray" => ctx.xarray = true,
            "dask" => ctx.dask = true,
            "numpy" => ctx.numpy = true,
            "pandas" => ctx.pandas = true,
            "netCDF4" | "netcdf4" => ctx.netcdf4 = true,
            "zarr" => ctx.zarr = true,
            "h5py" => ctx.h5py = true,
            _ => {}
        }
    }
}

/// Per-file and per-line inline suppression state, built from
/// `# xray: disable=RULE_ID` and `# xray: disable-file=RULE_ID` comments.
#[derive(Debug, Default)]
pub struct Suppressions {
    /// Rules suppressed for the entire file
    pub file_level: HashSet<String>,
    /// Rules suppressed on a specific 1-based line number
    pub line_level: HashMap<usize, HashSet<String>>,
}

impl Suppressions {
    /// Returns true if `rule_id` is suppressed, either file-wide or on `line`.
    pub fn is_suppressed(&self, rule_id: &str, line: usize) -> bool {
        self.file_level.contains(rule_id)
            || self
                .line_level
                .get(&line)
                .is_some_and(|s| s.contains(rule_id))
    }

    fn from_source(source: &str) -> Self {
        let mut s = Suppressions::default();
        for (i, line) in source.lines().enumerate() {
            let line_num = i + 1;
            // Look for `# xray:` anywhere on the line (allows inline comments)
            if let Some(pos) = line.find("# xray:") {
                let after = line[pos + 7..].trim_start(); // text after "# xray:"
                if let Some(rules_str) = after.strip_prefix("disable-file=") {
                    // File-wide: # xray: disable-file=XR001,XR002
                    for rule in rules_str
                        .split(',')
                        .map(str::trim)
                        .filter(|s| !s.is_empty())
                    {
                        s.file_level.insert(rule.to_string());
                    }
                } else if let Some(rules_str) = after.strip_prefix("disable=") {
                    // Line-level: # xray: disable=XR001
                    for rule in rules_str
                        .split(',')
                        .map(str::trim)
                        .filter(|s| !s.is_empty())
                    {
                        s.line_level
                            .entry(line_num)
                            .or_default()
                            .insert(rule.to_string());
                    }
                }
            }
        }
        s
    }
}

pub fn parse_file(path: &str) -> Result<ParsedFile> {
    // Read raw bytes so we handle non-ASCII path characters on all platforms
    // and gracefully recover from non-UTF-8 bytes (e.g. latin-1 comments)
    // by replacing them with the UTF-8 replacement character rather than
    // returning a hard error.
    let bytes = std::fs::read(path).map_err(|e| anyhow::anyhow!("cannot read {path}: {e}"))?;
    let source = String::from_utf8_lossy(&bytes).into_owned();
    parse_source(source)
}

pub fn parse_source(source: String) -> Result<ParsedFile> {
    // Normalise Windows CRLF line endings to LF so that:
    //  1. tree-sitter row numbers match our 1-based line numbers.
    //  2. Suppression comment scanning via `str::lines()` behaves correctly.
    // This is a no-op on files that already use LF.
    let source = source.replace("\r\n", "\n");
    let mut parser = Parser::new();
    parser.set_language(&tree_sitter_python::LANGUAGE.into())?;

    let tree = parser
        .parse(&source, None)
        .ok_or_else(|| anyhow::anyhow!("tree-sitter failed to produce a parse tree"))?;

    let imports = ImportContext::from_tree(tree.root_node(), source.as_bytes());
    let suppressions = Suppressions::from_source(&source);

    Ok(ParsedFile {
        source,
        tree,
        imports,
        suppressions,
    })
}

/// Convenience: get 1-based (line, col) from a tree-sitter node
pub fn position(node: &Node<'_>) -> (usize, usize) {
    let p = node.start_position();
    (p.row + 1, p.column + 1)
}

/// Extract the UTF-8 text of a node
pub fn node_text<'a>(node: &Node<'_>, source: &'a [u8]) -> &'a str {
    node.utf8_text(source).unwrap_or("<invalid utf8>")
}

/// Returns true if `call_node` has a keyword_argument whose name exactly matches `kw`.
/// Uses AST traversal rather than substring matching to avoid false positives.
pub fn has_keyword_arg(call_node: Node<'_>, source: &[u8], kw: &str) -> bool {
    let mut cursor = call_node.walk();
    for child in call_node.children(&mut cursor) {
        if child.kind() == "argument_list" {
            let mut arg_cursor = child.walk();
            for arg in child.children(&mut arg_cursor) {
                if arg.kind() == "keyword_argument" {
                    if let Some(name_node) = arg.child_by_field_name("name") {
                        if node_text(&name_node, source) == kw {
                            return true;
                        }
                    }
                }
            }
        }
    }
    false
}

/// Returns true if `node` is nested anywhere inside a `for_statement` body.
pub fn is_inside_for_loop(node: Node<'_>) -> bool {
    let mut current = node.parent();
    while let Some(n) = current {
        if n.kind() == "for_statement" {
            return true;
        }
        current = n.parent();
    }
    false
}

/// Returns the raw source text of the value of the keyword argument named `kw`
/// in `call_node`, or `None` if no such keyword argument exists.
/// The returned text includes quotes for string literals (e.g. `"scipy"`).
pub fn keyword_arg_value<'a>(call_node: Node<'_>, source: &'a [u8], kw: &str) -> Option<&'a str> {
    let mut cursor = call_node.walk();
    for child in call_node.children(&mut cursor) {
        if child.kind() == "argument_list" {
            let mut arg_cursor = child.walk();
            for arg in child.children(&mut arg_cursor) {
                if arg.kind() == "keyword_argument" {
                    if let Some(name_node) = arg.child_by_field_name("name") {
                        if node_text(&name_node, source) == kw {
                            if let Some(val_node) = arg.child_by_field_name("value") {
                                return Some(node_text(&val_node, source));
                            }
                        }
                    }
                }
            }
        }
    }
    None
}
