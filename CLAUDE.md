# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## What This Project Is

AIMF (AI Minimal Format) is a token-efficient project context format — a compiler pipeline that converts a local codebase into a single `.aimf` document optimized for LLM ingestion. It addresses the problem that JSON/YAML/XML are wasteful when fed into AI context windows. The output format is ASCII, index-first, and lazy-loading by design.

The repo has two components:
- **`aimf-compiler/`** — Rust/Tauri app (pipeline core + desktop GUI + CLI binary)
- **`aimf-reader/`** — Python parser + Cowork skill for consuming `.aimf` documents

## Build Commands

All Rust work happens inside `aimf-compiler/src-tauri/`:

```bash
# Debug build (library only, no binary features)
cargo build

# Build the CLI binary
cargo build --features cli

# Build the Tauri desktop app
cargo build --features tauri

# Run the CLI (compile a project)
cargo run --features cli -- --path /path/to/project --repo owner/repo --branch main

# Run with all options
cargo run --features cli -- --path . --output project.aimf --max_hot 20 --max_hot_size 4096

# Check without building
cargo check
cargo clippy
```

No test suite exists yet. There is no Makefile.

**Python reader** requires no dependencies (stdlib only):

```bash
python aimf-reader/scripts/parse_aimf.py project.aimf
python aimf-reader/scripts/parse_aimf.py project.aimf --index
python aimf-reader/scripts/parse_aimf.py project.aimf --hot
python aimf-reader/scripts/parse_aimf.py project.aimf --load F3
python aimf-reader/scripts/parse_aimf.py project.aimf --search auth
python aimf-reader/scripts/parse_aimf.py project.aimf --stats
```

## Architecture: 4-Stage Pipeline

The compiler is orchestrated in `lib.rs` and flows through four stages:

```
local project → Scanner → Classifier → Partitioner → Emitter → .aimf file
```

1. **Scanner** (`scanner/mod.rs`) — Walks the directory tree, computes SHA-256 (8-char truncated), ignores `.git`, `node_modules`, `target`, `__pycache__`, lock files, `dist`, `build`.

2. **Classifier** (`classifier/mod.rs`) — Maps extensions to `ResourceType` (SRC, CFG, DOC, TEST, MRM, BIN, META, OTHER). Detects entry points (main.rs, lib.rs, index.ts, app.py, etc.). Generates semantic hints (≤60 chars).

3. **Partitioner** (`partitioner/mod.rs`) — Assigns files to glob-matched groups, sets `LoadWhen` policy (always / on_edit / on_request / on_error / on_group / never), scores files for hot candidacy, selects hot payload strategy (Full / Head(N) / Signature / Summary / Delta). Triggers sharding if resource count exceeds threshold (default 500).

4. **Emitter** (`emitter/mod.rs`) — Writes the 5-section AIMF document:
   - `@H` — Header (version, counts, repo/branch/commit, timestamp, root path, shard info)
   - `@G` — Groups (logical clusters with glob patterns)
   - `@I` — Index (one line per resource: `ID|Type|LoadWhen|Path|Size|Hash|Group|Hint`)
   - `@M` — Hot Memory (materialized content + special keys: CTX, ARCH, DEPS, ERR, DELTA)
   - `@C` — Cold Blocks (embedded content for self-contained docs)

## Key Types (`types.rs`)

- `Resource` — path, size, hash, type, group, hint, load_when, entry_point flag
- `Group` — id, name, glob patterns
- `HotEntry` — resource ID, strategy, materialized content
- `CompilerConfig` — all tunable parameters (max_hot_files, max_hot_file_size, summary_threshold, token_budget, shard_threshold, ignore_patterns)
- `LoadWhen` — enum: Always, OnEdit, OnRequest, OnError, OnGroup, Never
- `HotStrategy` — enum: Full, Summary, Head(usize), Signature, Delta

## Hot Memory Special Keys

Beyond file content, `@M` embeds synthetic keys:
- `CTX` — current task context (focus group, cursor position)
- `ARCH` — architecture summary (group descriptions, file composition counts)
- `DEPS` — dependency overview
- `ERR` — build error context (populated on failure)
- `DELTA` — git diff since last compile

## Sharding (500+ files)

When the project exceeds `shard_threshold`:
- **Spine** (`project.aimf`): @H + @G + @I + @M with no file content, just architecture keys
- **Group shards** (`project.G1.aimf`, etc.): @M and @C for that group's resources only

The spine is always loaded; group shards are fetched on demand.

## Tauri Desktop App

`main.rs` exposes two IPC commands to the frontend (`src/index.html`):
- `compile_project(path, config)` → returns AIMF string
- `save_to_file(content, path)` → writes .aimf to disk

The UI is vanilla HTML/CSS/JS with a dark GitHub-style theme.

## Spec & Design Docs

- `AIMF-v1-SPEC.md` — formal EBNF grammar, token budget estimates, full format reference
- `startingPoint.md` — design rationale (why not JSON/YAML/Mermaid)
- `AIMF-High-Level.mermaid` / `AIMF-Detailed.mermaid` — architecture diagrams
- `aimf-reader/SKILL.md` — Cowork skill documentation for consuming .aimf files
