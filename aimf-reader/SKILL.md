---
name: aimf-reader
description: "Load and navigate AIMF (AI Minimal Format) project context documents. Use this skill whenever you see .aimf files, the user mentions AIMF, project context loading, says 'load project context', 'read the aimf', 'design mode', 'what resources are available', or wants to work with a codebase described by an AIMF manifest. Also trigger when the user asks to explore project structure, load mermaid diagrams, check decision docs, or prepare a task for Sonnet. AIMF is the token-efficient navigation protocol that tells you what exists in a project without loading anything — you decide what to materialize based on the task."
---

# AIMF Reader — Navigation Manifest Protocol

AIMF is a lightweight manifest that tells you what project resources exist, what they mean, where to find them, and how much they cost in tokens to load. It is NOT a data format — it contains no file content. You read the manifest (~100 tokens), know everything that's available, and load resources from the working folder on demand.

## Document Format

An AIMF document has exactly two sections:

```
@NAV — what exists (resource catalog with descriptions and token costs)
@CTX — where we are (project state, session notes, what's next)
```

### @NAV entries

Each line: `id | type | path | about | tokens | load`

- **id**: Human-meaningful tag (ARCH, AUTH, D001 — not F1, F2)
- **type**: MRM (mermaid), DEC (decision), DIR (directory), CFG (config), DOC (documentation), SPEC (specification), SRC (source), TASK (task)
- **path**: Where to find it relative to project root
- **about**: One-line description — enough to reason about the resource without loading it
- **tokens**: Estimated token cost to load
- **load**: When relevant — `design`, `task:X`, `task:*`, `reference`

### @CTX entries

Plain key-value notes: `key: value` — status, last_session, focus, next, blockers, etc.

## Operating Modes

When the user loads AIMF, ALWAYS read @NAV and @CTX first (~100 tokens). Then operate based on the mode they specify:

### Exploration Mode (default)

You know everything that exists but load nothing. When the user asks about a specific area, load that one resource. It gets prompt-cached. As the conversation shifts focus, load additional resources incrementally.

**Workflow:**
1. Read @NAV and @CTX
2. Tell the user what's available (summarize, don't dump the raw index)
3. Wait for them to ask about something specific
4. Load the relevant resource(s) from the working folder
5. Continue — each loaded resource stays cached

### Design Mode

The user says "design mode" or "load everything for design." Load all resources tagged with `load: design` (typically all mermaid diagrams + decision docs + specs).

**CRITICAL: Present a token budget BEFORE loading anything.**

**Workflow:**
1. Read @NAV and @CTX
2. Filter for all entries with `design` in their load hints
3. Sum their token costs and present a budget table:

```
Loading design context:

  ARCH  AIMF-High-Level.mermaid         320 tokens
  DETL  AIMF-Detailed.mermaid           780 tokens
  SPEC  AIMF-v1-SPEC.md               2,400 tokens
  D001  decisions/001-use-tauri.md       200 tokens
                                   ────────────────
  Total estimated:                     3,700 tokens

Should I load all of these, or exclude any?
```

4. After user approves, load the approved resources
5. All loaded resources get prompt-cached — you're ready to design across the whole system
6. As the user drills into specific areas, load additional resources from @NAV on demand

### Task Execution Mode (Sonnet)

The document contains @NAV + @CTX + markdown task instructions (separated by `---`). Load everything in @NAV without asking — Opus already curated what's needed.

**Workflow:**
1. Read @NAV — load ALL listed resources from the working folder
2. Read @CTX for background and constraints
3. Read the markdown task section after `---`
4. Execute the task with full context loaded

## Loading Resources

When you need to load a resource from @NAV:

1. Read the **path** field — it's relative to the project root
2. For **files** (MRM, DEC, CFG, DOC, SPEC, SRC): read the file directly
3. For **directories** (DIR): list the directory contents, then read files as needed for the task
4. For **TASK** type: read the task document (it may contain its own embedded AIMF)

Resources are loaded from the working folder (the project directory). If a resource is missing, tell the user and continue with what's available.

## Generating Tasks (Opus → Sonnet)

When the user asks you to create a task for Sonnet:

1. Determine which @NAV resources Sonnet will need for this task
2. Create a new document with:
   - @NAV section containing only the curated subset
   - @CTX section briefing Sonnet on decisions, constraints, existing code
   - `---` separator
   - Markdown instructions: what to build, patterns to follow, tests to write
3. Include a `<!-- Total resource cost: ~X tokens -->` comment after the separator
4. Save as `TASK-<name>.task.md`

The total token cost (resources + instructions) tells the user the execution cost before launching Sonnet.

## Parsing

To parse AIMF programmatically, use the bundled parser:

```bash
python scripts/parse_aimf.py <file.aimf>           # Full JSON summary
python scripts/parse_aimf.py <file.aimf> --nav      # Navigation entries
python scripts/parse_aimf.py <file.aimf> --ctx      # Working context
python scripts/parse_aimf.py <file.aimf> --budget design  # Token budget for a mode
python scripts/parse_aimf.py <file.aimf> --search auth    # Search by id/path/about
```

But the format is simple enough to read directly — it's just pipe-delimited text.
