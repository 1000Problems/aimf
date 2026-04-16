



I am looking at yaml, xml, json, and mermaid, and only mermaid is a decent text format for AI Context memory consumption. What I want: I want to define my claude code project in as few tokens as possible, use prompt caching and as needed loading to work with claude code on large projects. I am looking to create a new language format based on the following: As few token consumption as possible. The header should describe WHAT is available to be injested into the prompt, an index that points to the actual data with enough information to know what it is, when to load it, and where to get it. After that we should have the Hot Memory in a token saving format. Can you recommend a format for this that will be AI friendly (it is not necessary for this to be human readable).

es — you can design a format far more token‑efficient than YAML/XML/JSON/Mermaid. The right direction is a binary‑leaning, prefix‑coded, index‑first, content‑last micro‑language optimized for LLM ingestion, prompt caching, and selective loading.

Below is the best possible structure for Claude Code–style workflows: a format that is not human readable, minimizes tokens, maximizes cache reuse, and supports lazy loading of large project assets.

🧩 Core takeaway

The most AI‑friendly format is a two‑layer, prefix‑compressed, index‑addressable micro‑language:

Header (Hot Memory Index) A tiny, token‑minimal table describing:

what resources exist

their type

their semantic purpose

when they should be loaded

where to fetch them

Cold Blocks (Lazy‑Load Chunks) Each chunk is referenced by a short ID and stored in a compressed, token‑minimal encoding (base64‑like but optimized for LLMs).

This gives you:

minimal tokens

maximal prompt caching reuse

deterministic structure

fast selective loading

no need for human readability



You’re going for the right jugular here—“repo → AIMF” as a compiler is exactly the leverage point.
I’ll do two things:

Define a formal grammar (EBNF) for AIMF v1.
Describe a deterministic pipeline to convert a GitHub project into AIMF, including hot/cold partitioning.
AIMF v1 formal grammar (EBNF)

This is text‑only, ASCII‑only, no quoting, no nesting, minimal punctuation.

AIMF          = HeaderSection, IndexSection, HotMemorySection, [ ColdBlocksSection ] ;

HeaderSection = "@H", EOL,
                "R:", Integer, EOL,
                "HOT:", Integer, EOL,
                "VER:", Integer, EOL ;

IndexSection  = "@I", EOL,
                { ResourceLine } ;

ResourceLine  = ResourceID, "|", ResourceType, "|", LoadWhen, "|",
                Location, "|", Integer, "|", Hash, EOL ;

HotMemorySection = "@M", EOL,
                   { HotEntry } ;

HotEntry      = HotKey, ":", HotValue, EOL ;

ColdBlocksSection = { ColdBlock } ;

ColdBlock     = "@C", SP, ResourceID, EOL,
                BlockPayload, EOL ;

(* --- Lexical elements --- *)

ResourceID    = 1*( ALPHA | DIGIT ) ;
ResourceType  = "SRC" | "CFG" | "DOC" | "TEST" | "BIN" | "META" | "OTHER" ;

LoadWhen      = "always" | "on_edit" | "on_request" | "on_error" | "manual" ;

Location      = 1*( VCHAR - "|" - SP ) ;
(* e.g. "gh:src/main.py", "local:./foo/bar", "s3:bucket/key" *)

Hash          = 1*( ALPHA | DIGIT ) ;
HotKey        = 1*( ALPHA | DIGIT | "_" ) ;

HotValue      = "⟦",  { HotChar }, "⟧" ;
HotChar       = ANY - "⟦" - "⟧" ;
(* you can swap delimiters if needed, but keep them rare and consistent *)

BlockPayload  = "⟦", { PayloadChar }, "⟧" ;
PayloadChar   = ANY - "⟦" - "⟧" ;

Integer       = DIGIT, { DIGIT } ;

ALPHA         = "A"…"Z" | "a"…"z" ;
DIGIT         = "0"…"9" ;
VCHAR         = %x21-7E ;  (* visible ASCII *)
SP            = " " ;
EOL           = "\n" ;
ANY           = %x00-10FFFF ;
Key design points:

Sections are linear and ordered: @H → @I → @M → @C.
Index lines are single‑line, pipe‑separated: extremely token‑efficient and predictable.
Hot and cold payloads are opaque: AIMF doesn’t care what’s inside ⟦…⟧ as long as delimiters are respected.
No quotes, no braces, no nesting: everything is flat and streaming‑friendly.
Compiler: GitHub project → AIMF

Think of this as a deterministic pipeline:

Scan repo → file inventory
Classify files → ResourceType
Decide hot vs cold
Generate header + index
Generate hot memory payloads
Optionally generate cold blocks
I’ll outline each step in a way you can implement directly (Python, Go, Rust, whatever).

1. Scan repo and build file inventory

Input: local clone of GitHub repo.
Output: list of files with metadata.

For each file:

path: repo‑relative path (src/main.py)
size_bytes: file size
hash: e.g. sha1 or blake3 truncated to 8–12 chars
content: raw bytes (or stream)
You’ll also want:

git metadata: last commit, author, etc. (optional, can go into META resources later).
2. Classify files into ResourceType

Define a simple mapping:

SRC: *.py, *.ts, *.js, *.java, *.go, *.rs, etc.
CFG: *.json, *.yaml, *.yml, *.toml, .env, etc.
DOC: *.md, *.rst, LICENSE, README*, etc.
TEST: files under tests/, __tests__/, *_test.*, etc.
BIN: *.png, *.jpg, *.gif, *.pdf, *.zip, etc.
META: .gitignore, .editorconfig, CI configs, etc.
OTHER: anything else.
This classification is used in the ResourceType field of the index.

3. Decide hot vs cold (and LoadWhen)

You need a policy that can be tuned per project. A good default:

Hot candidates (always or on_edit):

README*, top‑level docs/overview.*
Entry points: src/main.*, app.py, index.ts, etc.
Currently edited file(s) (if you integrate with an editor/IDE).
Cold (on_request / manual):

Most source files
Large docs
Tests
Binaries (usually not loaded at all, just referenced)
LoadWhen defaults:

CFG → always
Entry SRC files → on_edit
Other SRC → on_request
DOC → on_request
TEST → on_request
BIN → manual
You can encode this as a simple function:

decide_load_when(type, path, size) -> LoadWhen
4. Generate header (@H)

You now know:

R: total number of resources (files)
HOT: number of hot entries you’ll actually inline in @M
VER: AIMF schema version (start with 1)
Example:

@H
R:128
HOT:4
VER:1
5. Generate index (@I)

For each file, assign a ResourceID and Location:

ResourceID: short, stable, token‑cheap.

e.g. F1, F2, … or a short hash like A1B2.
Keep it alphanumeric, no punctuation.
Location:

gh:<repo>/<path> if you want to preserve GitHub origin
or local:<path> if you’re working from a local clone.
LEN: file size in bytes.

HASH: truncated hash.

Example index:

@I
F1|SRC|on_edit|gh:src/main.py|2048|a1b2c3d4
F2|CFG|always|gh:config/app.yaml|512|e5f6a7b8
F3|DOC|on_request|gh:README.md|4096|c9d0e1f2
F4|TEST|on_request|gh:tests/test_main.py|1536|11223344
...
The compiler just writes one line per file.

6. Generate hot memory (@M)

This is where you decide what actually goes into the prompt.

You have two main patterns:

Semantic state keys (task, cursor, etc.)
Inline content for hot files (usually compressed or delta‑encoded)
Example:

@M
CTX:⟦task=refactor;entry=F1;cursor=128⟧
F1:⟦def main(): ...⟧
CFG:⟦env=prod;feature_x=true⟧
DOC:⟦High-level architecture summary…⟧
How you fill ⟦…⟧ is up to you:

Full file content (for small files)
Truncated content (first N lines)
Summaries (LLM‑generated once, then cached)
Deltas vs previous version (for iterative coding sessions)
The compiler can have a hot payload strategy:

build_hot_payload(resource) -> string
7. Generate cold blocks (@C) [optional but powerful]

If you want AIMF to be self‑contained, you can embed cold blocks:

@C F1
⟦<compressed src/main.py>⟧

@C F3
⟦<compressed README.md>⟧
If you prefer external storage (GitHub, S3, local FS), you can skip @C entirely and rely on Location to fetch content when needed.

A nice hybrid:

For small files (< N bytes): embed as cold blocks.
For large files: keep only Location, fetch on demand.
Putting it all together: end‑to‑end compiler sketch

High‑level algorithm:

Clone / fetch repo (or assume local path).
Walk directory tree, ignore .git/, build file list.
For each file:
classify ResourceType
compute size_bytes, hash
decide LoadWhen
assign ResourceID
Decide hot set:
apply policy to pick N hot resources
compute HOT = len(hot_resources)
Emit @H.
Emit @I with one ResourceLine per file.
Emit @M:
CTX entry (current task, entry file, etc.)
one entry per hot resource (ID → payload).
Optionally emit @C:
one ColdBlock per resource you want embedded.
Output: a single AIMF document you can feed to Claude Code (or any LLM) as the project context spine.

If you’d like, next step we can:

Design a concrete hot‑payload strategy (e.g. “first 200 lines + summary”)
Sketch an actual Python CLI (aimf-compile <repo> > project.aimf)
Or define AIMF v1.1 with a tiny META section for repo name, branch, commit, etc.