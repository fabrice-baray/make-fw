//! depgraph
//!
//! Walks a folder tree, reads every gcc-generated `.d` dependency file it
//! finds, and produces a Graphviz `.dot` graph whose nodes are the
//! first-level subfolders of the input tree. An edge A -> B means at least
//! one file under A depends on (includes) a file under B. If dependencies
//! exist in both directions between two folders, that pair is drawn as a
//! single red bidirectional edge instead of two black ones.
//!
//! Usage:
//!     depgraph <input_folder> [output.dot] [--verbose]
//!
//! Assumptions (see README.md for details):
//!   - Paths recorded inside .d files are absolute (as confirmed by the
//!     project owner). Relative paths are still handled as a fallback,
//!     resolved against the directory containing the .d file.
//!   - A dependency where target and prerequisite fall in the same
//!     first-level subfolder is ignored (it's an internal edge, not a
//!     between-folder edge).
//!   - A dependency pointing outside the input tree entirely (e.g. a system
//!     header like /usr/include/stdio.h) is ignored.

use std::collections::HashSet;
use std::env;
use std::fs;
use std::io;
use std::path::{Component, Path, PathBuf};
use std::process;

struct Config {
    root: PathBuf,
    output_path: PathBuf,
    verbose: bool,
}

fn main() {
    let config = match parse_args() {
        Ok(c) => c,
        Err(msg) => {
            eprintln!("{msg}");
            print_usage();
            process::exit(1);
        }
    };

    if !config.root.is_dir() {
        eprintln!("Error: '{}' is not a directory", config.root.display());
        process::exit(1);
    }

    let root = fs::canonicalize(&config.root).unwrap_or_else(|e| {
        eprintln!(
            "Error: could not canonicalize '{}': {e}",
            config.root.display()
        );
        process::exit(1);
    });

    let nodes = match first_level_subfolders(&root) {
        Ok(n) => n,
        Err(e) => {
            eprintln!("Error reading '{}': {e}", root.display());
            process::exit(1);
        }
    };

    if nodes.is_empty() {
        eprintln!(
            "Warning: no first-level subfolders found under '{}'",
            root.display()
        );
    }

    let dep_files = find_dep_files(&root);
    if config.verbose {
        eprintln!("Found {} dependency file(s)", dep_files.len());
    }

    let mut edges: HashSet<(String, String)> = HashSet::new();
    let mut parse_errors = 0usize;

    for dep_file in &dep_files {
        match parse_dep_file(dep_file) {
            Ok(pairs) => {
                for (target_raw, prereq_raw) in pairs {
                    let target_abs = resolve_path(&target_raw, dep_file);
                    let prereq_abs = resolve_path(&prereq_raw, dep_file);

                    let target_folder = top_level_folder(&target_abs, &root);
                    let prereq_folder = top_level_folder(&prereq_abs, &root);

                    match (target_folder, prereq_folder) {
                        (Some(tf), Some(pf)) if tf != pf => {
                            if config.verbose && edges.insert((tf.clone(), pf.clone())) {
                                eprintln!("edge: {tf} -> {pf}  ({} -> {})", target_raw, prereq_raw);
                            } else {
                                edges.insert((tf, pf));
                            }
                        }
                        // Same folder, or one/both sides outside the tree:
                        // ignored silently, per spec.
                        _ => {}
                    }
                }
            }
            Err(e) => {
                eprintln!("Warning: failed to parse '{}': {e}", dep_file.display());
                parse_errors += 1;
            }
        }
    }

    if parse_errors > 0 {
        eprintln!("{parse_errors} file(s) failed to parse");
    }

    let dot = render_dot(&nodes, &edges);

    if let Err(e) = fs::write(&config.output_path, &dot) {
        eprintln!(
            "Error: could not write '{}': {e}",
            config.output_path.display()
        );
        process::exit(1);
    }

    println!(
        "Graph written to '{}' ({} node(s), {} edge(s) before merging)",
        config.output_path.display(),
        nodes.len(),
        edges.len()
    );
}

fn print_usage() {
    eprintln!("Usage: depgraph <input_folder> [output.dot] [--verbose]");
}

fn parse_args() -> Result<Config, String> {
    let mut args: Vec<String> = env::args().skip(1).collect();

    let verbose = if let Some(pos) = args.iter().position(|a| a == "--verbose" || a == "-v") {
        args.remove(pos);
        true
    } else {
        false
    };

    if args.is_empty() {
        return Err("Error: missing <input_folder> argument".to_string());
    }

    let root = PathBuf::from(&args[0]);
    let output_path = args
        .get(1)
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("graph.dot"));

    Ok(Config {
        root,
        output_path,
        verbose,
    })
}

/// Names of directories directly under `root` (not recursive).
fn first_level_subfolders(root: &Path) -> io::Result<Vec<String>> {
    let mut result = Vec::new();
    for entry in fs::read_dir(root)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                result.push(name.to_string());
            }
        }
    }
    result.sort();
    Ok(result)
}

/// Recursively collects every file ending in `.d` under `root`.
fn find_dep_files(root: &Path) -> Vec<PathBuf> {
    let mut result = Vec::new();
    let mut stack = vec![root.to_path_buf()];
    while let Some(dir) = stack.pop() {
        let entries = match fs::read_dir(&dir) {
            Ok(e) => e,
            Err(_) => continue,
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                stack.push(path);
            } else if path.extension().and_then(|e| e.to_str()) == Some("d") {
                result.push(path);
            }
        }
    }
    result
}

/// Parses a gcc-generated `.d` file into (target, prerequisite) pairs.
///
/// Format looks like:
/// ```text
/// foo.o: /abs/path/foo.c /abs/path/foo.h \
///   /abs/path/bar.h
/// /abs/path/foo.h:
/// /abs/path/bar.h:
/// ```
/// (the trailing empty rules come from `-MP` and are harmless here since
/// they have no prerequisites).
fn parse_dep_file(path: &Path) -> io::Result<Vec<(String, String)>> {
    let content = fs::read_to_string(path)?;

    // Join Makefile line continuations ("\" at end of line) into one
    // logical line before splitting into rules.
    let joined = content.replace("\\\r\n", " ").replace("\\\n", " ");

    let mut pairs = Vec::new();

    for line in joined.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        if let Some(colon_pos) = find_rule_colon(line) {
            let (targets_str, rest) = line.split_at(colon_pos);
            let prereqs_str = &rest[1..]; // skip the ':'

            let targets = split_make_tokens(targets_str);
            let prereqs = split_make_tokens(prereqs_str);

            for t in &targets {
                for p in &prereqs {
                    pairs.push((t.clone(), p.clone()));
                }
            }
        }
    }

    Ok(pairs)
}

/// Finds the ':' that separates targets from prerequisites on a rule line,
/// skipping a Windows drive-letter colon (e.g. "C:\foo") if present.
fn find_rule_colon(line: &str) -> Option<usize> {
    let bytes = line.as_bytes();
    for (i, &b) in bytes.iter().enumerate() {
        if b == b':' {
            let looks_like_drive_letter = i == 1 && bytes[0].is_ascii_alphabetic();
            let next_is_slash = bytes.get(i + 1).map_or(false, |&c| c == b'/' || c == b'\\');
            if looks_like_drive_letter && next_is_slash {
                continue;
            }
            return Some(i);
        }
    }
    None
}

/// Splits whitespace-separated Makefile tokens, honoring backslash-escaped
/// spaces (e.g. `foo\ bar.h` is one filename, not two tokens).
fn split_make_tokens(s: &str) -> Vec<String> {
    let mut tokens = Vec::new();
    let mut current = String::new();
    let mut chars = s.chars().peekable();

    while let Some(c) = chars.next() {
        match c {
            '\\' if chars.peek() == Some(&' ') => {
                current.push(' ');
                chars.next();
            }
            c if c.is_whitespace() => {
                if !current.is_empty() {
                    tokens.push(std::mem::take(&mut current));
                }
            }
            c => current.push(c),
        }
    }
    if !current.is_empty() {
        tokens.push(current);
    }
    tokens
}

/// Resolves a path recorded in a .d file. Per project spec these are
/// expected to already be absolute; relative paths (should they occur) are
/// resolved against the directory containing the .d file as a best-effort
/// fallback.
fn resolve_path(raw: &str, dep_file: &Path) -> PathBuf {
    let p = Path::new(raw);
    let candidate = if p.is_absolute() {
        p.to_path_buf()
    } else {
        dep_file
            .parent()
            .map(|d| d.join(p))
            .unwrap_or_else(|| p.to_path_buf())
    };
    normalize(&candidate)
}

/// Lexically normalizes `.` and `..` components without requiring the path
/// to exist on disk (headers referenced in a .d file may have since been
/// deleted or moved).
fn normalize(path: &Path) -> PathBuf {
    let mut result = PathBuf::new();
    for component in path.components() {
        match component {
            Component::ParentDir => {
                result.pop();
            }
            Component::CurDir => {}
            other => result.push(other.as_os_str()),
        }
    }
    result
}

/// Returns the first-level subfolder name of `root` that `path` lives
/// under, or `None` if `path` is not inside `root` at all (e.g. a system
/// header).
fn top_level_folder(path: &Path, root: &Path) -> Option<String> {
    let rel = path.strip_prefix(root).ok()?;
    let mut components = rel.components();
    let first = components.next()?;
    // If path == root exactly there is no subfolder component.
    Some(first.as_os_str().to_string_lossy().to_string())
}

fn escape(s: &str) -> String {
    s.replace('\\', "\\\\").replace('"', "\\\"")
}

/// Renders the final dot file: all nodes declared up front, then edges,
/// merging any A<->B pair that exists in both directions into a single red
/// bidirectional edge.
fn render_dot(nodes: &[String], edges: &HashSet<(String, String)>) -> String {
    let mut dot = String::new();
    dot.push_str("digraph dependencies {\n");
    dot.push_str("    rankdir=LR;\n");
    dot.push_str("    node [shape=box];\n\n");

    for node in nodes {
        dot.push_str(&format!("    \"{}\";\n", escape(node)));
    }
    dot.push('\n');

    let mut drawn: HashSet<(String, String)> = HashSet::new();

    // Iterate in a stable order for reproducible output.
    let mut sorted_edges: Vec<&(String, String)> = edges.iter().collect();
    sorted_edges.sort();

    for (a, b) in sorted_edges {
        if drawn.contains(&(a.clone(), b.clone())) || drawn.contains(&(b.clone(), a.clone())) {
            continue;
        }
        let reverse_exists = edges.contains(&(b.clone(), a.clone()));
        if reverse_exists {
            dot.push_str(&format!(
                "    \"{}\" -> \"{}\" [color=red, dir=both];\n",
                escape(a),
                escape(b)
            ));
            drawn.insert((a.clone(), b.clone()));
            drawn.insert((b.clone(), a.clone()));
        } else {
            dot.push_str(&format!("    \"{}\" -> \"{}\";\n", escape(a), escape(b)));
            drawn.insert((a.clone(), b.clone()));
        }
    }

    dot.push_str("}\n");
    dot
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn splits_simple_tokens() {
        let tokens = split_make_tokens(" /a/b.c /a/b.h ");
        assert_eq!(tokens, vec!["/a/b.c", "/a/b.h"]);
    }

    #[test]
    fn splits_tokens_with_escaped_space() {
        let tokens = split_make_tokens(r"/a/my\ file.c /a/b.h");
        assert_eq!(tokens, vec!["/a/my file.c", "/a/b.h"]);
    }

    #[test]
    fn finds_colon_ignoring_windows_drive_letter() {
        assert_eq!(find_rule_colon("foo.o: a.c"), Some(5));
        assert_eq!(find_rule_colon("C:\\proj\\foo.o: C:\\proj\\a.c"), Some(13));
    }

    #[test]
    fn top_level_folder_detects_correct_subfolder() {
        let root = Path::new("/proj");
        assert_eq!(
            top_level_folder(Path::new("/proj/moduleA/src/a.c"), root),
            Some("moduleA".to_string())
        );
        assert_eq!(top_level_folder(Path::new("/usr/include/stdio.h"), root), None);
        assert_eq!(top_level_folder(Path::new("/proj"), root), None);
    }

    #[test]
    fn normalize_collapses_parent_dirs() {
        assert_eq!(
            normalize(Path::new("/proj/moduleA/../moduleB/x.h")),
            PathBuf::from("/proj/moduleB/x.h")
        );
    }

    #[test]
    fn parses_multiline_rule_with_continuations() {
        let dir = std::env::temp_dir().join("depgraph_test_parse");
        let _ = fs::create_dir_all(&dir);
        let dep_path = dir.join("foo.d");
        fs::write(
            &dep_path,
            "/proj/moduleA/foo.o: /proj/moduleA/foo.c /proj/moduleA/foo.h \\\n  /proj/moduleB/bar.h\n/proj/moduleB/bar.h:\n",
        )
        .unwrap();

        let pairs = parse_dep_file(&dep_path).unwrap();
        assert!(pairs.contains(&(
            "/proj/moduleA/foo.o".to_string(),
            "/proj/moduleA/foo.c".to_string()
        )));
        assert!(pairs.contains(&(
            "/proj/moduleA/foo.o".to_string(),
            "/proj/moduleB/bar.h".to_string()
        )));

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn bidirectional_pair_rendered_red_once() {
        let nodes = vec!["A".to_string(), "B".to_string()];
        let mut edges = HashSet::new();
        edges.insert(("A".to_string(), "B".to_string()));
        edges.insert(("B".to_string(), "A".to_string()));

        let dot = render_dot(&nodes, &edges);
        assert_eq!(dot.matches("color=red").count(), 1);
        assert_eq!(dot.matches("->").count(), 1);
    }

    #[test]
    fn one_directional_pair_rendered_black() {
        let nodes = vec!["A".to_string(), "B".to_string()];
        let mut edges = HashSet::new();
        edges.insert(("A".to_string(), "B".to_string()));

        let dot = render_dot(&nodes, &edges);
        assert_eq!(dot.matches("color=red").count(), 0);
        assert!(dot.contains("\"A\" -> \"B\";"));
    }
}
