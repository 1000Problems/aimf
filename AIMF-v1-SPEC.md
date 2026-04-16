# AIMF v1 — AI Minimal Format Specification

**Version:** 1.0
**Purpose:** A lightweight navigation manifest that tells an LLM what project resources exist, what they mean, where to find them, and how much they cost to load — without loading anything. The LLM decides what to materialize based on the task at hand, using prompt caching to keep loaded resources warm across turns.

---

## 1. Design Principles

1. **The manifest IS the context.** AIMF is not a data format — it's a map. A 500-file project's AIMF should be 30-80 lines, not 30,000.
2. **Nothing loads until asked.** @NAV describes resources; Claude reads them from the working folder when needed. No embedded content, no cold blocks.
3. **Token cost is first-class.** Every @NAV entry carries an estimated token cost. Before loading anything, Claude can present a budget and let the user approve.
4. **Prompt cache is the memory layer.** Once Claude loads a resource, it stays warm in prompt cache. AIMF doesn't manage caching — it enables it by making load decisions explicit.
5. **AIMF flows between models.** Opus writes AIMF to hand off context to Sonnet. The same format serves as project manifest AND task context.

---

## 2. Document Structure

An AIMF document has exactly two sections:

```
@NAV    ← Navigation manifest (what exists, what it means, where it is)
@CTX    ← Working context (project state, session notes, what's next)
```

Both sections are required. @NAV can be empty (for pure-task documents). @CTX can be minimal.

---

## 3. Section Specifications

### 3.1 @NAV — Navigation Manifest

Each line is one resource the LLM should **know about** but **not load** until needed.

Format: pipe-delimited, one line per resource, with a header line.

```
@NAV
id    | type | path                              | about                                         | tokens | load
ARCH  | MRM  | docs/architecture.mermaid          | System-wide component map, data flow overview  | 320    | design
AUTH  | MRM  | docs/auth-flow.mermaid             | OAuth2 + session management sequence diagram   | 480    | design, task:auth
DB    | MRM  | docs/data-model.mermaid            | Entity relationships, all core tables          | 640    | design, task:db
D001  | DEC  | decisions/001-use-tauri.md          | Why Tauri over Electron: perf + binary size    | 200    | reference
D002  | DEC  | decisions/002-ascii-format.md       | Why text-only ASCII over binary encoding       | 180    | reference
COMP  | DIR  | src/compiler/                       | 4-stage pipeline: scan, classify, curate, emit | 4200   | task:compiler
API   | DIR  | src/api/                            | REST endpoint handlers                         | 1800   | task:api
TEST  | DIR  | tests/                              | Integration and unit tests                     | 2400   | task:*
CFG   | CFG  | Cargo.toml                          | Dependencies and build configuration           | 180    | task:*
READ  | DOC  | README.md                           | Project overview, setup instructions           | 600    | reference
```

**Fields:**

| Field  | Description |
|--------|-------------|
| id     | Short, human-meaningful tag. Use project-relevant names: ARCH, AUTH, DB, D001 — not F1, F2. |
| type   | Resource type: `MRM` (mermaid), `DEC` (decision doc), `DIR` (directory), `CFG` (config), `DOC` (documentation), `SPEC` (specification), `SRC` (single source file), `TASK` (task document) |
| path   | Relative path from project root. Directories end with `/`. |
| about  | One-line description rich enough to reason about the resource without loading it. Max 80 chars. |
| tokens | Estimated token cost to load this resource. Calculated as `ceil(bytes / 4)` for files, sum of all files for directories. |
| load   | Comma-separated load hints: `design` (load in design mode), `task:X` (load for task X), `task:*` (load for any task), `reference` (load only when explicitly asked) |

**Resource Types:**

| Type | Meaning | Typical Content |
|------|---------|-----------------|
| MRM  | Mermaid diagram | Architecture, flow, ER, sequence diagrams |
| DEC  | Decision document | ADRs, design rationale, tradeoff analysis |
| DIR  | Source directory | Code organized as a logical unit |
| CFG  | Configuration file | Cargo.toml, package.json, tsconfig.json |
| DOC  | Documentation | README, guides, API docs |
| SPEC | Specification | Format specs, protocol definitions |
| SRC  | Single source file | A specific file (vs. a directory) |
| TASK | Task document | Opus-generated task with embedded AIMF |

**Load Hints:**

| Hint | When to load |
|------|-------------|
| `design` | User puts Claude in design mode — load proactively |
| `task:auth` | Working on the auth feature specifically |
| `task:*` | Any task (tests, config — always relevant) |
| `reference` | Only when the user or conversation explicitly needs it |

### 3.2 @CTX — Working Context

Plain key-value working notes. Not file content — project state that orients Claude at session start.

```
@CTX
status: Compiler and skill implemented. Reader tested end-to-end.
last_session: 2026-04-15 — Redesigned AIMF from v1 (repo dump) to v2 (navigation manifest).
focus: Testing full Opus→Sonnet handoff loop.
next: Run compiler against a real 500-file project, validate token estimates.
blockers: None.
decisions_pending: Whether to support inline task instructions or keep as separate markdown.
notes: The session monitor (FOUNDATION.md) has proven JSONL parsing code we can reuse.
```

**Common keys:**

| Key | Purpose |
|-----|---------|
| status | Current project state in one sentence |
| last_session | When and what happened last (use absolute dates) |
| focus | What area of the project is active |
| next | What should happen next |
| blockers | Anything preventing progress |
| decisions_pending | Open questions needing resolution |
| notes | Freeform context that doesn't fit elsewhere |

Additional keys are allowed. Keep @CTX under 15 lines — it should be scannable in seconds.

---

## 4. Token Budget Protocol

When Claude loads AIMF and enters a mode, it MUST present a token budget before loading resources:

**Design mode example:**
```
Loading design context requires 8 resources:

  ARCH  docs/architecture.mermaid          320 tokens
  AUTH  docs/auth-flow.mermaid             480 tokens
  DB    docs/data-model.mermaid            640 tokens
  API   docs/api-surface.mermaid           520 tokens
  D001  decisions/001-use-tauri.md         200 tokens
  D002  decisions/002-ascii-format.md      180 tokens
  CFG   Cargo.toml                         180 tokens
  TEST  tests/                           2,400 tokens
                                    ─────────────────
  Total estimated:                       4,920 tokens

Should I load all of these, or would you like to exclude any?
```

The user can say "skip TEST and D002" and Claude loads only the approved set.

---

## 5. Task Handoff Format

When Opus generates a task for Sonnet, the task document contains three parts:

```
@NAV
[curated subset of project AIMF — only what Sonnet needs for this task]

@CTX
[briefing written by Opus — decisions made, constraints, what exists]

---
# Task: [title]

[Plain markdown instructions — what to build, patterns to follow, tests to write]
```

Sonnet reads @NAV, loads everything listed (no approval step — Opus already curated), then follows the markdown task.

The total token cost (AIMF resources + task instructions) is reported in the task document header so the user knows execution cost before launching Sonnet.

---

## 6. Formal Grammar (EBNF)

```ebnf
AIMF           = NavSection, CtxSection, [ TaskSection ] ;

NavSection     = "@NAV", NL, [ HeaderLine ], { NavEntry } ;
HeaderLine     = "id", SEP, "type", SEP, "path", SEP, "about", SEP, "tokens", SEP, "load", NL ;
NavEntry       = ID, SEP, Type, SEP, Path, SEP, About, SEP, Tokens, SEP, LoadHints, NL ;

CtxSection     = "@CTX", NL, { CtxLine } ;
CtxLine        = Key, ":", SP, Value, NL ;

TaskSection    = "---", NL, { ANY } ;   (* plain markdown *)

(* Lexical *)
ID             = 1*( ALPHA | DIGIT ) ;
Type           = "MRM" | "DEC" | "DIR" | "CFG" | "DOC" | "SPEC" | "SRC" | "TASK" ;
Path           = 1*( VCHAR - "|" ) ;
About          = 1*( VCHAR - "|" ) ;     (* max 80 chars *)
Tokens         = 1*DIGIT ;
LoadHints      = LoadHint, { ",", SP, LoadHint } ;
LoadHint       = "design" | "reference" | "task:", 1*( ALPHA | "*" ) ;
Key            = 1*( ALPHA | "_" ) ;
Value          = 1*( VCHAR | SP ) ;
SEP            = SP, "|", SP ;            (* " | " *)

ALPHA          = "A"-"Z" | "a"-"z" ;
DIGIT          = "0"-"9" ;
VCHAR          = %x21-7E ;
SP             = " " ;
NL             = %x0A ;
ANY            = %x00-10FFFF ;
```

---

## 7. Example: Complete AIMF Document

```
@NAV
id    | type | path                         | about                                          | tokens | load
ARCH  | MRM  | AIMF-High-Level.mermaid       | System architecture: compiler → doc → skill     | 320    | design
DETL  | MRM  | AIMF-Detailed.mermaid         | Full pipeline detail with all stages and flows  | 780    | design
SPEC  | SPEC | AIMF-v1-SPEC.md               | Complete format specification with grammar       | 2400   | design, reference
COMP  | DIR  | aimf-compiler/src-tauri/src/   | Rust compiler: scanner, classifier, curator      | 4200   | task:compiler
SKILL | DIR  | aimf-reader/                   | Cowork skill: SKILL.md + parser script           | 1800   | task:skill
CFG   | CFG  | aimf-compiler/src-tauri/Cargo.toml | Dependencies and build config               | 180    | task:compiler

@CTX
status: v2 redesign complete. Compiler and skill implemented, untested.
last_session: 2026-04-15 — Rebuilt everything around navigation manifest concept.
focus: End-to-end testing across exploration, design, and task modes.
next: Compile this project, test in fresh Opus session, generate Sonnet task.
blockers: None.
```

That's 16 lines for the entire project. ~100 tokens to load the manifest. Then Claude knows everything that exists and can load any resource on demand.

---

## 8. File Extension

`.aimf` — plain text, UTF-8, LF line endings.

Task documents with embedded AIMF use `.task.md` extension.
