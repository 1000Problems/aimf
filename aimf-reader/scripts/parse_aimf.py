#!/usr/bin/env python3
"""
AIMF Parser — Parse an AIMF v1 document and output structured JSON summary.

Usage:
    python parse_aimf.py <file.aimf>                # Full summary
    python parse_aimf.py <file.aimf> --index        # Just the resource index
    python parse_aimf.py <file.aimf> --hot          # Just hot memory entries
    python parse_aimf.py <file.aimf> --group G2     # Resources in group G2
    python parse_aimf.py <file.aimf> --search auth  # Search by path/hint
    python parse_aimf.py <file.aimf> --load F3      # Load resource F3 from working folder
    python parse_aimf.py <file.aimf> --stats        # Token/size statistics
"""

import sys
import json
import os
import re
from pathlib import Path
from collections import defaultdict


class AimfDocument:
    def __init__(self):
        self.header = {}
        self.groups = []
        self.resources = []
        self.hot_entries = []
        self.cold_blocks = []

    def to_summary(self):
        type_counts = defaultdict(int)
        group_counts = defaultdict(int)
        load_when_counts = defaultdict(int)
        total_size = 0

        for r in self.resources:
            type_counts[r["type"]] += 1
            group_counts[r["group_id"]] += 1
            load_when_counts[r["load_when"]] += 1
            total_size += r["size"]

        return {
            "header": self.header,
            "stats": {
                "total_resources": len(self.resources),
                "hot_entries": len(self.hot_entries),
                "cold_blocks": len(self.cold_blocks),
                "groups": len(self.groups),
                "total_size_bytes": total_size,
                "by_type": dict(type_counts),
                "by_group": dict(group_counts),
                "by_load_when": dict(load_when_counts),
            },
            "groups": self.groups,
            "hot_keys": [h["key"] for h in self.hot_entries],
        }

    def to_index(self):
        return [
            {
                "id": r["id"],
                "type": r["type"],
                "load_when": r["load_when"],
                "path": r["path"],
                "size": r["size"],
                "group": r["group_id"],
                "hint": r["hint"],
            }
            for r in self.resources
        ]

    def get_hot(self):
        return self.hot_entries

    def get_group(self, group_id):
        return [r for r in self.resources if r["group_id"] == group_id]

    def search(self, query):
        q = query.lower()
        return [
            r for r in self.resources
            if q in r["path"].lower() or q in r["hint"].lower()
        ]

    def load_resource(self, resource_id):
        """Load a resource's content from the working folder."""
        resource = next((r for r in self.resources if r["id"] == resource_id), None)
        if not resource:
            return {"error": f"Resource {resource_id} not found"}

        # Check cold blocks first
        cold = next((c for c in self.cold_blocks if c["id"] == resource_id), None)
        if cold:
            return {
                "id": resource_id,
                "path": resource["path"],
                "source": "cold_block",
                "content": cold["payload"],
            }

        # Load from working folder
        root = self.header.get("ROOT", ".")
        full_path = os.path.join(root, resource["path"])

        if resource["load_when"] == "never":
            return {
                "id": resource_id,
                "path": resource["path"],
                "source": "none",
                "content": f"[binary/never-load resource: {resource['path']}]",
            }

        try:
            with open(full_path, "r", encoding="utf-8", errors="replace") as f:
                content = f.read()
            return {
                "id": resource_id,
                "path": resource["path"],
                "source": "working_folder",
                "size": len(content),
                "content": content,
            }
        except FileNotFoundError:
            return {
                "id": resource_id,
                "path": resource["path"],
                "source": "not_found",
                "content": f"[file not found: {full_path}]",
            }
        except Exception as e:
            return {
                "id": resource_id,
                "path": resource["path"],
                "source": "error",
                "content": f"[error reading file: {e}]",
            }

    def get_stats(self):
        """Token and size statistics."""
        # Estimate tokens (~4 chars per token for code)
        index_chars = sum(
            len(f"{r['id']}|{r['type']}|{r['load_when']}|{r['path']}|{r['size']}|{r['hash']}|{r['group_id']}|{r['hint']}\n")
            for r in self.resources
        )
        hot_chars = sum(len(h["key"]) + len(h["payload"]) + 10 for h in self.hot_entries)
        header_chars = sum(len(f"{k}:{v}\n") for k, v in self.header.items())

        return {
            "estimated_tokens": {
                "header": header_chars // 4,
                "index": index_chars // 4,
                "hot_memory": hot_chars // 4,
                "total_spine": (header_chars + index_chars + hot_chars) // 4,
            },
            "sizes": {
                "total_project_bytes": sum(r["size"] for r in self.resources),
                "hot_content_chars": hot_chars,
                "index_chars": index_chars,
            },
            "compression_ratio": (
                sum(r["size"] for r in self.resources) / max(header_chars + index_chars + hot_chars, 1)
            ),
        }


def parse_aimf(text):
    """Parse an AIMF v1 document from text."""
    doc = AimfDocument()
    lines = text.split("\n")
    i = 0

    while i < len(lines):
        line = lines[i].rstrip()

        # Header section
        if line == "@H":
            i += 1
            while i < len(lines) and lines[i].strip() and not lines[i].startswith("@"):
                kv = lines[i].strip()
                if ":" in kv:
                    key, _, val = kv.partition(":")
                    doc.header[key] = val
                i += 1
            continue

        # Groups section
        if line == "@G":
            i += 1
            while i < len(lines) and lines[i].strip() and not lines[i].startswith("@"):
                parts = lines[i].strip().split("|", 2)
                if len(parts) == 3:
                    doc.groups.append({
                        "id": parts[0],
                        "label": parts[1],
                        "patterns": parts[2].split(","),
                    })
                i += 1
            continue

        # Index section
        if line == "@I":
            i += 1
            while i < len(lines) and lines[i].strip() and not lines[i].startswith("@"):
                parts = lines[i].strip().split("|")
                if len(parts) >= 7:
                    doc.resources.append({
                        "id": parts[0],
                        "type": parts[1],
                        "load_when": parts[2],
                        "path": parts[3],
                        "size": int(parts[4]) if parts[4].isdigit() else 0,
                        "hash": parts[5],
                        "group_id": parts[6],
                        "hint": parts[7] if len(parts) > 7 else "",
                    })
                i += 1
            continue

        # Hot memory entries
        if line.startswith("@M "):
            key = line[3:].strip()
            i += 1
            payload = _extract_payload(lines, i)
            doc.hot_entries.append({"key": key, "payload": payload["text"]})
            i = payload["end_line"]
            continue

        # Cold blocks
        if line.startswith("@C "):
            resource_id = line[3:].strip()
            i += 1
            payload = _extract_payload(lines, i)
            doc.cold_blocks.append({"id": resource_id, "payload": payload["text"]})
            i = payload["end_line"]
            continue

        i += 1

    return doc


def _extract_payload(lines, start):
    """Extract content between << and >> delimiters."""
    text_parts = []
    i = start
    started = False

    while i < len(lines):
        line = lines[i]

        if not started:
            if line.startswith("<<"):
                started = True
                rest = line[2:]
                # Check if payload ends on same line
                if rest.endswith(">>"):
                    return {"text": rest[:-2], "end_line": i + 1}
                text_parts.append(rest)
            i += 1
            continue

        if line.endswith(">>"):
            text_parts.append(line[:-2])
            return {"text": "\n".join(text_parts), "end_line": i + 1}

        text_parts.append(line)
        i += 1

    return {"text": "\n".join(text_parts), "end_line": i}


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

    doc = parse_aimf(text)

    # Handle flags
    if "--index" in sys.argv:
        print(json.dumps(doc.to_index(), indent=2))
    elif "--hot" in sys.argv:
        print(json.dumps(doc.get_hot(), indent=2))
    elif "--group" in sys.argv:
        idx = sys.argv.index("--group")
        if idx + 1 < len(sys.argv):
            group_id = sys.argv[idx + 1]
            print(json.dumps(doc.get_group(group_id), indent=2))
        else:
            print(json.dumps({"error": "Specify group ID"}))
    elif "--search" in sys.argv:
        idx = sys.argv.index("--search")
        if idx + 1 < len(sys.argv):
            query = sys.argv[idx + 1]
            print(json.dumps(doc.search(query), indent=2))
        else:
            print(json.dumps({"error": "Specify search query"}))
    elif "--load" in sys.argv:
        idx = sys.argv.index("--load")
        if idx + 1 < len(sys.argv):
            resource_id = sys.argv[idx + 1]
            print(json.dumps(doc.load_resource(resource_id), indent=2))
        else:
            print(json.dumps({"error": "Specify resource ID"}))
    elif "--stats" in sys.argv:
        print(json.dumps(doc.get_stats(), indent=2))
    else:
        print(json.dumps(doc.to_summary(), indent=2))


if __name__ == "__main__":
    main()
