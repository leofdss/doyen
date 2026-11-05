#!/usr/bin/env python3
"""Check the OpenSpec changes against each other (conflicts that `openspec validate` cannot see).

`openspec validate --all --strict` checks each change alone (format, scenarios, lengths).
This script checks the rules in TODO.md ("Rules that keep the changes free of conflicts"):

1. One capability belongs to one open change, and each proposal lists exactly its spec files.
2. No MODIFIED / REMOVED / RENAMED delta for a capability that is not in openspec/specs/.
3. Requirement names are unique across the open changes (warning), and scenario names are
   unique inside a capability (error: tests name their scenario as `<capability> / <scenario>`).
4. Every DOYEN_* code is in the code table of TODO.md, and its owner capability names it.
5. Every capability and change named in backticks exists.
6. The "Depends on" line of each proposal matches TODO.md, has no cycle, and TODO.md lists
   every dependency before the change. The capabilities column matches the spec files.
7. Minimum versions and default settings have one value everywhere.
8. A capitalised name in double quotes in a tasks.md ("Master dies") is a scenario or a
   requirement of some spec (tasks cite the tests they make pass by these names).

Usage: python3 scripts/check-specs.py   (from the repo root; exit code 1 on an error)
"""

from __future__ import annotations

import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
CHANGES = ROOT / "openspec" / "changes"
MAIN_SPECS = ROOT / "openspec" / "specs"
TODO = ROOT / "TODO.md"

CAP_SUFFIXES = ("-core", "-reporting", "-config", "-contract", "-api", "-adapter", "-binding",
                "-release", "-checks")
NOT_CODES = re.compile(r"^DOYEN_(TEST_\w*|GO_DEPLOY_KEY)$")
NOT_CAPS = {"futures-core"}  # crate names that look like capabilities

errors: list[str] = []
warnings: list[str] = []


def rel(path: Path) -> str:
    return str(path.relative_to(ROOT))


def open_changes() -> dict[str, Path]:
    return {p.name: p for p in sorted(CHANGES.iterdir()) if p.is_dir() and p.name != "archive"}


def todo_table(heading: str) -> list[list[str]]:
    """Rows of the first Markdown table after a heading of TODO.md (cells stripped)."""
    text = TODO.read_text()
    start = text.find(heading)
    if start < 0:
        errors.append(f"TODO.md: heading {heading!r} not found")
        return []
    rows = []
    for line in text[start:].splitlines()[1:]:
        line = line.strip()
        if line.startswith("|"):
            cells = [c.strip() for c in line.strip("|").split("|")]
            if not set("".join(cells)) <= set("-: "):
                rows.append(cells)
        elif rows:
            break
    return rows[1:]  # without the header row


def ticks(text: str) -> list[str]:
    return re.findall(r"`([^`\n]+)`", text)


def main() -> int:
    changes = open_changes()
    main_caps = {p.name for p in MAIN_SPECS.iterdir() if p.is_dir()} if MAIN_SPECS.exists() else set()

    # 1. capabilities per change, and the proposal's list
    cap_owner: dict[str, str] = {}
    caps_of: dict[str, set[str]] = {}
    for name, path in changes.items():
        spec_dir = path / "specs"
        caps = {p.parent.relative_to(spec_dir).as_posix() for p in spec_dir.glob("**/spec.md")} \
            if spec_dir.exists() else set()
        caps_of[name] = caps
        for cap in caps:
            if cap in cap_owner:
                errors.append(f"capability `{cap}` is in two open changes: {cap_owner[cap]}, {name}")
            cap_owner.setdefault(cap, name)
        proposal = (path / "proposal.md").read_text()
        new_part = proposal.split("### New Capabilities", 1)[-1].split("### Modified", 1)[0]
        listed = set(re.findall(r"^- `([^`]+)`", new_part, re.M))
        if listed != caps:
            errors.append(f"{name}: proposal lists {sorted(listed)}, spec files are {sorted(caps)}")

    known_caps = set(cap_owner) | main_caps

    # 2. deltas on capabilities that do not exist yet; 3. requirement names
    req_seen: dict[str, str] = {}
    for name, path in changes.items():
        for spec in path.glob("specs/**/spec.md"):
            cap = spec.parent.relative_to(path / "specs").as_posix()
            text = spec.read_text()
            for op in ("MODIFIED", "REMOVED", "RENAMED"):
                if f"## {op} Requirements" in text and cap not in main_caps:
                    errors.append(f"{rel(spec)}: {op} delta for `{cap}`, which is not in openspec/specs/")
            scenarios = [s.strip().lower() for s in re.findall(r"^#### Scenario: (.+)$", text, re.M)]
            for s in sorted({s for s in scenarios if scenarios.count(s) > 1}):
                errors.append(f"{rel(spec)}: scenario {s!r} appears twice (tests name scenarios per capability)")
            for req in re.findall(r"^### Requirement: (.+)$", text, re.M):
                key = req.strip().lower()
                if key in req_seen and req_seen[key] != f"{name}/{cap}":
                    warnings.append(f"requirement {req!r} in {req_seen[key]} and {name}/{cap}")
                req_seen.setdefault(key, f"{name}/{cap}")

    # 4. codes
    owners = {}
    for row in todo_table("**Each code has one owner:**"):
        code, owner = row[0].strip("`"), row[1].strip("`")
        owners[code] = owner
        if owner not in known_caps:
            errors.append(f"TODO.md: owner `{owner}` of {code} is not a capability")
    files = [p for p in CHANGES.glob("*/**/*.md") if "archive" not in p.parts]
    files += [ROOT / "README.md", TODO]
    for f in files:
        for code in set(re.findall(r"\bDOYEN_[A-Z0-9_]+\b", f.read_text())):
            if not NOT_CODES.match(code) and code not in owners:
                errors.append(f"{rel(f)}: {code} is not in the code table of TODO.md")
    for code, owner in owners.items():
        spec = CHANGES / cap_owner.get(owner, "") / "specs" / owner / "spec.md"
        if owner in main_caps:
            spec = MAIN_SPECS / owner / "spec.md"
        if not spec.exists() or code not in spec.read_text():
            errors.append(f"{code}: its owner `{owner}` does not name it")

    # 5. names in backticks
    for f in files:
        for tok in ticks(f.read_text()):
            if re.fullmatch(r"[a-z][a-z0-9]*(-[a-z0-9]+)+", tok):
                if tok.endswith(CAP_SUFFIXES) and tok not in known_caps and tok not in changes \
                        and tok not in NOT_CAPS:
                    errors.append(f"{rel(f)}: unknown capability `{tok}`")
                if re.match(r"(add|publish)-", tok) and tok not in changes \
                        and not any(p.name.endswith(tok) for p in (CHANGES / "archive").glob("*")):
                    errors.append(f"{rel(f)}: unknown change `{tok}`")

    # 6. dependencies and the order table
    order_rows = todo_table("## Order")
    order = [r[1].strip("`") for r in order_rows]
    index = {c: i for i, c in enumerate(order)}
    for c in changes:
        if c not in index:
            errors.append(f"TODO.md: change {c} is not in the order table")
    deps: dict[str, set[str]] = {}
    for name, path in changes.items():
        m = re.search(r"Depends on:(.+?)(?:\n\n|$)", (path / "proposal.md").read_text(), re.S)
        deps[name] = {t for t in ticks(m.group(1)) if t in changes} if m else set()
        if not m:
            errors.append(f"{name}: proposal has no 'Depends on:' line")
    for row in order_rows:
        name = row[1].strip("`")
        if name not in changes:
            continue
        nums = re.findall(r"\d+", row[3])
        todo_deps = {order[int(n) - 1] for n in nums if 0 < int(n) <= len(order)}
        if todo_deps != deps[name]:
            errors.append(f"{name}: TODO.md depends on {sorted(todo_deps)}, proposal on {sorted(deps[name])}")
        for d in deps[name]:
            if index.get(d, -1) >= index[name]:
                errors.append(f"{name}: depends on {d}, which is not before it in TODO.md")
        todo_caps = set(ticks(row[4]))
        if todo_caps != caps_of[name]:
            errors.append(f"{name}: TODO.md capabilities {sorted(todo_caps)}, spec files {sorted(caps_of[name])}")
    state: dict[str, int] = {}

    def visit(n: str, stack: list[str]) -> None:
        if state.get(n) == 1:
            errors.append("dependency cycle: " + " -> ".join(stack + [n]))
        elif state.get(n) != 2:
            state[n] = 1
            for d in deps.get(n, ()):
                visit(d, stack + [n])
            state[n] = 2

    for n in changes:
        visit(n, [])

    # 7. one value for each minimum version and default
    facts: dict[str, dict[str, set[str]]] = {}
    patterns = {
        "min version": [r"\b(Redis|MongoDB|PostgreSQL)\s+(\d+(?:\.\d+)?)\s+or\s+newer",
                        r"older\s+than\s+(Redis|MongoDB|PostgreSQL)\s+(\d+(?:\.\d+)?)",
                        r"\b(Redis|MongoDB|PostgreSQL)\s+server\s+older\s+than\s+(\d+(?:\.\d+)?)",
                        r"\b(Redis|MongoDB|PostgreSQL)\s+(\d+(?:\.\d+)?)\+",
                        r"\b(Rust|Go)\s+(1\.\d+)\b"],
        "default": [r"`(timeoutMs|renewIntervalMs)` \(default (\d+)\)",
                    r"`(timeoutMs|renewIntervalMs)`\s*\|\s*`?(\d+)"],
    }
    for f in files:
        text = f.read_text()
        for kind, pats in patterns.items():
            for pat in pats:
                for what, value in re.findall(pat, text):
                    facts.setdefault(f"{kind} {what}", {}).setdefault(value, set()).add(rel(f))
    for fact, values in sorted(facts.items()):
        if len(values) > 1:
            detail = "; ".join(f"{v} in {', '.join(sorted(fs))}" for v, fs in values.items())
            errors.append(f"{fact} has more than one value: {detail}")

    # 8. scenario and requirement names quoted in tasks.md exist in some spec
    names: set[str] = set()
    for spec in list(CHANGES.glob("*/specs/**/spec.md")) + list(MAIN_SPECS.glob("**/spec.md")):
        if "archive" in spec.parts:
            continue
        text = spec.read_text()
        names |= {n.strip().lower() for n in re.findall(r"^#{3,4} (?:Requirement|Scenario): (.+)$", text, re.M)}
    for name, path in changes.items():
        tasks = path / "tasks.md"
        if not tasks.exists():
            continue
        for quoted in re.findall(r"\"([A-Z][^\"\n]+)\"", tasks.read_text()):
            q = quoted.strip().lower()
            if q.endswith("…"):
                ok = any(n.startswith(q[:-1].strip()) for n in names)
            else:
                ok = q in names
            if not ok:
                errors.append(f"{name}/tasks.md: \"{quoted}\" is not a scenario or requirement name")

    for w in warnings:
        print(f"warning: {w}")
    for e in errors:
        print(f"error: {e}")
    print(f"{len(changes)} changes, {len(cap_owner)} capabilities, {len(owners)} codes, "
          f"{len(facts)} shared values: {len(errors)} errors, {len(warnings)} warnings")
    return 1 if errors else 0


if __name__ == "__main__":
    sys.exit(main())
