#!/usr/bin/env python3
"""
AIMF v2 Parser — Parse navigation manifests and working context.

Usage:
    python parse_aimf.py <file.aimf>                 # Full summary
    python parse_aimf.py <file.aimf> --nav            # Navigation entries as JSON
    python parse_aimf.py <file.aimf> --ctx            # Working context as JSON
    python parse_aimf.py <file.aimf> --budget design  # Token budget for a mode
    python parse_aimf.py <file.aimf> --budget task:auth
    python parse_aimf.py <file.aimf> --search auth    # Search nav by id/path/about
    python parse_aimf.py <file.aimf> --load ARCH      # Load resource content by ID
"""

import sys
import json
import os
from collections import defaultdict


class AimfDocument:
    def __init__(self):
        self.nav = []     # list of dicts
        self.ctx = {}     # key -> value
        self.task = None   # markdown task body (if present)

    def summary(self):
        type_counts = defaultdict(int)
        total_tokens = 0
        design_tokens = 0

        for entry in self.nav:
            type_counts[entry["type"]] += 1
            total_tokens += entry["tokens"]
            if "design" in entry.get("load", ""):
                design_tokens += entry["tokens"]

        return {
            "resources": len(self.nav),
            "types": dict(type_counts),
            "total_tokens": total_tokens,
            "design_tokens": design_tokens,
            "ctx_keys": list(self.ctx.keys()),
            "has_task": self.task is not None,
        }

    def budget(self, mode):
        """Calculate token budget for a specific mode."""
        matching = []
        for entry in self.nav:
            load_hints = [h.strip() for h in entry.get("load", "").split(",")]
            if mode in load_hints or "task:*" in load_hints:
                matching.append(entry)

        total = sum(e["tokens"] for e in matching)
        return {
            "mode": mode,
            "resources": [
                {"id": e["id"], "path": e["path"], "tokens": e["tokens"], "about": e["about"]}
                for e in matching
            ],
            "total_tokens": total,
            "count": len(matching),
        }

    def search(self, query):
        """Search nav entries by id, path, or about."""
        q = query.lower()
        return [
            e for e in self.nav
            if q in e["id"].lower()
            or q in e["path"].lower()
            or q in e["about"].lower()
        ]

    def load_resource(self, resource_id, root=None):
        """Load a resource's content from the working folder."""
        entry = next((e for e in self.nav if e["id"] == resource_id), None)
        if not entry:
            return {"error": f"Resource '{resource_id}' not found in @NAV"}

        # Determine root from @CTX or argument
        if root is None:
            root = "."

        full_path = os.path.join(root, entry["path"])

        if entry["type"] == "DIR":
            # List directory contents
            if os.path.isdir(full_path):
                files = []
                for f in sorted(os.listdir(full_path)):
                    fp = os.path.join(full_path, f)
                    if os.path.isfile(fp):
                        size = os.path.getsize(fp)
                        files.append({"name": f, "size": size, "tokens": size // 4})
                return {
                    "id": resource_id,
                    "path": entry["path"],
                    "type": "directory",
                    "files": files,
                    "total_files": len(files),
                }
            return {"error": f"Directory not found: {full_path}"}

        # Read file
        try:
            with open(full_path, "r", encoding="utf-8", errors="replace") as f:
                content = f.read()
            return {
                "id": resource_id,
                "path": entry["path"],
                "type": "file",
                "tokens": len(content) // 4,
                "content": content,
            }
        except FileNotFoundError:
            return {"error": f"File not found: {full_path}"}
        except Exception as e:
            return {"error": str(e)}


def parse(text):
    """Parse an AIMF v2 document."""
    doc = AimfDocument()
    lines = text.split("\n")
    i = 0
    section = None

    while i < len(lines):
        line = lines[i].rstrip()

        # Section markers
        if line.strip() == "@NAV":
            section = "nav"
            i += 1
            continue
        elif line.strip() == "@CTX":
            section = "ctx"
            i += 1
            continue
        elif line.strip() == "---":
            # Everything after --- is the task markdown
            i += 1
            task_lines = []
            while i < len(lines):
                task_lines.append(lines[i])
                i += 1
            doc.task = "\n".join(task_lines).strip()
            break

        # Parse based on section
        if section == "nav":
            entry = parse_nav_line(line)
            if entry:
                doc.nav.append(entry)
        elif section == "ctx":
            if ":" in line and not line.startswith("#"):
                key, _, value = line.partition(":")
                doc.ctx[key.strip()] = value.strip()

        i += 1

    return doc


def parse_nav_line(line):
    """Parse a single @NAV entry line."""
    stripped = line.strip()
    if not stripped or stripped.startswith("id") or stripped.startswith("-"):
        return None  # Skip header and separator lines

    parts = [p.strip() for p in stripped.split("|")]
    if len(parts) < 6:
        return None

    try:
        tokens = int(parts[4])
    except ValueError:
        return None  # Skip non-data lines (e.g. header)

    return {
        "id": parts[0],
        "type": parts[1],
        "path": parts[2],
        "about": parts[3],
        "tokens": tokens,
        "load": parts[5] if len(parts) > 5 else "",
    }


def main():
    if len(sys.argv) < 2:
        print(__doc__)
        sys.exit(1)

    filepath = sys.argv[1]

    try:
        with open(filepath, "r", encoding="utf-8") as f:
            text = f.read()
    except FileNotFoundError:
        print(json.dumps({"error": f"File not found: {filepath}"}))
        sys.exit(1)

    doc = parse(text)

    if "--nav" in sys.argv:
        print(json.dumps(doc.nav, indent=2))
    elif "--ctx" in sys.argv:
        print(json.dumps(doc.ctx, indent=2))
    elif "--budget" in sys.argv:
        idx = sys.argv.index("--budget")
        mode = sys.argv[idx + 1] if idx + 1 < len(sys.argv) else "design"
        print(json.dumps(doc.budget(mode), indent=2))
    elif "--search" in sys.argv:
        idx = sys.argv.index("--search")
        query = sys.argv[idx + 1] if idx + 1 < len(sys.argv) else ""
        print(json.dumps(doc.search(query), indent=2))
    elif "--load" in sys.argv:
        idx = sys.argv.index("--load")
        rid = sys.argv[idx + 1] if idx + 1 < len(sys.argv) else ""
        print(json.dumps(doc.load_resource(rid), indent=2))
    else:
        print(json.dumps(doc.summary(), indent=2))


if __name__ == "__main__":
    main()
