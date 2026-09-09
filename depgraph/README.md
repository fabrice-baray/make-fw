# depgraph

Reads gcc-generated `.d` dependency files under a folder tree and emits a
Graphviz `.dot` graph showing dependencies *between first-level subfolders*.

- **Nodes**: first-level subfolders of the input folder.
- **Edge A -> B**: at least one file under `A` depends on (includes) a file
  under `B`.
- **Red edge**: dependencies exist in both directions between `A` and `B`
  (drawn as a single edge with `dir=both`, not two separate arrows).

Dependencies within the same subfolder, or pointing outside the input tree
entirely (e.g. system headers like `/usr/include/stdio.h`), are ignored.

## Build

No external crates are used, so this builds fully offline:

```sh
cargo build --release
```

The binary will be at `target/release/depgraph`.

## Run

```sh
depgraph <input_folder> [output.dot] [--verbose]
```

- `input_folder`: root of the tree containing your subfolders and `.d` files.
- `output.dot`: optional, defaults to `graph.dot` in the current directory.
- `--verbose` / `-v`: print each discovered folder-to-folder edge as it's
  found, and a count of parsed `.d` files, to stderr.

Then render it, e.g.:

```sh
dot -Tpng graph.dot -o graph.png
```

## Assumptions

1. **Paths in `.d` files are absolute.** This matches gcc invoked with
   absolute source/include paths. If your build instead uses paths relative
   to the compiler's working directory, they're resolved as a fallback
   relative to the `.d` file's own directory — but this may not match your
   actual build directory. Let me know if you need a `--base-dir` override.
2. Only files ending in `.d` are treated as dependency files (the standard
   gcc `-MMD`/`-MD` output extension).
3. A `.d` file can contain multiple rules (e.g. the extra empty rules gcc
   emits with `-MP`); all are parsed.
4. Backslash-escaped spaces in filenames (`foo\ bar.h`) are handled;
   Windows drive-letter colons (`C:\...`) are not mistaken for the
   target/prerequisite separator.
5. Symlinks are not specially resolved beyond what `fs::canonicalize` does
   on the root folder itself.

## Testing

```sh
cargo test
```

Unit tests cover: token splitting, escaped spaces, the drive-letter-colon
edge case, top-level-folder detection, path normalization, multi-line rule
parsing with continuations, and the red-bidirectional-edge merging logic.

## Trying it on a sample tree

```sh
mkdir -p sample/moduleA sample/moduleB sample/moduleC
cat > sample/moduleA/a.d <<'EOF'
/abs/sample/moduleA/a.o: /abs/sample/moduleA/a.c /abs/sample/moduleB/b.h
EOF
cat > sample/moduleB/b.d <<'EOF'
/abs/sample/moduleB/b.o: /abs/sample/moduleB/b.c /abs/sample/moduleA/a.h
EOF
cat > sample/moduleC/c.d <<'EOF'
/abs/sample/moduleC/c.o: /abs/sample/moduleC/c.c /abs/sample/moduleA/a.h
EOF
```
(replace `/abs/sample` with the real absolute path to `sample` on your
machine, since paths are expected to be absolute)

Running `depgraph sample` should produce moduleA<->moduleB as a **red**
bidirectional edge, and moduleC -> moduleA as a plain black edge.
