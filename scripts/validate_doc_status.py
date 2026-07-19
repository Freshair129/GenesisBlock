#!/usr/bin/env python3
import sys
import re
import pathlib
import tempfile
import shutil

def parse_frontmatter(content: str) -> str | None:
    """Extract the status field from YAML frontmatter.

    Frontmatter is a block between the first line '---' and the next line '---'.
    Returns the status string if found, None otherwise.
    """
    # Find the frontmatter block
    fm_match = re.match(r'---\s*\n', content)
    if not fm_match:
        return None

    start = fm_match.end()
    end_match = re.compile(r'^---\s*$', re.MULTILINE).search(content, start)
    if not end_match:
        return None

    fm_block = content[start:end_match.start()].strip()

    # Look for a line starting with 'status:'
    status_match = re.search(r'^status:\s*(.+)$', fm_block, re.MULTILINE)
    if not status_match:
        return None

    return status_match.group(1).strip()

def validate_file(path: pathlib.Path) -> list[str]:
    """Validate a single Markdown file. Returns list of violation strings."""
    violations = []

    try:
        content = path.read_text(encoding="utf-8", errors="replace")
    except Exception as e:
        return [f"read error: {e}"]

    status = parse_frontmatter(content)

    if status is None:
        if re.search(r'^---\s*$', content, re.MULTILINE):
            violations.append("no status key in frontmatter")
        else:
            violations.append("no frontmatter block")
        return violations

    # Check if status is one of the valid forms
    if status == "current":
        return violations
    if status == "historical":
        return violations

    m = re.match(r"^superseded-by:\s*(.+)$", status)
    if not m:
        violations.append(f"invalid status value: {status}")
        return violations

    target_name = m.group(1).strip()
    target_path = path.parent / target_name

    if not target_path.is_file():
        violations.append(f"superseded-by target missing: {target_name}")

    return violations

def validate_dir(root: pathlib.Path) -> tuple[list[tuple[pathlib.Path, str]], int]:
    """Validate all .md files in root. Returns (violations, count)."""
    violations = []
    count = 0

    for path in root.rglob("*.md"):
        count += 1
        for v in validate_file(path):
            violations.append((path, v))

    return violations, count

def self_test() -> int:
    """Run self-test with 4 fixture files."""
    with tempfile.TemporaryDirectory() as tmpdir:
        root = pathlib.Path(tmpdir)

        # Fixture a: valid current
        (root / "a.md").write_text("---\nstatus: current\n---\n", encoding="utf-8")

        # Fixture b: valid superseded-by a.md
        (root / "b.md").write_text("---\nstatus: superseded-by: a.md\n---\n", encoding="utf-8")

        # Fixture c: frontmatter but no status
        (root / "c.md").write_text("---\nauthor: alice\n---\n", encoding="utf-8")

        # Fixture d: no frontmatter
        (root / "d.md").write_text("# No frontmatter\n", encoding="utf-8")

        violations, count = validate_dir(root)

        # Expect exactly 2 violations: c and d
        if len(violations) != 2:
            print(f"SELF-TEST FAIL: expected 2 violations, got {len(violations)}")
            return 1

        # Files a and b should be clean
        a_violations = [v for v in violations if v[0].name == "a.md"]
        b_violations = [v for v in violations if v[0].name == "b.md"]
        if a_violations or b_violations:
            print("SELF-TEST FAIL: a.md or b.md has violations")
            return 1

        print("SELF-TEST PASS")
        return 0

def main() -> int:
    if len(sys.argv) == 2 and sys.argv[1] == "--self-test":
        return self_test()

    if len(sys.argv) != 2:
        print(f"Usage: {sys.argv[0]} <docs_dir> [--self-test]", file=sys.stderr)
        return 1

    root = pathlib.Path(sys.argv[1])
    if not root.is_dir():
        print(f"Error: {root} is not a directory", file=sys.stderr)
        return 1

    violations, count = validate_dir(root)

    for path, reason in violations:
        rel = path.relative_to(root)
        print(f"VIOLATION {rel}: {reason}")

    print(f"{len(violations)} violations in {count} files")

    return 1 if violations else 0

if __name__ == "__main__":
    sys.exit(main())
