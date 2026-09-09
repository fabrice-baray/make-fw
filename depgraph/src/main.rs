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
//!   - Paths recorded inside .d files may contain an unexpanded build
//!     variable such as `$(ROOT)/subfolder/file.h` (gcc records exactly
//!     what was on its command line, and some build systems pass paths
//!     that still contain a make variable at that point). depgraph never
//!     needs to know what `$(ROOT)` actually expands to: since every
//!     dependency path is guaranteed to live somewhere below it, a file's
//!     top-level folder is identified by looking for one of the known
//!     first-level subfolder *names* among the path's components, rather
//!     than by resolving the path to an absolute filesystem location.
//!   - A dependency where target and prerequisite fall in the same
//!     first-level subfolder is ignored (it's an internal edge, not a
//!     between-folder edge).
//!   - A dependency whose path contains none of the known first-level
//!     subfolder names (e.g. a system header like /usr/include/stdio.h) is
//!     ignored.

use std::collections::HashSet;
use std::env;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
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

    let nodes = match first_level_subfolders(&config.root) {
        Ok(n) => n,
        Err(e) => {
            eprintln!("Error reading '{}': {e}", config.root.display());
            process::exit(1);
        }
    };

    if nodes.is_empty() {
        eprintln!(
            "Warning: no first-level subfolders found under '{}'",
            config.root.display()
        );
    }

    let dep_files = find_dep_files(&config.root);
    if config.verbose {
        eprintln!("Found {} dependency file(s)", dep_files.len());
    }

    let mut edges: HashSet<(String, String)> = HashSet::new();
    let mut parse_errors = 0usize;

    for dep_file in &dep_files {
        match parse_dep_file(dep_file) {
            Ok(pairs) => {
                for (target_raw, prereq_raw) in pairs {
                    let target_folder = classify(&target_raw, &nodes);
                    let prereq_folder = classify(&prereq_raw, &nodes);

                    match (target_folder, prereq_folder) {
                        (Some(tf), Some(pf)) if tf != pf => {
                            let is_new = !edges.contains(&(tf.clone(), pf.clone()));
                            edges.insert((tf.clone(), pf.clone()));
                            if config.verbose && is_new {
                                eprintln!(
                                    "edge: {tf} -> {pf}  ({target_raw} -> {prereq_raw})"
                                );
                            }
                        }
                        // Same folder, or one/both sides didn't match any
                        // known node name: ignored silently, per spec.
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
/// foo.o: $(ROOT)/moduleA/foo.c $(ROOT)/moduleA/foo.h \
///   $(ROOT)/moduleB/bar.h
/// $(ROOT)/moduleA/foo.h:
/// $(ROOT)/moduleB/bar.h:
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

/// Determines which known first-level node folder a raw dependency path
/// belongs to.
///
/// Rather than resolving `raw_path` to an absolute filesystem path (which
/// would require knowing the value of any build variable like `$(ROOT)` it
/// might still contain), this looks for one of the known node names among
/// the path's directory components. Since every dependency path is
/// guaranteed to live somewhere below such a variable, matching by name is
/// sufficient and avoids needing the variable's actual value at all.
///
/// The left-most matching component wins, and the final component (the
/// filename itself) is never considered a folder match.
fn classify(raw_path: &str, nodes: &[String]) -> Option<String> {
    let components: Vec<&str> = raw_path
        .split(['/', '\\'])
        .filter(|c| !c.is_empty())
        .collect();

    if components.len() < 2 {
        return None; // no directory component at all, just a bare filename
    }

    for component in &components[..components.len() - 1] {
        if let Some(node) = nodes.iter().find(|n| n.as_str() == *component) {
            return Some(node.clone());
        }
    }
    None
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
        let tokens = split_make_tokens(" $(ROOT)/a/b.c $(ROOT)/a/b.h ");
        assert_eq!(tokens, vec!["$(ROOT)/a/b.c", "$(ROOT)/a/b.h"]);
    }

    #[test]
    fn splits_tokens_with_escaped_space() {
        let tokens = split_make_tokens(r"$(ROOT)/my\ file.c $(ROOT)/b.h");
        assert_eq!(tokens, vec!["$(ROOT)/my file.c", "$(ROOT)/b.h"]);
    }

    #[test]
    fn finds_colon_ignoring_windows_drive_letter() {
        assert_eq!(find_rule_colon("foo.o: a.c"), Some(5));
        assert_eq!(find_rule_colon("C:\\proj\\foo.o: C:\\proj\\a.c"), Some(13));
    }

    #[test]
    fn classify_finds_node_name_after_root_variable() {
        let nodes = vec!["moduleA".to_string(), "moduleB".to_string()];
        assert_eq!(
            classify("$(ROOT)/moduleA/src/a.c", &nodes),
            Some("moduleA".to_string())
        );
        assert_eq!(
            classify("${ROOT}/moduleB/inc/b.h", &nodes),
            Some("moduleB".to_string())
        );
    }

    #[test]
    fn classify_finds_node_name_at_arbitrary_depth() {
        // $(ROOT) can be several levels above the scanned folder; the node
        // name just needs to appear somewhere among the components.
        let nodes = vec!["moduleA".to_string()];
        assert_eq!(
            classify("$(ROOT)/some/ancestor/path/moduleA/src/a.c", &nodes),
            Some("moduleA".to_string())
        );
    }

    #[test]
    fn classify_ignores_paths_with_no_known_node() {
        let nodes = vec!["moduleA".to_string(), "moduleB".to_string()];
        assert_eq!(classify("/usr/include/stdio.h", &nodes), None);
    }

    #[test]
    fn classify_does_not_match_the_filename_itself() {
        // A file literally named "moduleA" (no extension) must not be
        // mistaken for the folder "moduleA": since "other" isn't a known
        // node, this should fall through to None rather than matching the
        // filename component "moduleA".
        let nodes = vec!["moduleA".to_string()];
        assert_eq!(classify("$(ROOT)/other/moduleA", &nodes), None);
    }

    #[test]
    fn parses_multiline_rule_with_continuations() {
        let dir = std::env::temp_dir().join("depgraph_test_parse");
        let _ = fs::create_dir_all(&dir);
        let dep_path = dir.join("foo.d");
        fs::write(
            &dep_path,
            "$(ROOT)/moduleA/foo.o: $(ROOT)/moduleA/foo.c $(ROOT)/moduleA/foo.h \\\n  $(ROOT)/moduleB/bar.h\n$(ROOT)/moduleB/bar.h:\n",
        )
        .unwrap();

        let pairs = parse_dep_file(&dep_path).unwrap();
        assert!(pairs.contains(&(
            "$(ROOT)/moduleA/foo.o".to_string(),
            "$(ROOT)/moduleA/foo.c".to_string()
        )));
        assert!(pairs.contains(&(
            "$(ROOT)/moduleA/foo.o".to_string(),
            "$(ROOT)/moduleB/bar.h".to_string()
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
