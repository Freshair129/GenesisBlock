#!/usr/bin/env python3
"""Validate GenesisBlockDB documentation metadata and local links.

The repository intentionally contains legacy/interview Markdown without
frontmatter. Those files are allowed to remain outside the canonical registry;
registered documents and any document that declares frontmatter are checked.
"""

import pathlib
import re
import sys
import tempfile


VALID_STATUSES = {
    "active",
    "accepted",
    "beta",
    "candidate",
    "canonical",
    "current",
    "deprecated",
    "draft",
    "evidence",
    "historical",
    "need review",
    "proposed",
    "reference",
    "stable",
    "superseded",
    "under review",
    "unstable",
}


def _clean_scalar(value: str) -> str:
    value = value.strip()
    if len(value) >= 2 and value[0] == value[-1] and value[0] in {'"', "'"}:
        return value[1:-1]
    return value


def parse_frontmatter(content: str) -> dict[str, str] | None:
    """Extract top-level scalar frontmatter fields from a Markdown file."""
    opening = re.match(r"^---\s*\n", content)
    if not opening:
        return None

    closing = re.compile(r"^---\s*$", re.MULTILINE).search(content, opening.end())
    if not closing:
        return None

    fields: dict[str, str] = {}
    for line in content[opening.end() : closing.start()].splitlines():
        match = re.match(r"^([A-Za-z_][\w-]*):\s*(.*?)\s*$", line)
        if match:
            fields[match.group(1)] = _clean_scalar(match.group(2))
    return fields


def _code_cell(value: str) -> str:
    value = value.strip()
    if value.startswith("`") and value.endswith("`"):
        return value[1:-1]
    return _clean_scalar(value)


def parse_registry(root: pathlib.Path) -> list[dict[str, str]]:
    registry = root / "DOC-REGISTRY.md"
    if not registry.is_file():
        return []

    rows: list[dict[str, str]] = []
    headers: list[str] = []
    for line in registry.read_text(encoding="utf-8", errors="replace").splitlines():
        if not line.startswith("|"):
            continue
        cells = [cell.strip() for cell in line.strip().strip("|").split("|")]
        if cells and cells[0].lower() == "role":
            headers = [cell.lower() for cell in cells]
            continue
        if len(cells) < 6 or not cells[1].startswith("`") or not cells[-1].startswith("`"):
            continue
        owner = ""
        if "owner" in headers:
            owner = _clean_scalar(cells[headers.index("owner")])
        rows.append(
            {
                "doc_id": _code_cell(cells[1]),
                "version": _code_cell(cells[2]),
                "status": _clean_scalar(cells[3]).lower(),
                "owner": owner,
                "path": _code_cell(cells[-1]),
            }
        )
    return rows


def _validate_status(path: pathlib.Path, fields: dict[str, str], violations: list[str]) -> None:
    status = fields.get("status")
    if status is None:
        violations.append("no status key in frontmatter")
        return

    normalized = status.strip().lower()
    if normalized.startswith("superseded-by:"):
        target_name = status.split(":", 1)[1].strip()
        target_path = path.parent / target_name
        if not target_path.is_file():
            violations.append(f"superseded-by target missing: {target_name}")
        return

    if normalized not in VALID_STATUSES:
        violations.append(f"invalid status value: {status}")
        return

    if normalized == "superseded":
        target_name = fields.get("superseded_by")
        if target_name:
            target_path = path.parent / target_name
            if not target_path.is_file():
                violations.append(f"superseded_by target missing: {target_name}")


MARKDOWN_LINK = re.compile(r"(?<!!)\[[^\]]*\]\(([^)\s]+)(?:\s+[^)]*)?\)")


def _validate_links(path: pathlib.Path, content: str, violations: list[str]) -> None:
    for raw_target in MARKDOWN_LINK.findall(content):
        target = raw_target.strip("<>")
        if not target or target.startswith(("#", "http://", "https://", "mailto:", "tel:", "file:")):
            continue
        if re.match(r"^[A-Za-z]:[\\/]", target):
            continue
        target = target.split("#", 1)[0].split("?", 1)[0]
        if not target:
            continue
        if not (path.parent / target).resolve().exists():
            violations.append(f"broken relative link: {raw_target}")


def validate_dir(root: pathlib.Path) -> tuple[list[tuple[pathlib.Path, str]], int]:
    violations: list[tuple[pathlib.Path, str]] = []
    paths = sorted(root.rglob("*.md"))
    fields_by_path: dict[pathlib.Path, dict[str, str]] = {}
    doc_ids: dict[str, pathlib.Path] = {}

    for path in paths:
        try:
            content = path.read_text(encoding="utf-8", errors="replace")
        except Exception as exc:
            violations.append((path, f"read error: {exc}"))
            continue

        fields = parse_frontmatter(content)
        if fields is None:
            continue
        fields_by_path[path] = fields
        local: list[str] = []
        _validate_status(path, fields, local)
        _validate_links(path, content, local)
        for violation in local:
            violations.append((path, violation))

        doc_id = fields.get("doc_id")
        if doc_id:
            previous = doc_ids.get(doc_id)
            if previous:
                violations.append((path, f"duplicate doc_id: {doc_id} (also {previous})"))
            else:
                doc_ids[doc_id] = path

    registry_path = root / "DOC-REGISTRY.md"
    if not registry_path.is_file():
        violations.append((root, "DOC-REGISTRY.md is missing"))
    else:
        registry_ids: set[str] = set()
        for row in parse_registry(root):
            # Registry paths are repository-root-relative (for example
            # docs/API_REFERENCE.md), while this validator receives docs/.
            path = root.parent / row["path"]
            if row["doc_id"] in registry_ids:
                violations.append((registry_path, f"duplicate registry doc_id: {row['doc_id']}"))
            registry_ids.add(row["doc_id"])
            if not path.is_file():
                violations.append((registry_path, f"registered path missing: {row['path']}"))
                continue
            fields = fields_by_path.get(path)
            if fields is None:
                violations.append((path, "registered document has no frontmatter"))
                continue
            keys = ("doc_id", "version", "status")
            if row["owner"]:
                keys += ("owner",)
            for key in keys:
                actual = fields.get(key)
                expected = row[key]
                if actual is None:
                    violations.append((path, f"frontmatter missing {key}; registry expects {expected}"))
                elif actual.strip().lower() != expected.strip().lower():
                    violations.append((path, f"frontmatter {key}={actual!r} disagrees with registry {expected!r}"))

    return violations, len(paths)


def self_test() -> int:
    with tempfile.TemporaryDirectory() as tmpdir:
        root = pathlib.Path(tmpdir)
        (root / "target.md").write_text("---\nstatus: current\n---\n", encoding="utf-8")
        (root / "valid.md").write_text(
            "---\nstatus: accepted\ndoc_id: VALID\n---\n[Target](target.md)\n",
            encoding="utf-8",
        )
        (root / "bad.md").write_text("---\nstatus: invalid\n---\n[Missing](missing.md)\n", encoding="utf-8")
        violations, count = validate_dir(root)
        if count != 3 or len(violations) != 3:
            print(f"SELF-TEST FAIL: expected 3 violations in 3 files, got {len(violations)}")
            return 1
        print("SELF-TEST PASS")
        return 0


def main() -> int:
    if len(sys.argv) == 2 and sys.argv[1] == "--self-test":
        return self_test()
    if len(sys.argv) != 2:
        print(f"Usage: {sys.argv[0]} <docs_dir> or --self-test", file=sys.stderr)
        return 1

    root = pathlib.Path(sys.argv[1])
    if not root.is_dir():
        print(f"Error: {root} is not a directory", file=sys.stderr)
        return 1

    violations, count = validate_dir(root)
    for path, reason in violations:
        print(f"VIOLATION {path.relative_to(root)}: {reason}")
    print(f"{len(violations)} violations in {count} files")
    return 1 if violations else 0


if __name__ == "__main__":
    sys.exit(main())
