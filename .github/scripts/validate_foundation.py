#!/usr/bin/env python3
"""Validate the agent-executable foundation without judging prose quality.

This gate checks structural invariants that can be verified mechanically:
- required foundation files exist
- required contract sections exist
- backlog task IDs are unique and well-formed
- task references in foundation docs point to real backlog tasks
- relative Markdown links resolve
- readiness states and critical agent contracts are present
- no implementation phase is accidentally unlocked by a missing gate

It deliberately does NOT claim that architecture, business rules, tax policy,
or regulatory research are semantically correct. Those remain human/domain
review gates with explicit evidence requirements.
"""

from __future__ import annotations

import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]

REQUIRED_FILES = [
    "ARCHITECTURE.md",
    "EXECUTION_PLAN.md",
    "EXECUTION_PLAN_DETAILED.md",
    "AGENT_SYSTEM.md",
    "AGENT_PROMPT.md",
    "TASK_SPEC.md",
    "DEFINITION_OF_READY.md",
    "BACKLOG.md",
    "ACCEPTANCE_MATRIX.md",
    "FOUNDATION_EVIDENCE.md",
    "FOUNDATION_READINESS_STATES.md",
    "PHASE_0_AGENT_READINESS_GATE.md",
    "PHASE_0_5_DOMAIN_FINALIZATION.md",
    "PHASE_0_6_COMMERCIAL_REGULATORY_FINALIZATION.md",
    "DATABASE_RULES.md",
    "DOMAIN_CONTRACTS.md",
    "SECURITY_MODEL.md",
    "SYNC_SPEC.md",
    "RELEASE_SPEC.md",
    "PRODUCT_SPEC.md",
    "UI_SPEC.md",
]

REQUIRED_SECTIONS = {
    "TASK_SPEC.md": [
        "Identity",
        "Objective",
        "Dependencies",
        "Contracts",
        "Business rules",
        "Acceptance criteria",
        "Tests required",
        "Evidence",
        "Failure/recovery",
        "Rollback",
        "Definition of Done",
    ],
    "AGENT_SYSTEM.md": [
        "Operating",
        "Task",
        "Evidence",
        "Gate",
    ],
    "AGENT_PROMPT.md": ["readiness", "phase", "evidence"],
    "FOUNDATION_EVIDENCE.md": ["commit", "CI", "evidence"],
    "FOUNDATION_READINESS_STATES.md": [
        "FOUNDATION_DESIGNED",
        "FOUNDATION_VERIFIED",
        "AGENT_IMPLEMENTATION_READY",
        "PRODUCTION_READY",
        "LAUNCH_READY",
    ],
}

TASK_RE = re.compile(r"\bF(?:\d+|\d+\.\d+)\.\d+\b")
# Accepts F1.01, F0.5.01, F0.6.12, etc.  The stricter form below prevents
# accidental acceptance of malformed IDs such as F1.1.2.3.
STRICT_TASK_RE = re.compile(r"\bF\d+(?:\.\d+)?\.\d{2}\b")


def fail(errors: list[str], message: str) -> None:
    errors.append(message)


def read(path: Path) -> str:
    return path.read_text(encoding="utf-8")


def validate_required_files(errors: list[str]) -> None:
    for rel in REQUIRED_FILES:
        if not (ROOT / rel).is_file():
            fail(errors, f"Missing required foundation file: {rel}")


def validate_sections(errors: list[str]) -> None:
    for rel, sections in REQUIRED_SECTIONS.items():
        path = ROOT / rel
        if not path.is_file():
            continue
        text = read(path).lower()
        for section in sections:
            if section.lower() not in text:
                fail(errors, f"{rel} is missing required contract marker: {section}")


def validate_backlog(errors: list[str]) -> set[str]:
    path = ROOT / "BACKLOG.md"
    if not path.is_file():
        return set()

    text = read(path)
    ids = STRICT_TASK_RE.findall(text)
    if not ids:
        fail(errors, "BACKLOG.md contains no valid task IDs")
        return set()

    seen: set[str] = set()
    duplicates: set[str] = set()
    for task_id in ids:
        if task_id in seen:
            duplicates.add(task_id)
        seen.add(task_id)

    for task_id in sorted(duplicates):
        fail(errors, f"Duplicate task ID in BACKLOG.md: {task_id}")

    # Every task line must have an actual task ID as its first token after '-'.
    for number, line in enumerate(text.splitlines(), 1):
        stripped = line.strip()
        if stripped.startswith("-") and re.match(r"^-\s+F", stripped):
            token = stripped[1:].strip().split(maxsplit=1)[0]
            if not STRICT_TASK_RE.fullmatch(token):
                fail(errors, f"Malformed task ID at BACKLOG.md:{number}: {token}")

    return seen


def validate_task_references(errors: list[str], task_ids: set[str]) -> None:
    if not task_ids:
        return

    ignored_dirs = {"node_modules", "target", ".git"}
    for path in ROOT.rglob("*.md"):
        if any(part in ignored_dirs for part in path.parts):
            continue
        text = read(path)
        for task_id in sorted(set(TASK_RE.findall(text))):
            if task_id not in task_ids:
                fail(errors, f"{path.relative_to(ROOT)} references unknown task ID: {task_id}")


def validate_markdown_links(errors: list[str]) -> None:
    link_re = re.compile(r"\[[^\]]+\]\(([^)]+)\)")
    ignored_dirs = {"node_modules", "target", ".git"}

    for path in ROOT.rglob("*.md"):
        if any(part in ignored_dirs for part in path.parts):
            continue
        text = read(path)
        for raw_target in link_re.findall(text):
            target = raw_target.strip().split()[0].strip("<>")
            if not target or target.startswith(("http://", "https://", "mailto:", "#")):
                continue
            # Strip an optional fragment/query from relative file links.
            target = target.split("#", 1)[0].split("?", 1)[0]
            if not target:
                continue
            resolved = (path.parent / target).resolve()
            try:
                resolved.relative_to(ROOT.resolve())
            except ValueError:
                fail(errors, f"Markdown link escapes repository: {path.relative_to(ROOT)} -> {raw_target}")
                continue
            if not resolved.exists():
                fail(errors, f"Broken Markdown link: {path.relative_to(ROOT)} -> {raw_target}")


def validate_agent_contracts(errors: list[str]) -> None:
    agent = ROOT / "AGENT_SYSTEM.md"
    prompt = ROOT / "AGENT_PROMPT.md"
    if agent.is_file():
        text = read(agent).lower()
        for marker in ("one task", "evidence", "gate", "definition of ready"):
            if marker not in text:
                fail(errors, f"AGENT_SYSTEM.md missing operational rule: {marker}")
    if prompt.is_file():
        text = read(prompt).lower()
        for marker in ("agent_system.md", "task_spec.md", "definition_of_ready.md", "backlog.md"):
            if marker not in text:
                fail(errors, f"AGENT_PROMPT.md does not reference required control document: {marker}")


def validate_workflow(errors: list[str]) -> None:
    path = ROOT / ".github/workflows/ci.yml"
    if not path.is_file():
        fail(errors, "Missing .github/workflows/ci.yml")
        return
    text = read(path)
    required = [
        "foundation-validation",
        ".github/scripts/validate_foundation.py",
        "cargo check",
        "cargo test",
        "npm run build",
        "secret-scan",
    ]
    for marker in required:
        if marker not in text:
            fail(errors, f"CI workflow missing required foundation gate marker: {marker}")


def main() -> int:
    errors: list[str] = []
    validate_required_files(errors)
    validate_sections(errors)
    task_ids = validate_backlog(errors)
    validate_task_references(errors, task_ids)
    validate_markdown_links(errors)
    validate_agent_contracts(errors)
    validate_workflow(errors)

    if errors:
        print("FOUNDATION VALIDATION: FAIL")
        for error in errors:
            print(f"::error::{error}")
        return 1

    print(f"FOUNDATION VALIDATION: PASS ({len(task_ids)} unique backlog task IDs verified)")
    print("Structural validation passed; semantic architecture/regulatory correctness remains subject to its explicit review gates.")
    return 0


if __name__ == "__main__":
    sys.exit(main())
