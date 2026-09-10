//! depgraph
//!
//! Walks a folder tree, reads every gcc-generated `.d` dependency file it
//! finds, and produces a Graphviz `.dot` graph of folder-to-folder
//! dependencies. An edge A -> B means at least one file under A depends on
//! (includes) a file under B. If dependencies exist in both directions
//! between two folders, that pair is drawn as a single red bidirectional
//! edge instead of two black ones.
//!
//! Usage:
//!     depgraph <input_folder> [output.dot] [--verbose] [--reduce]
//!              [--level N] [--edge-mode flat|hierarchical]
//!
//! ## Levels
//!
//! `--level N` (default 1) controls how many folder levels are broken out.
//! At `--level 1` (the original behavior), only first-level subfolders are
//! nodes. At `--level 2`, any first-level folder that itself has
//! subdirectories is drawn as a Graphviz subgraph cluster, with its
//! immediate children as the nodes inside it; `--level 3` allows another
//! level of nesting inside those, and so on. A dependency on a file that
//! sits directly inside an exploded (clustered) folder, outside any of its
//! recognized child subfolders, is dropped rather than attributed to the
//! folder itself.
//!
//! ## Edge modes
//!
//! `--edge-mode` controls how a cross-folder dependency is placed once
//! folders are broken into multiple levels:
//!   - `flat` (default): the edge connects the deepest known folder on
//!     each side directly, however far apart they are in the hierarchy.
//!   - `hierarchical`: the edge is drawn between the two folders at the
//!     point where their paths first diverge. A dependency from
//!     `A/sub1/file` to `B/sub2/file` is drawn as `A -> B` (they diverge
//!     immediately). A dependency from `A/sub1/file` to `A/sub2/file` is
//!     drawn as `A/sub1 -> A/sub2`, nested inside cluster `A` (they share
//!     `A` and diverge one level down). See [`hierarchical_edge`].
//!
//! Assumptions (see README.md for details):
//!   - Paths recorded inside .d files may contain an unexpanded build
//!     variable such as `$(ROOT)/subfolder/file.h` (gcc records exactly
//!     what was on its command line, and some build systems pass paths
//!     that still contain a make variable at that point). depgraph never
//!     needs to know what `$(ROOT)` actually expands to: since every
//!     dependency path is guaranteed to live somewhere below it, a file's
//!     folder is identified by looking for known subfolder *names* among
//!     the path's components, rather than by resolving the path to an
//!     absolute filesystem location.
//!   - A dependency where target and prerequisite fall in the same folder
//!     (at the deepest level considered) is ignored.
//!   - A dependency whose path contains none of the known subfolder names
//!     (e.g. a system header like /usr/include/stdio.h) is ignored.
//!   - `--reduce` applies a transitive reduction to the edge set before
//!     rendering: an edge A -> B is dropped if B is still reachable from A
//!     through some other path of edges. See [`transitive_reduction`] for
//!     the cycle caveat.

use std::collections::{HashMap, HashSet};
use std::env;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::process;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum EdgeMode {
    Flat,
    Hierarchical,
}

struct Config {
    root: PathBuf,
    output_path: PathBuf,
    verbose: bool,
    reduce: bool,
    level: usize,
    edge_mode: EdgeMode,
}

/// A folder in the tree being turned into graph nodes/clusters.
///
/// `children` is non-empty only when this folder has real subdirectories
/// on disk *and* `--level` allows descending further; such a folder is
/// rendered as a Graphviz subgraph cluster containing its children instead
/// of as a plain node.
#[derive(Debug, Clone)]
struct FolderNode {
    name: String,
    path_id: String,
    children: Vec<FolderNode>,
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

    let tree = match build_folder_tree(&config.root, config.level) {
        Ok(t) => t,
        Err(e) => {
            eprintln!("Error reading '{}': {e}", config.root.display());
            process::exit(1);
        }
    };

    if tree.is_empty() {
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
                let file_edges = edges_from_pairs(&pairs, &tree, config.edge_mode);
                for edge in file_edges {
                    let is_new = edges.insert(edge.clone());
                    if config.verbose && is_new {
                        eprintln!(
                            "edge: {} -> {}  (from {})",
                            edge.0,
                            edge.1,
                            dep_file.display()
                        );
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

    let edges_before_reduction = edges.len();
    if config.reduce {
        edges = transitive_reduction(&edges);
        if config.verbose {
            eprintln!(
                "Transitive reduction: {} edge(s) -> {} edge(s)",
                edges_before_reduction,
                edges.len()
            );
        }
    }

    let dot = render_dot(&tree, &edges, config.edge_mode);

    if let Err(e) = fs::write(&config.output_path, &dot) {
        eprintln!(
            "Error: could not write '{}': {e}",
            config.output_path.display()
        );
        process::exit(1);
    }

    println!(
        "Graph written to '{}' ({} top-level folder(s), {} leaf node(s), {} edge(s){})",
        config.output_path.display(),
        tree.len(),
        count_leaf_nodes(&tree),
        edges.len(),
        if config.reduce {
            format!(" after reduction from {edges_before_reduction}")
        } else {
            String::new()
        }
    );
}

fn print_usage() {
    eprintln!(
        "Usage: depgraph <input_folder> [output.dot] [--verbose] [--reduce] \
         [--level N] [--edge-mode flat|hierarchical]"
    );
    eprintln!(
        "  --reduce               Apply a transitive reduction: drop an edge A -> B if B is\n\
         still reachable from A through some other path of edges."
    );
    eprintln!(
        "  --level N              How many folder levels to break out (default 1). A folder\n\
         with subdirectories is drawn as a cluster containing its children when N allows it."
    );
    eprintln!(
        "  --edge-mode MODE       'flat' (default) connects the deepest known folder on each\n\
         side directly; 'hierarchical' draws the edge where the two paths first diverge."
    );
}

fn parse_args() -> Result<Config, String> {
    let mut args: Vec<String> = env::args().skip(1).collect();

    let verbose = if let Some(pos) = args.iter().position(|a| a == "--verbose" || a == "-v") {
        args.remove(pos);
        true
    } else {
        false
    };

    let reduce = if let Some(pos) = args.iter().position(|a| a == "--reduce") {
        args.remove(pos);
        true
    } else {
        false
    };

    let level = if let Some(pos) = args.iter().position(|a| a == "--level") {
        if pos + 1 >= args.len() {
            return Err("Error: --level requires a positive integer argument".to_string());
        }
        let value = args.remove(pos + 1);
        args.remove(pos);
        value
            .parse::<usize>()
            .ok()
            .filter(|&n| n >= 1)
            .ok_or_else(|| format!("Error: --level expects a positive integer, got '{value}'"))?
    } else {
        1
    };

    let edge_mode = if let Some(pos) = args.iter().position(|a| a == "--edge-mode") {
        if pos + 1 >= args.len() {
            return Err("Error: --edge-mode requires 'flat' or 'hierarchical'".to_string());
        }
        let value = args.remove(pos + 1);
        args.remove(pos);
        match value.as_str() {
            "flat" => EdgeMode::Flat,
            "hierarchical" => EdgeMode::Hierarchical,
            other => {
                return Err(format!(
                    "Error: --edge-mode expects 'flat' or 'hierarchical', got '{other}'"
                ))
            }
        }
    } else {
        EdgeMode::Flat
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
        reduce,
        level,
        edge_mode,
    })
}

/// Builds the folder tree used for graph nodes/clusters, descending up to
/// `level` folder levels deep. A folder becomes a leaf (no `children`)
/// either because it has no subdirectories on disk, or because `level`
/// doesn't allow descending any further.
fn build_folder_tree(root: &Path, level: usize) -> io::Result<Vec<FolderNode>> {
    build_level(root, "", level)
}

fn build_level(dir: &Path, path_prefix: &str, remaining_depth: usize) -> io::Result<Vec<FolderNode>> {
    let mut result = Vec::new();
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                let path_id = if path_prefix.is_empty() {
                    name.to_string()
                } else {
                    format!("{path_prefix}/{name}")
                };
                let children = if remaining_depth > 1 {
                    build_level(&path, &path_id, remaining_depth - 1)?
                } else {
                    Vec::new()
                };
                result.push(FolderNode {
                    name: name.to_string(),
                    path_id,
                    children,
                });
            }
        }
    }
    result.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(result)
}

fn count_leaf_nodes(tree: &[FolderNode]) -> usize {
    tree.iter()
        .map(|n| {
            if n.children.is_empty() {
                1
            } else {
                count_leaf_nodes(&n.children)
            }
        })
        .sum()
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

/// Determines the full ancestor chain of known folders a raw dependency
/// path belongs to, e.g. `["moduleA", "moduleA/sub1"]`.
///
/// Rather than resolving `raw_path` to an absolute filesystem path (which
/// would require knowing the value of any build variable like `$(ROOT)` it
/// might still contain), this looks for a known folder *name* among the
/// path's directory components, then keeps descending into that folder's
/// known children for as long as they keep matching. Since every
/// dependency path is guaranteed to live somewhere below such a variable,
/// matching by name is sufficient and avoids needing the variable's actual
/// value at all.
///
/// Returns `None` if no known top-level folder name appears at all, or if
/// descent stops at a folder that has children (it's rendered as a
/// cluster) but the next path component doesn't match any of them — i.e.
/// the file lives directly in that folder, outside any recognized child,
/// which is dropped rather than attributed to the folder itself.
fn classify_chain(raw_path: &str, tree: &[FolderNode]) -> Option<Vec<String>> {
    let components: Vec<&str> = raw_path
        .split(['/', '\\'])
        .filter(|c| !c.is_empty())
        .collect();

    if components.len() < 2 {
        return None; // no directory component at all, just a bare filename
    }
    let dir_components = &components[..components.len() - 1];

    // Find the left-most directory component matching a root-level folder.
    let mut start = None;
    let mut current: Option<&FolderNode> = None;
    for (i, component) in dir_components.iter().enumerate() {
        if let Some(node) = tree.iter().find(|n| n.name == *component) {
            start = Some(i);
            current = Some(node);
            break;
        }
    }
    let mut current = current?;
    let mut chain = vec![current.path_id.clone()];
    let mut next_index = start.unwrap() + 1;

    loop {
        if current.children.is_empty() {
            return Some(chain);
        }
        match dir_components.get(next_index) {
            Some(component) => match current.children.iter().find(|c| c.name == *component) {
                Some(child) => {
                    current = child;
                    chain.push(current.path_id.clone());
                    next_index += 1;
                }
                None => return None, // lives directly in an exploded folder: dropped
            },
            None => return None, // ran out of components before reaching a leaf: dropped
        }
    }
}

/// Converts the (target, prerequisite) pairs returned by [`parse_dep_file`]
/// for a single `.d` file into a set of edges, per `mode`.
fn edges_from_pairs(
    pairs: &[(String, String)],
    tree: &[FolderNode],
    mode: EdgeMode,
) -> HashSet<(String, String)> {
    let mut edges = HashSet::new();
    for (target_raw, prereq_raw) in pairs {
        let target_chain = classify_chain(target_raw, tree);
        let prereq_chain = classify_chain(prereq_raw, tree);
        if let (Some(tc), Some(pc)) = (target_chain, prereq_chain) {
            let edge = match mode {
                EdgeMode::Flat => {
                    let tf = tc.last().expect("chain is never empty");
                    let pf = pc.last().expect("chain is never empty");
                    if tf != pf {
                        Some((tf.clone(), pf.clone()))
                    } else {
                        None
                    }
                }
                EdgeMode::Hierarchical => hierarchical_edge(&tc, &pc),
            };
            if let Some(e) = edge {
                edges.insert(e);
            }
        }
    }
    edges
}

/// Given the full ancestor chains of a target and a prerequisite, finds
/// where the two chains first diverge and returns the pair of folders at
/// that point — i.e. the closest common ancestor's two differing children.
/// Returns `None` if the chains are identical throughout (same folder on
/// both sides) or if one chain is entirely a prefix of the other (which
/// shouldn't occur given how chains are built, but is treated as "no
/// edge" rather than panicking).
fn hierarchical_edge(target_chain: &[String], prereq_chain: &[String]) -> Option<(String, String)> {
    let mut i = 0;
    while i < target_chain.len() && i < prereq_chain.len() && target_chain[i] == prereq_chain[i] {
        i += 1;
    }
    if i < target_chain.len() && i < prereq_chain.len() {
        let a = &target_chain[i];
        let b = &prereq_chain[i];
        if a != b {
            return Some((a.clone(), b.clone()));
        }
    }
    None
}

/// The folder that should textually enclose an edge's declaration in the
/// rendered dot file: the deepest common ancestor of its two endpoint
/// path ids, or `None` for the top level if they share no ancestor.
fn common_ancestor_scope(a: &str, b: &str) -> Option<String> {
    let a_parts: Vec<&str> = a.split('/').collect();
    let b_parts: Vec<&str> = b.split('/').collect();
    let mut common = Vec::new();
    for (x, y) in a_parts.iter().zip(b_parts.iter()) {
        if x == y {
            common.push(*x);
        } else {
            break;
        }
    }
    if common.is_empty() {
        None
    } else {
        Some(common.join("/"))
    }
}

/// Applies a transitive reduction to a directed edge set: an edge A -> B is
/// dropped whenever B is still reachable from A using some other path
/// through the remaining edges, since such an edge adds no reachability
/// information beyond what the rest of the graph already implies.
///
/// For a DAG this produces the unique minimal graph with the same
/// reachability relation as the input. **Caveat:** this folder graph can
/// contain cycles (most simply, a bidirectional pair A <-> B). A graph with
/// cycles has no single well-defined minimal reduction, so edges are
/// processed in a fixed, deterministic (sorted) order and each is dropped
/// greedily if it's currently redundant; the result is always a valid
/// reduction (same reachability as the input) but, inside a cycle, which
/// specific edges survive can depend on that processing order. A minimal
/// cycle with no shortcut edges (e.g. a lone A <-> B pair with no other
/// path between them) is always left untouched, since dropping either
/// direction would break reachability.
fn transitive_reduction(edges: &HashSet<(String, String)>) -> HashSet<(String, String)> {
    let mut adjacency: HashMap<String, HashSet<String>> = HashMap::new();
    for (a, b) in edges {
        adjacency.entry(a.clone()).or_default().insert(b.clone());
    }

    // Deterministic order so the result doesn't depend on hash iteration.
    let mut sorted_edges: Vec<(String, String)> = edges.iter().cloned().collect();
    sorted_edges.sort();

    for (u, v) in sorted_edges {
        if let Some(successors) = adjacency.get_mut(&u) {
            successors.remove(&v);
        }
        if !is_reachable(&adjacency, &u, &v) {
            // Removing this edge broke reachability: it was load-bearing,
            // so put it back.
            adjacency.entry(u).or_default().insert(v);
        }
        // Otherwise leave it removed: some other path already covers it.
    }

    let mut result = HashSet::new();
    for (u, successors) in &adjacency {
        for v in successors {
            result.insert((u.clone(), v.clone()));
        }
    }
    result
}

/// Depth-first reachability check: is `target` reachable from `start`
/// using the edges currently in `adjacency`?
fn is_reachable(adjacency: &HashMap<String, HashSet<String>>, start: &str, target: &str) -> bool {
    let mut visited: HashSet<&str> = HashSet::new();
    let mut stack: Vec<&str> = vec![start];
    visited.insert(start);

    while let Some(node) = stack.pop() {
        if let Some(successors) = adjacency.get(node) {
            for next in successors {
                if next == target {
                    return true;
                }
                if visited.insert(next.as_str()) {
                    stack.push(next.as_str());
                }
            }
        }
    }
    false
}

fn escape(s: &str) -> String {
    s.replace('\\', "\\\\").replace('"', "\\\"")
}

/// Sanitizes a path id into a valid Graphviz cluster identifier (letters,
/// digits and underscores only).
fn sanitize_id(s: &str) -> String {
    s.chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '_' })
        .collect()
}

/// Groups edges by which folder should textually enclose them in the
/// rendered dot file. In `Flat` mode every edge is placed at the top
/// level, since its two endpoints may not share any common ancestor
/// cluster. In `Hierarchical` mode, by construction, both endpoints of an
/// edge are siblings directly under [`common_ancestor_scope`].
fn group_edges_by_scope(
    edges: &HashSet<(String, String)>,
    mode: EdgeMode,
) -> HashMap<Option<String>, Vec<(String, String)>> {
    let mut map: HashMap<Option<String>, Vec<(String, String)>> = HashMap::new();
    for (a, b) in edges {
        let scope = match mode {
            EdgeMode::Flat => None,
            EdgeMode::Hierarchical => common_ancestor_scope(a, b),
        };
        map.entry(scope).or_default().push((a.clone(), b.clone()));
    }
    map
}

/// Renders the final dot file: nodes and nested subgraph clusters from the
/// folder tree, then edges placed per [`group_edges_by_scope`], merging any
/// A<->B pair that exists in both directions into a single red
/// bidirectional edge instead of two black ones. An edge whose two
/// endpoints sit under different top-level folders gets `minlen=0` plus
/// `ltail`/`lhead` pointing at each side's top-level cluster (when that
/// side is actually exploded into one), so it visually terminates at the
/// cluster boundary instead of diving to the specific inner node — see
/// [`cross_cluster_attrs`].
fn render_dot(tree: &[FolderNode], edges: &HashSet<(String, String)>, mode: EdgeMode) -> String {
    let mut dot = String::new();
    dot.push_str("digraph dependencies {\n");
    dot.push_str("    rankdir=TB;\n");
    dot.push_str("    compound=true;\n");
    dot.push_str("    nodesep=.55;\n");
    dot.push_str("    node [shape=ellipse];\n\n");

    let edges_by_scope = group_edges_by_scope(edges, mode);
    let mut drawn: HashSet<(String, String)> = HashSet::new();

    for node in tree {
        render_folder_node(node, &edges_by_scope, edges, tree, &mut drawn, &mut dot, 1);
    }

    if let Some(top_edges) = edges_by_scope.get(&None) {
        render_edges(top_edges, edges, tree, &mut drawn, &mut dot, 1);
    }

    dot.push_str("}\n");
    dot
}

fn render_folder_node(
    node: &FolderNode,
    edges_by_scope: &HashMap<Option<String>, Vec<(String, String)>>,
    all_edges: &HashSet<(String, String)>,
    tree: &[FolderNode],
    drawn: &mut HashSet<(String, String)>,
    dot: &mut String,
    indent: usize,
) {
    let pad = "    ".repeat(indent);
    if node.children.is_empty() {
        dot.push_str(&format!(
            "{pad}\"{}\" [label=\"{}\"];\n",
            escape(&node.path_id),
            escape(&node.name)
        ));
        return;
    }

    dot.push_str(&format!(
        "{pad}subgraph cluster_{} {{\n",
        sanitize_id(&node.path_id)
    ));
    dot.push_str(&format!("{pad}    label=\"{}\";\n", escape(&node.name)));
    for child in &node.children {
        render_folder_node(child, edges_by_scope, all_edges, tree, drawn, dot, indent + 1);
    }
    if let Some(scoped_edges) = edges_by_scope.get(&Some(node.path_id.clone())) {
        render_edges(scoped_edges, all_edges, tree, drawn, dot, indent + 1);
    }
    dot.push_str(&format!("{pad}}}\n"));
}

/// The first path component of a node id, e.g. `"moduleA"` for both
/// `"moduleA"` and `"moduleA/subA1"`.
fn top_level_name(node_id: &str) -> &str {
    node_id.split('/').next().unwrap_or(node_id)
}

/// Whether the named top-level folder is exploded into a cluster (has
/// children in the tree), and therefore has a `cluster_<name>` that a
/// `ltail`/`lhead` attribute can legally point to.
fn is_exploded_top_level(name: &str, tree: &[FolderNode]) -> bool {
    tree.iter().any(|n| n.name == name && !n.children.is_empty())
}

/// Extra edge attributes for an edge that crosses between two different
/// top-level folders where at least one side is exploded into a cluster:
/// `minlen=0`, plus `ltail`/`lhead` naming each exploded side's top-level
/// cluster — always the top-level cluster, even if the actual endpoint is
/// nested deeper (`--level` > 2). Returns `None` when both endpoints
/// share the same top-level folder (the edge stays inside a single
/// cluster and doesn't cross anything), or when neither side is actually
/// a cluster (a plain top-level-to-top-level edge has no boundary to clip
/// to). A side that isn't itself exploded is simply omitted from the
/// attribute list, since there's no cluster name to give it.
fn cross_cluster_attrs(a: &str, b: &str, tree: &[FolderNode]) -> Option<Vec<String>> {
    let ta = top_level_name(a);
    let tb = top_level_name(b);
    if ta == tb {
        return None;
    }
    let ta_exploded = is_exploded_top_level(ta, tree);
    let tb_exploded = is_exploded_top_level(tb, tree);
    if !ta_exploded && !tb_exploded {
        // Neither side is a cluster at all (e.g. plain top-level-to-top-
        // level edge at --level 1): nothing to clip to a boundary.
        return None;
    }
    let mut attrs = vec!["minlen=0".to_string()];
    if ta_exploded {
        attrs.push(format!("ltail=\"cluster_{}\"", sanitize_id(ta)));
    }
    if tb_exploded {
        attrs.push(format!("lhead=\"cluster_{}\"", sanitize_id(tb)));
    }
    Some(attrs)
}

fn render_edges(
    edges: &[(String, String)],
    all_edges: &HashSet<(String, String)>,
    tree: &[FolderNode],
    drawn: &mut HashSet<(String, String)>,
    dot: &mut String,
    indent: usize,
) {
    let pad = "    ".repeat(indent);
    let mut sorted: Vec<&(String, String)> = edges.iter().collect();
    sorted.sort();

    for (a, b) in sorted {
        if drawn.contains(&(a.clone(), b.clone())) || drawn.contains(&(b.clone(), a.clone())) {
            continue;
        }
        let reverse_exists = all_edges.contains(&(b.clone(), a.clone()));

        let mut attrs: Vec<String> = Vec::new();
        if reverse_exists {
            attrs.push("color=red".to_string());
            attrs.push("dir=both".to_string());
        }
        if let Some(cross_attrs) = cross_cluster_attrs(a, b, tree) {
            attrs.extend(cross_attrs);
        }
        let attr_suffix = if attrs.is_empty() {
            String::new()
        } else {
            format!(" [{}]", attrs.join(", "))
        };

        dot.push_str(&format!(
            "{pad}\"{}\" -> \"{}\"{};\n",
            escape(a),
            escape(b),
            attr_suffix
        ));

        if reverse_exists {
            drawn.insert((a.clone(), b.clone()));
            drawn.insert((b.clone(), a.clone()));
        } else {
            drawn.insert((a.clone(), b.clone()));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn leaf(name: &str, path_id: &str) -> FolderNode {
        FolderNode {
            name: name.to_string(),
            path_id: path_id.to_string(),
            children: Vec::new(),
        }
    }

    /// A small two-level fixture:
    ///   moduleA/ (exploded: subA1, subA2)
    ///   moduleB/ (leaf: no subdirectories)
    fn sample_tree() -> Vec<FolderNode> {
        vec![
            FolderNode {
                name: "moduleA".to_string(),
                path_id: "moduleA".to_string(),
                children: vec![
                    leaf("subA1", "moduleA/subA1"),
                    leaf("subA2", "moduleA/subA2"),
                ],
            },
            leaf("moduleB", "moduleB"),
        ]
    }

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
    fn classify_chain_leaf_folder_at_level_1() {
        let tree = sample_tree();
        assert_eq!(
            classify_chain("$(ROOT)/moduleB/inc/b.h", &tree),
            Some(vec!["moduleB".to_string()])
        );
    }

    #[test]
    fn classify_chain_descends_into_exploded_folder() {
        let tree = sample_tree();
        assert_eq!(
            classify_chain("$(ROOT)/moduleA/subA1/src/a.c", &tree),
            Some(vec!["moduleA".to_string(), "moduleA/subA1".to_string()])
        );
    }

    #[test]
    fn classify_chain_drops_loose_file_in_exploded_folder() {
        // moduleA is exploded (has children), but this file sits directly
        // in moduleA, not in subA1 or subA2.
        let tree = sample_tree();
        assert_eq!(classify_chain("$(ROOT)/moduleA/loose.c", &tree), None);
    }

    #[test]
    fn classify_chain_ignores_paths_with_no_known_folder() {
        let tree = sample_tree();
        assert_eq!(classify_chain("/usr/include/stdio.h", &tree), None);
    }

    #[test]
    fn classify_chain_finds_folder_at_arbitrary_ancestor_depth() {
        // $(ROOT) can be several levels above the scanned folder; the
        // folder name just needs to appear somewhere among the components.
        let tree = sample_tree();
        assert_eq!(
            classify_chain("$(ROOT)/some/ancestor/moduleB/x.h", &tree),
            Some(vec!["moduleB".to_string()])
        );
    }

    #[test]
    fn hierarchical_edge_diverges_at_top_level() {
        let target_chain = vec!["A".to_string(), "A/sub1".to_string()];
        let prereq_chain = vec!["B".to_string(), "B/sub2".to_string()];
        assert_eq!(
            hierarchical_edge(&target_chain, &prereq_chain),
            Some(("A".to_string(), "B".to_string()))
        );
    }

    #[test]
    fn hierarchical_edge_diverges_inside_shared_ancestor() {
        let target_chain = vec!["A".to_string(), "A/subA1".to_string()];
        let prereq_chain = vec!["A".to_string(), "A/subA2".to_string()];
        assert_eq!(
            hierarchical_edge(&target_chain, &prereq_chain),
            Some(("A/subA1".to_string(), "A/subA2".to_string()))
        );
    }

    #[test]
    fn hierarchical_edge_none_for_identical_chains() {
        let chain = vec!["A".to_string(), "A/sub1".to_string()];
        assert_eq!(hierarchical_edge(&chain, &chain), None);
    }

    #[test]
    fn common_ancestor_scope_finds_shared_prefix() {
        assert_eq!(
            common_ancestor_scope("A/subA1", "A/subA2"),
            Some("A".to_string())
        );
        assert_eq!(common_ancestor_scope("A", "B"), None);
    }

    #[test]
    fn edges_from_pairs_flat_mode_connects_deepest_nodes() {
        let tree = sample_tree();
        let pairs = vec![(
            "$(ROOT)/moduleA/subA1/a.o".to_string(),
            "$(ROOT)/moduleB/b.h".to_string(),
        )];
        let edges = edges_from_pairs(&pairs, &tree, EdgeMode::Flat);
        assert!(edges.contains(&("moduleA/subA1".to_string(), "moduleB".to_string())));
    }

    #[test]
    fn edges_from_pairs_hierarchical_mode_connects_top_level() {
        let tree = sample_tree();
        let pairs = vec![(
            "$(ROOT)/moduleA/subA1/a.o".to_string(),
            "$(ROOT)/moduleB/b.h".to_string(),
        )];
        let edges = edges_from_pairs(&pairs, &tree, EdgeMode::Hierarchical);
        assert!(edges.contains(&("moduleA".to_string(), "moduleB".to_string())));
    }

    #[test]
    fn edges_from_pairs_hierarchical_mode_connects_siblings() {
        let tree = sample_tree();
        let pairs = vec![(
            "$(ROOT)/moduleA/subA1/a.o".to_string(),
            "$(ROOT)/moduleA/subA2/b.h".to_string(),
        )];
        let edges = edges_from_pairs(&pairs, &tree, EdgeMode::Hierarchical);
        assert!(edges.contains(&(
            "moduleA/subA1".to_string(),
            "moduleA/subA2".to_string()
        )));
    }

    #[test]
    fn edges_from_pairs_drops_same_folder_and_unmatched_pairs() {
        let tree = vec![leaf("moduleA", "moduleA"), leaf("moduleB", "moduleB")];
        let pairs = vec![
            (
                "$(ROOT)/moduleA/foo.o".to_string(),
                "$(ROOT)/moduleB/bar.h".to_string(),
            ),
            (
                // same folder on both sides: not an edge
                "$(ROOT)/moduleA/foo.o".to_string(),
                "$(ROOT)/moduleA/foo.c".to_string(),
            ),
            (
                // prerequisite outside the tree: not an edge
                "$(ROOT)/moduleA/foo.o".to_string(),
                "/usr/include/stdio.h".to_string(),
            ),
        ];

        let edges = edges_from_pairs(&pairs, &tree, EdgeMode::Flat);
        assert_eq!(edges.len(), 1);
        assert!(edges.contains(&("moduleA".to_string(), "moduleB".to_string())));
    }

    fn edge_set(pairs: &[(&str, &str)]) -> HashSet<(String, String)> {
        pairs
            .iter()
            .map(|(a, b)| (a.to_string(), b.to_string()))
            .collect()
    }

    #[test]
    fn transitive_reduction_drops_direct_shortcut() {
        // A -> C is redundant because A -> B -> C already reaches C.
        let edges = edge_set(&[("A", "B"), ("B", "C"), ("A", "C")]);
        let reduced = transitive_reduction(&edges);
        assert_eq!(reduced, edge_set(&[("A", "B"), ("B", "C")]));
    }

    #[test]
    fn transitive_reduction_drops_diamond_shortcut() {
        // A -> D is redundant: A can already reach D via B or via C.
        let edges = edge_set(&[("A", "B"), ("A", "C"), ("B", "D"), ("C", "D"), ("A", "D")]);
        let reduced = transitive_reduction(&edges);
        assert_eq!(
            reduced,
            edge_set(&[("A", "B"), ("A", "C"), ("B", "D"), ("C", "D")])
        );
    }

    #[test]
    fn transitive_reduction_keeps_minimal_cycle() {
        // A lone bidirectional pair has no alternate path in either
        // direction, so both edges must be kept.
        let edges = edge_set(&[("A", "B"), ("B", "A")]);
        let reduced = transitive_reduction(&edges);
        assert_eq!(reduced, edges);
    }

    #[test]
    fn transitive_reduction_preserves_reachability_with_shortcut_into_cycle() {
        // A -> B is redundant here: A can already reach B via A -> C -> B.
        let edges = edge_set(&[("A", "B"), ("B", "A"), ("A", "C"), ("C", "B")]);
        let reduced = transitive_reduction(&edges);
        let adjacency: HashMap<String, HashSet<String>> = {
            let mut m: HashMap<String, HashSet<String>> = HashMap::new();
            for (a, b) in &reduced {
                m.entry(a.clone()).or_default().insert(b.clone());
            }
            m
        };
        assert!(is_reachable(&adjacency, "A", "B"));
        assert!(is_reachable(&adjacency, "B", "A"));
        assert!(!reduced.contains(&("A".to_string(), "B".to_string())));
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
    fn build_folder_tree_respects_level() {
        let dir = std::env::temp_dir().join("depgraph_test_tree");
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(dir.join("moduleA/subA1")).unwrap();
        fs::create_dir_all(dir.join("moduleB")).unwrap();

        let level1 = build_folder_tree(&dir, 1).unwrap();
        assert_eq!(level1.len(), 2);
        assert!(level1.iter().all(|n| n.children.is_empty()));

        let level2 = build_folder_tree(&dir, 2).unwrap();
        let module_a = level2.iter().find(|n| n.name == "moduleA").unwrap();
        assert_eq!(module_a.children.len(), 1);
        assert_eq!(module_a.children[0].path_id, "moduleA/subA1");
        let module_b = level2.iter().find(|n| n.name == "moduleB").unwrap();
        assert!(module_b.children.is_empty());

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn global_settings_include_compound_and_nodesep() {
        let tree = vec![leaf("A", "A")];
        let edges = HashSet::new();
        let dot = render_dot(&tree, &edges, EdgeMode::Flat);
        assert!(dot.contains("compound=true;"));
        assert!(dot.contains("nodesep=.55;"));
    }

    #[test]
    fn plain_top_level_edge_gets_no_cross_cluster_attrs() {
        // Neither A nor B is exploded into a cluster, so there's no
        // boundary to clip the edge to.
        let tree = vec![leaf("A", "A"), leaf("B", "B")];
        assert_eq!(cross_cluster_attrs("A", "B", &tree), None);
    }

    #[test]
    fn edge_into_exploded_folder_gets_lhead_only() {
        // "moduleB" is a plain leaf (no ltail possible); "moduleA" is
        // exploded, so only lhead is added, alongside minlen=0.
        let tree = sample_tree();
        let attrs = cross_cluster_attrs("moduleB", "moduleA/subA1", &tree).unwrap();
        assert!(attrs.contains(&"minlen=0".to_string()));
        assert!(attrs.contains(&"lhead=\"cluster_moduleA\"".to_string()));
        assert!(!attrs.iter().any(|a| a.starts_with("ltail")));
    }

    #[test]
    fn edge_between_two_exploded_folders_gets_both_ltail_and_lhead() {
        let mut tree = sample_tree();
        tree.push(FolderNode {
            name: "moduleC".to_string(),
            path_id: "moduleC".to_string(),
            children: vec![leaf("subC1", "moduleC/subC1")],
        });
        let attrs = cross_cluster_attrs("moduleC/subC1", "moduleA/subA2", &tree).unwrap();
        assert!(attrs.contains(&"minlen=0".to_string()));
        assert!(attrs.contains(&"ltail=\"cluster_moduleC\"".to_string()));
        assert!(attrs.contains(&"lhead=\"cluster_moduleA\"".to_string()));
    }

    #[test]
    fn edge_inside_same_top_level_folder_gets_no_cross_cluster_attrs() {
        let tree = sample_tree();
        assert_eq!(
            cross_cluster_attrs("moduleA/subA1", "moduleA/subA2", &tree),
            None
        );
    }

    #[test]
    fn bidirectional_pair_rendered_red_once() {
        let tree = vec![leaf("A", "A"), leaf("B", "B")];
        let mut edges = HashSet::new();
        edges.insert(("A".to_string(), "B".to_string()));
        edges.insert(("B".to_string(), "A".to_string()));

        let dot = render_dot(&tree, &edges, EdgeMode::Flat);
        assert_eq!(dot.matches("color=red").count(), 1);
        assert_eq!(dot.matches("->").count(), 1);
    }

    #[test]
    fn one_directional_pair_rendered_black() {
        let tree = vec![leaf("A", "A"), leaf("B", "B")];
        let mut edges = HashSet::new();
        edges.insert(("A".to_string(), "B".to_string()));

        let dot = render_dot(&tree, &edges, EdgeMode::Flat);
        assert_eq!(dot.matches("color=red").count(), 0);
        assert!(dot.contains("\"A\" -> \"B\";"));
    }

    #[test]
    fn exploded_folder_renders_as_cluster() {
        let tree = sample_tree();
        let edges = HashSet::new();
        let dot = render_dot(&tree, &edges, EdgeMode::Flat);
        assert!(dot.contains("subgraph cluster_moduleA"));
        assert!(dot.contains("\"moduleA/subA1\" [label=\"subA1\"];"));
        assert!(dot.contains("\"moduleA/subA2\" [label=\"subA2\"];"));
        // moduleB is a leaf: no cluster for it, and its label is its own
        // (unshortened) name since it isn't nested inside anything.
        assert!(!dot.contains("subgraph cluster_moduleB"));
        assert!(dot.contains("\"moduleB\" [label=\"moduleB\"];"));
    }

    #[test]
    fn hierarchical_mode_nests_sibling_edge_inside_cluster() {
        let tree = sample_tree();
        let mut edges = HashSet::new();
        edges.insert(("moduleA/subA1".to_string(), "moduleA/subA2".to_string()));

        let dot = render_dot(&tree, &edges, EdgeMode::Hierarchical);
        let cluster_start = dot.find("subgraph cluster_moduleA").unwrap();
        let cluster_end = dot[cluster_start..].find('}').unwrap() + cluster_start;
        let edge_pos = dot
            .find("\"moduleA/subA1\" -> \"moduleA/subA2\";")
            .unwrap();
        assert!(
            edge_pos > cluster_start && edge_pos < cluster_end,
            "expected the sibling edge to be nested inside cluster_moduleA"
        );
    }
}
