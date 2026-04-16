# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## What This Project Is

AIMF (AI Minimal Format) is a **navigation manifest** — a lightweight document (~40–80 lines) that tells an LLM what project resources exist, where to find them, how much they cost in tokens, and when to load them. It does NOT embed file content. Resources are loaded on demand from the working folder.

The repo has two components:
- **`aimf-compiler/`** — Rust CLI (with optional Tauri desktop GUI) that scans a project and generates an `.aimf` manifest
- **`aimf-reader/`** — Python parser + Cowork skill for consuming `.aimf` documents

## Core Concept

AIMF solves the problem of getting project context into an LLM without wasting tokens on JSON/YAML overhead or loading everything upfront. The manifest is ~100 tokens. You read it, know the whole project, and load specific resources on demand — each one gets prompt-cached and stays warm across turns.

Three scenarios define AIMF's purpose:
1. **Exploration** — Load AIMF (~100 tokens), know the whole project, fetch individual resources on demand
2. **Design Mode** — Load all mermaid diagrams + decision docs into prompt cache (~3,500 tokens), design across the full system
3. **Task Handoff** — Opus creates a task document with curated AIMF header + markdown instructions → Sonnet reads it, loads resources, follows instructions

## AIMF Format (v2)

An AIMF document has exactly two sections:

```
@NAV — what exists (resource catalog with descriptions and token costs)
@CTX — where we are (project state, session notes, what's next)
```

**@NAV entries** (pipe-delimited): `id | type | path | about | tokens | load`
- **type**: MRM (mermaid), DEC (decision), DIR (directory), CFG (config), DOC (documentation), SPEC (specification), SRC (source), TASK (task)
- **load hints**: `design`, `task:X`, `task:*`, `reference`
- **tokens**: estimated cost via `bytes / 4` heuristic

**@CTX entries** (key-value): status, last_session, focus, next, blockers, token_budget, notes

**Task documents** add a `---` separator followed by markdown instructions (Opus → Sonnet handoff).

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

# Run with session log mining
cargo run --features cli -- --path . --output project.aimf --sessions ~/.claude/projects/myproject/

# Check without building
cargo check
cargo clippy
```

No test suite exists yet. There is no Makefile.

**Python reader** requires no dependencies (stdlib only):

```bash
python aimf-reader/scripts/parse_aimf.py project.aimf           # Full JSON summary
python aimf-reader/scripts/parse_aimf.py project.aimf --nav      # Navigation entries
python aimf-reader/scripts/parse_aimf.py project.aimf --ctx      # Working context
python aimf-reader/scripts/parse_aimf.py project.aimf --budget design  # Token budget for a mode
python aimf-reader/scripts/parse_aimf.py project.aimf --search auth    # Search by id/path/about
python aimf-reader/scripts/parse_aimf.py project.aimf --load ARCH      # Load resource by ID
```

## Architecture: 4-Stage Pipeline

The compiler is orchestrated in `lib.rs` and flows through four stages:

```
local project → Scanner → Classifier → Curator → Emitter → .aimf manifest
```

1. **Scanner** (`scanner/mod.rs`) — Discovers meaningful resources (not every file). Looks for: mermaid files, decision docs, specs, key configs, READMEs, source directories. Tracks source directories as logical units (e.g., `src/api/`) not individual files.

2. **Classifier** (`classifier/mod.rs`) — Generates descriptions by reading file content heuristically (no AI). Extracts mermaid diagram types, markdown headings, config names. Assigns meaningful IDs (ARCH, D001, API — not F1, F2). Sets default load hints per resource type.

3. **Curator** (`curator/mod.rs`) — Takes discovered resources + optional session enrichments → `(Vec<NavEntry>, Vec<CtxEntry>)`. Generates unique IDs, estimates tokens via `bytes / 4`, auto-generates @CTX fields (status, compiled date, token budget).

4. **Emitter** (`emitter/mod.rs`) — Writes the two-section AIMF document (@NAV with aligned columns, @CTX with key-value pairs). Also supports `emit_task()` for generating Opus → Sonnet handoff documents.

Optional: **Session Miner** (`session_miner/mod.rs`) — Parses Claude Code `.jsonl` session logs to enrich resource descriptions. Extracts mermaid context and file path references using regex (no AI, no token consumption).

## Key Types (`types.rs`)

- `ResourceType` — enum: MRM, DEC, DIR, CFG, DOC, SPEC, SRC, TASK
- `NavEntry` — id, resource_type, path, about, tokens, load_hints
- `CtxEntry` — key, value
- `AimfDocument` — nav: Vec<NavEntry>, ctx: Vec<CtxEntry>
- `CompilerConfig` — root, repo, branch, session_logs, ignore_patterns, include_source_dirs, max_about_len

## Public API (`lib.rs`)

- `compile(config) → String` — full pipeline, returns AIMF text
- `compile_to_doc(config) → AimfDocument` — returns structured document
- `generate_task(doc, task_nav_ids, task_ctx, task_markdown) → String` — creates task handoff document

## CLI (`cli.rs`)

Feature-gated behind `--features cli`. Uses clap.

```
aimf --path ./project --output project.aimf --repo owner/repo --branch main --sessions ~/.claude/projects/myproject/
```

Flags: `--path`, `--repo`, `--branch`, `--sessions`, `--output`, `--include-dirs`

## Tauri Desktop App

Feature-gated behind `--features tauri`. `main.rs` exposes two IPC commands:
- `compile_project(path, config)` → returns AIMF string
- `save_aimf(content, path)` → writes .aimf to disk

The UI is vanilla HTML/CSS/JS with a dark GitHub-style theme (`src/index.html`).

## Deprecated v1 Modules

The following modules exist as stubs for backward compatibility and should not be used:
- `partitioner/mod.rs` — replaced by curator
- `hot_strategy/mod.rs` — v2 does not embed content

## Spec & Design Docs

- `AIMF-v1-SPEC.md` — v2 format specification with EBNF grammar and token budget protocol
- `startingPoint.md` — original design rationale
- `AIMF-High-Level.mermaid` / `AIMF-Detailed.mermaid` — architecture diagrams (v1, pending update)
- `AIMF-Project-Blueprint.docx` — comprehensive project blueprint
- `FOUNDATION.md` — session log mining techniques (JSONL parsing, mermaid extraction regex)
- `aimf-reader/SKILL.md` — Cowork skill documentation (three operating modes)
- `TASK-example.task.md` — example Opus → Sonnet task handoff document
- `project.aimf` — real AIMF manifest for this project
