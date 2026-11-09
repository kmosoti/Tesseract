#!/usr/bin/env python3
"""Check the linked Markdown graph without a proprietary backend or YAML package."""
import datetime
import pathlib
import re
import sys

ROOT = pathlib.Path(__file__).resolve().parents[1]
errors = []
ids = set()
index = (ROOT / "knowledge/README.md").read_text()
for folder, prefix, kind in [("decisions", "D", "decision"), ("research", "R", "research"), ("observations", "O", "observation")]:
    for path in sorted((ROOT / "knowledge" / folder).glob("*.md")):
        text = path.read_text()
        parts = text.split("---", 2)
        if len(parts) != 3 or parts[0].strip():
            errors.append(f"{path}: missing front matter")
            continue
        fields = dict(re.findall(r"^([a-z_]+):\s*(.+)$", parts[1], re.M))
        for key in ["id", "type", "status", "updated", "added_by", "human_ratified"]:
            if key not in fields:
                errors.append(f"{path}: missing {key}")
        identifier = fields.get("id", "")
        if not re.fullmatch(prefix + r"-\d{4}", identifier) or identifier in ids or not path.name.startswith(identifier + "-"):
            errors.append(f"{path}: invalid, duplicate, or mismatched ID")
        ids.add(identifier)
        if fields.get("type") != kind or fields.get("added_by") not in {"agent", "human", "mixed"} or fields.get("human_ratified") not in {"true", "false"}:
            errors.append(f"{path}: invalid type or provenance")
        try:
            datetime.date.fromisoformat(fields.get("updated", ""))
        except ValueError:
            errors.append(f"{path}: invalid date")
        if fields.get("human_ratified") == "true":
            if not fields.get("ratified_by") or not fields.get("ratified_on"):
                errors.append(f"{path}: ratification lacks human/date")
        for section in ["Observed", "Inferred", "Hypothesis", "Unknown", "Relations", "Revisit when"]:
            if f"## {section}\n" not in text:
                errors.append(f"{path}: missing {section}")
        if identifier not in index:
            errors.append(f"{path}: not linked from index")
        if not re.search(r"^- \*[a-z-]+\*: \[", text, re.M):
            errors.append(f"{path}: no typed relation")
        for target in re.findall(r"\]\(([^)]+)\)", text):
            if "://" in target or target.startswith("#"):
                continue
            if not (path.parent / target.split("#")[0]).resolve().exists():
                errors.append(f"{path}: broken link {target}")
if errors:
    print("\n".join(errors), file=sys.stderr)
    sys.exit(1)
print(f"Validated {len(ids)} linked knowledge records")
