---
name: aimf-reader
description: "Load and navigate AIMF (AI Minimal Format) project context documents. Use this skill whenever the user mentions AIMF, .aimf files, project context loading, says 'load project context', 'read the aimf', 'what resources are available', or wants to work with a codebase that has been compiled into AIMF format. Also trigger when you detect an .aimf file in the working directory, when the user asks to explore a project structure from an AIMF document, or says things like 'load group X', 'show me the hot memory', 'what files are in this project'. This skill is the bridge between compiled AIMF documents and your working context — it tells you what exists, what's already loaded, and how to fetch what you need on demand."
---

# AIMF Reader — Project Context Loader

This skill lets you read AIMF (AI Minimal Format) documents and intelligently load project resources into your working context. AIMF is a token-efficient format that catalogs an entire codebase with a small index and selective hot/cold content loading.

## What AIMF looks like

An AIMF document has this structure:

```
@H          ← Header (metadata: version, resource count, root path)
@G          ← Groups (optional: logical clusters like "api", "core", "test")
@I          ← Index (one line per file: ID, type, load policy, path, size, hash, group, hint)
@M KEY      ← Hot Memory entries (content already materialized — always in context)
<<content>>
@C ID       ← Cold Blocks (optional embedded content, not hot)
<<content>>
```

The key insight: the @H and @I sections are cheap (a few thousand tokens even for 500+ file projects), while @M contains only the most important content. Everything else is fetched from the working folder on demand.

## How to use this skill

### Step 1: Find and parse the AIMF file

When this skill triggers, look for `.aimf` files in the working directory:

```bash
find . -name "*.aimf" -maxdepth 2 | head -5
```

Then run the parser to get a structured overview:

```bash
python /path/to/this/skill/scripts/parse_aimf.py <file.aimf>
```

This outputs a JSON summary with the header, groups, resource counts by type, and the hot memory keys. If the parser script isn't available, you can parse the file directly — the format is simple enough to read with basic text tools.

### Step 2: Load the hot memory

The @M section is your starting context. Read it carefully — it contains:

- **CTX** — current task context, what the user is focused on
- **ARCH** — architecture summary of the project
- **DEPS** — dependency overview
- **F1, F2, ...** — content of the most important files (entry points, configs)

This is your "working memory" for the project. Treat it as ground truth for orientation.

### Step 3: Load resources on demand

When you need a file that isn't in hot memory, use the index to find it:

1. **Search the @I index** for the resource by path, type, or hint
2. **Check the LoadWhen policy**:
   - `always` — should already be in @M. If not, load it.
   - `on_edit` — load when the user is editing this file
   - `on_request` — load when you or the user asks for it
   - `on_error` — load when an error references this file
   - `on_group` — load when the entire group is requested
   - `never` — reference only (binary assets), don't try to load
3. **Read from the working folder** using the ROOT path from @H plus the relative path from @I:
   ```
   ROOT + "/" + resource_path
   ```
4. **Check for cold blocks** (@C) first — if the content is embedded, use it instead of hitting the filesystem.

### Step 4: Group-based loading

For large projects, AIMF organizes files into groups (G1, G2, ...). When the user says something like "I need to work on the API layer" or "show me the tests", load the entire group:

1. Find the group ID from @G (e.g., G2|api|src/api/**)
2. Filter @I for all resources with that group ID
3. Load the ones that fit in your context budget (prioritize by LoadWhen, then by size ascending)

### Step 5: Sharded projects

If the @H section contains a `SHARD` field (e.g., `SHARD:1/3`), the project is split across multiple files:

- **Spine shard** (`project.aimf`): always load this first — it has the full index
- **Group shards** (`project.G1.aimf`, `project.G2.aimf`): load on demand when a group is needed

## Resource types

| Type  | Meaning | Typical LoadWhen |
|-------|---------|-----------------|
| SRC   | Source code | on_request, on_edit for entry points |
| CFG   | Configuration (toml, yaml, json) | always for small ones |
| DOC   | Documentation (md, rst, txt) | always for README, on_request otherwise |
| TEST  | Test files | on_request |
| MRM   | Mermaid diagrams | on_request |
| BIN   | Binary assets (images, etc.) | never |
| META  | CI configs, gitignore, etc. | on_group |
| OTHER | Everything else | on_request |

## Token budget awareness

Be mindful of how much context you're consuming. The AIMF index itself is very cheap (~10 tokens per resource line), but loading file contents adds up fast. General guidelines:

- The spine (@H + @G + @I + @M) for a 500-file project is ~7K-25K tokens
- Loading a group of 50 source files could be 50K-200K tokens
- Always prefer loading summaries/signatures over full files for initial orientation
- Load full content only when you need to read or edit specific code

## Workflow integration

When working on a task with AIMF context:

1. **Orient**: Read @M for architecture and current focus
2. **Navigate**: Use @I to find relevant files by type, group, or hint
3. **Load**: Fetch specific files from the working folder as needed
4. **Track**: If the user changes focus (new file, new task), mentally update CTX

## Example: user says "help me refactor the auth module"

1. Read @M ARCH to understand project structure
2. Search @I for resources with hints containing "auth" or paths containing "auth"
3. Load the matching files from the working folder
4. Also load any test files (type TEST) related to auth
5. Now you have full context to help with the refactor

## Parsing reference

For the full AIMF v1 specification (grammar, field definitions, sharding rules), read `references/AIMF-v1-SPEC.md` in this skill's directory.
