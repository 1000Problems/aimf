# AIMF v1 — AI Minimal Format Specification

**Version:** 1.0
**Purpose:** Encode an entire software project into a single, token-efficient document optimized for LLM context memory, prompt caching, and selective on-demand loading.

---

## 1. Design Principles

1. **Token economy** — every byte earns its place. No quotes, no braces, no nesting, no redundant whitespace.
2. **Cache-friendly** — sections are ordered so the stable prefix (@H → @I) rarely changes, maximizing prompt-cache hits. Hot memory (@M) is the volatile layer.
3. **Lazy loading** — the index describes *everything*; only hot entries are materialized. Cold resources are fetched from the working folder on demand.
4. **Text-only ASCII** — human-debuggable, no binary encoding. Delimiters use `<<` and `>>` (common, cheap tokens).
5. **Large-project ready** — supports sharding, tiered summarization, and resource groups for 500+ file repos.

---

## 2. Document Structure

An AIMF document consists of five ordered sections:

```
@H          ← Header (document metadata)
@G          ← Groups (optional, resource grouping for large projects)
@I          ← Index (resource catalog)
@M          ← Hot Memory (materialized content)
@C          ← Cold Blocks (optional, embedded content)
```

Sections MUST appear in this order. @G and @C are optional.

---

## 3. Section Specifications

### 3.1 Header (@H)

Provides document-level metadata. One key-value pair per line, colon-separated, no spaces around colon.

```
@H
V:1
R:847
HOT:12
GRP:8
REPO:owner/repo-name
BRANCH:main
COMMIT:a1b2c3d
TS:20260415T143022Z
ROOT:/path/to/working/folder
SHARD:1/3
```

| Field    | Required | Description |
|----------|----------|-------------|
| V        | yes      | AIMF version (integer) |
| R        | yes      | Total resource count |
| HOT      | yes      | Number of hot memory entries |
| GRP      | no       | Number of resource groups |
| REPO     | no       | GitHub origin (owner/repo) |
| BRANCH   | no       | Branch name |
| COMMIT   | no       | Short commit hash |
| TS       | yes      | Compilation timestamp (ISO 8601 compact) |
| ROOT     | yes      | Working folder root path |
| SHARD    | no       | Shard index/total (e.g. 1/3) for large projects |

### 3.2 Groups (@G) — Optional

For large projects, resources are organized into logical groups. Each group has a short ID, a human label, and a glob pattern or explicit membership.

```
@G
G1|core|src/lib/**
G2|api|src/api/**
G3|cfg|*.toml,*.yaml,*.json
G4|test|tests/**,*_test.*
G5|doc|docs/**,*.md
G6|ui|src/ui/**
G7|build|Cargo.*,Makefile,Dockerfile
G8|other|*
```

Format: `GroupID|Label|GlobPatterns`

- GroupID: short alphanumeric (G1, G2, ...)
- Label: human-readable name, no pipes
- GlobPatterns: comma-separated globs, evaluated in order, first match wins

Groups enable the LLM to request "load all api resources" rather than individual files.

### 3.3 Index (@I)

One line per resource. Pipe-delimited, fixed field order.

```
@I
F1|SRC|on_edit|src/main.rs|4096|a1b2c3d4|G1|entry point
F2|CFG|always|Cargo.toml|512|e5f6a7b8|G7|dependencies and build config
F3|DOC|on_request|README.md|8192|c9d0e1f2|G5|project overview
F4|SRC|on_request|src/api/routes.rs|2048|11223344|G2|API route definitions
F5|TEST|on_request|tests/test_main.rs|1536|55667788|G4|main test suite
F6|SRC|on_request|src/lib/parser.rs|3200|aabbccdd|G1|AIMF parser module
F7|META|never|assets/logo.png|24576|deadbeef|G8|binary asset
```

| Field | Description |
|-------|-------------|
| ResourceID | Short alphanumeric ID (F1, F2, ...) |
| ResourceType | SRC, CFG, DOC, TEST, MRM (mermaid), BIN, META, OTHER |
| LoadWhen | always, on_edit, on_request, on_error, on_group, never |
| Path | Relative path from ROOT |
| Size | File size in bytes |
| Hash | Truncated content hash (8 hex chars) |
| GroupID | Group membership (from @G, or `-` if no groups) |
| Hint | Short semantic hint (what this resource is/does), max 60 chars |

**ResourceType additions over draft:**
- `MRM` — Mermaid diagram files (architecture, flow, ER diagrams)
- Types: SRC, CFG, DOC, TEST, MRM, BIN, META, OTHER

**LoadWhen semantics:**
- `always` — include in every prompt (configs, entry points)
- `on_edit` — load when the user is actively editing this file
- `on_request` — load when the LLM or user explicitly asks for it
- `on_error` — load when a build/test error references this file
- `on_group` — load when the containing group is requested
- `never` — reference only (binaries, large assets)

### 3.4 Hot Memory (@M)

Materialized content that is always present in the prompt. Each entry starts with `@M` followed by the ResourceID or a special key, then the content between `<<` and `>>` delimiters.

```
@M CTX
<<task=refactor auth module;focus=G2;cursor=F4:128>>

@M F2
<<[package]
name = "aimf-compiler"
version = "0.1.0"
edition = "2021"

[dependencies]
...>>

@M F1
<<fn main() {
    let config = Config::load();
    let compiler = Compiler::new(config);
    compiler.run();
}>>

@M ARCH
<<System uses 4-stage pipeline: scan -> classify -> partition -> emit.
Entry: src/main.rs -> Compiler::run()
Core modules: scanner (file walk), classifier (type detection),
partitioner (hot/cold/group assignment), emitter (AIMF output).
API layer in src/api/ handles HTTP endpoints.
Tests mirror src/ structure under tests/.>>

@M DEPS
<<serde 1.0, tokio 1.0, clap 4.0, walkdir 2.0, sha2 0.10, tauri 2.0>>
```

**Special hot keys:**
- `CTX` — current task context, cursor position, focus group
- `ARCH` — architecture summary (LLM-generated or hand-written)
- `DEPS` — dependency summary
- `ERR` — last error context (populated on_error)
- `DELTA` — recent changes since last compilation

Any ResourceID (F1, F2, ...) can also appear as a hot entry with its full or summarized content.

**Hot payload strategies for large projects:**
1. **Full** — entire file content (for files < 2KB)
2. **Summary** — LLM-generated summary (for files 2KB-20KB)
3. **Head** — first N lines + `...` (for files > 20KB)
4. **Signature** — function/struct/class signatures only (for source files)
5. **Delta** — diff since last compilation (for iterative sessions)

The strategy is chosen by the compiler based on file size and type.

### 3.5 Cold Blocks (@C) — Optional

Embedded content for resources that should be self-contained but not hot. Same delimiter format.

```
@C F6
<<pub struct Parser {
    input: String,
    pos: usize,
}

impl Parser {
    pub fn new(input: String) -> Self { ... }
    pub fn parse_header(&mut self) -> Header { ... }
    pub fn parse_index(&mut self) -> Vec<Resource> { ... }
}>>
```

For large projects, cold blocks are typically omitted — the index + working folder path is sufficient for on-demand loading.

---

## 4. Sharding Strategy (Large Projects)

When a project exceeds a token budget (configurable, default 100K tokens), the compiler produces multiple AIMF shards:

- **Shard 0 (spine):** @H + @G + @I + @M with architecture/context only. No file content.
- **Shard 1..N (group shards):** One shard per group or cluster of groups, containing @M and @C entries for that group's resources.

The spine shard is always loaded. Group shards are loaded on demand.

Shard files are named: `project.aimf` (spine), `project.G1.aimf`, `project.G2.aimf`, etc.

---

## 5. Formal Grammar (EBNF)

```ebnf
AIMF           = HeaderSection, [ GroupSection ], IndexSection,
                 HotMemorySection, { ColdBlock } ;

HeaderSection  = "@H", NL, { HeaderLine } ;
HeaderLine     = Key, ":", Value, NL ;

GroupSection   = "@G", NL, { GroupLine } ;
GroupLine      = GroupID, "|", Label, "|", GlobPatterns, NL ;

IndexSection   = "@I", NL, { ResourceLine } ;
ResourceLine   = ResourceID, "|", ResourceType, "|", LoadWhen, "|",
                 Path, "|", Size, "|", Hash, "|", GroupID, "|", Hint, NL ;

HotMemorySection = { HotEntry } ;
HotEntry       = "@M", SP, HotKey, NL, "<<", Payload, ">>", NL ;

ColdBlock      = "@C", SP, ResourceID, NL, "<<", Payload, ">>", NL ;

(* Lexical *)
Key            = 1*ALPHA ;
Value          = 1*VCHAR ;
GroupID        = "G", 1*DIGIT ;
Label          = 1*( VCHAR - "|" ) ;
GlobPatterns   = GlobPattern, { ",", GlobPattern } ;
GlobPattern    = 1*( VCHAR - "|" - "," ) ;
ResourceID     = "F", 1*DIGIT ;
ResourceType   = "SRC" | "CFG" | "DOC" | "TEST" | "MRM" | "BIN" | "META" | "OTHER" ;
LoadWhen       = "always" | "on_edit" | "on_request" | "on_error" | "on_group" | "never" ;
Path           = 1*( VCHAR - "|" ) ;
Size           = 1*DIGIT ;
Hash           = 8*HEXDIG ;
Hint           = 1*( VCHAR - "|" ) ;      (* max 60 chars *)
HotKey         = ResourceID | "CTX" | "ARCH" | "DEPS" | "ERR" | "DELTA" ;
Payload        = { ANY - "<<" - ">>" } ;  (* no nested delimiters *)

ALPHA          = "A"-"Z" | "a"-"z" ;
DIGIT          = "0"-"9" ;
HEXDIG         = DIGIT | "a"-"f" ;
VCHAR          = %x21-7E ;
SP             = " " ;
NL             = %x0A ;
ANY            = %x00-10FFFF ;
```

---

## 6. Token Budget Estimates

| Section | Typical tokens (500-file project) |
|---------|-----------------------------------|
| @H      | ~30 |
| @G      | ~80 |
| @I      | ~5,000 (500 lines × ~10 tokens each) |
| @M      | ~2,000–20,000 (depends on hot strategy) |
| @C      | 0 (omitted for large projects) |
| **Spine total** | **~7,000–25,000 tokens** |

This leaves the majority of context window for actual work.

---

## 7. File Extension

`.aimf` — plain text, UTF-8 encoded, LF line endings.

---

## 8. Example: Minimal AIMF Document

```
@H
V:1
R:3
HOT:2
TS:20260415T143022Z
ROOT:/home/user/myproject

@I
F1|SRC|always|src/main.rs|1024|a1b2c3d4|-|entry point
F2|CFG|always|Cargo.toml|256|e5f6a7b8|-|build config
F3|DOC|on_request|README.md|4096|c9d0e1f2|-|project docs

@M CTX
<<task=initial setup;focus=F1>>

@M F1
<<fn main() {
    println!("Hello AIMF");
}>>
```
