#!/usr/bin/env python3
"""Validate the agent-executable foundation's mechanical invariants."""

from __future__ import annotations

import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]

REQUIRED_FILES = [
    "ARCHITECTURE.md", "EXECUTION_PLAN.md", "EXECUTION_PLAN_DETAILED.md",
    "AGENT_SYSTEM.md", "AGENT_PROMPT.md", "AGENT_SKILLS.md", "TASK_SPEC.md",
    "DEFINITION_OF_READY.md", "BACKLOG.md", "ACCEPTANCE_MATRIX.md",
    "FOUNDATION_EVIDENCE.md", "FOUNDATION_READINESS_STATES.md",
    "PHASE_0_AGENT_READINESS_GATE.md", "PHASE_0_5_DOMAIN_FINALIZATION.md",
    "PHASE_0_6_COMMERCIAL_REGULATORY_FINALIZATION.md", "DATABASE_RULES.md",
    "DOMAIN_CONTRACTS.md", "SECURITY_MODEL.md", "SECURITY_SCAN_POLICY.md",
    "DEPENDENCY_POLICY.md", "SYNC_SPEC.md", "RELEASE_SPEC.md",
    "PRODUCT_SPEC.md", "UI_SPEC.md", "ADR_SYSTEM.md", "COMMERCIAL_PROVIDER_MATRIX.md",
    "UI_CLOUD_EXECUTION_PLAN.md", "INDUSTRY_EXECUTION_PLAN.md",
    "CAPABILITY_MATRIX.md", "TASK_DEPENDENCY_GRAPH.md",
    ".github/workflows/ci.yml", ".github/workflows/foundation-evidence.yml",
]

REQUIRED_MARKERS = {
    "TASK_SPEC.md": [
        "Identity", "Objective", "Dependencies", "Contracts", "Business rules",
        "Acceptance criteria", "Tests required", "Evidence", "Failure/recovery",
        "Rollback", "Definition of Done",
    ],
    "AGENT_SYSTEM.md": [
        "agent operating system", "before every task", "evidence", "gate",
    ],
    "AGENT_PROMPT.md": [
        "readiness", "phase", "evidence", "TASK_DEPENDENCY_GRAPH.md",
        "CAPABILITY_MATRIX.md", "INDUSTRY_EXECUTION_PLAN.md", "UI_CLOUD_EXECUTION_PLAN.md",
    ],
    "AGENT_SKILLS.md": ["Core engineering", "Security", "Financial correctness", "Agent behavior skills"],
    "FOUNDATION_EVIDENCE.md": ["commit", "CI", "evidence"],
    "FOUNDATION_READINESS_STATES.md": [
        "FOUNDATION_DESIGNED", "FOUNDATION_VERIFIED", "AGENT_IMPLEMENTATION_READY",
        "PRODUCTION_READY", "LAUNCH_READY",
    ],
    "CAPABILITY_MATRIX.md": ["Industry", "Capability", "Preset rules"],
    "TASK_DEPENDENCY_GRAPH.md": ["hard dependency", "phase gate", "next task"],
    "UI_CLOUD_EXECUTION_PLAN.md": ["UI rules", "Supabase", "Webhooks"],
    "INDUSTRY_EXECUTION_PLAN.md": ["Universal industry sequence", "acceptance/evidence", "shared financial"],
}

TASK_RE = re.compile(r"\bF\d+(?:\.\d+)?\.\d{2}\b")


def fail(errors: list[str], message: str) -> None:
    errors.append(message)


def read(path: Path) -> str:
    return path.read_text(encoding="utf-8")


def validate_required_files(errors: list[str]) -> None:
    for rel in REQUIRED_FILES:
        if not (ROOT / rel).is_file():
            fail(errors, f"Missing required foundation file: {rel}")


def validate_markers(errors: list[str]) -> None:
    for rel, markers in REQUIRED_MARKERS.items():
        path = ROOT / rel
        if not path.is_file():
            continue
        text = read(path).lower()
        for marker in markers:
            if marker.lower() not in text:
                fail(errors, f"{rel} is missing required contract marker: {marker}")


def validate_backlog(errors: list[str]) -> set[str]:
    path = ROOT / "BACKLOG.md"
    if not path.is_file():
        return set()

    text = read(path)
    ids = TASK_RE.findall(text)
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

    for number, line in enumerate(text.splitlines(), 1):
        stripped = line.strip()
        if stripped.startswith("-") and re.match(r"^-\s+F", stripped):
            token = stripped[1:].strip().split(maxsplit=1)[0]
            if not TASK_RE.fullmatch(token):
                fail(errors, f"Malformed task ID at BACKLOG.md:{number}: {token}")

    return seen


def validate_task_references(errors: list[str], task_ids: set[str]) -> None:
    if not task_ids:
        return
    ignored_dirs = {"node_modules", "target", ".git"}
    for path in ROOT.rglob("*.md"):
        if any(part in ignored_dirs for part in path.parts):
            continue
        for task_id in sorted(set(TASK_RE.findall(read(path)))):
            if task_id not in task_ids:
                fail(errors, f"{path.relative_to(ROOT)} references unknown task ID: {task_id}")


def validate_markdown_links(errors: list[str]) -> None:
    link_re = re.compile(r"\[[^\]]+\]\(([^)]+)\)")
    ignored_dirs = {"node_modules", "target", ".git"}
    for path in ROOT.rglob("*.md"):
        if any(part in ignored_dirs for part in path.parts):
            continue
        for raw_target in link_re.findall(read(path)):
            target = raw_target.strip().split()[0].strip("<>")
            if not target or target.startswith(("http://", "https://", "mailto:", "#")):
                continue
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
        for marker in ("before every task", "evidence", "gate", "definition of ready"):
            if marker not in text:
                fail(errors, f"AGENT_SYSTEM.md missing operational rule: {marker}")
    if prompt.is_file():
        text = read(prompt).lower()
        for marker in (
            "agent_system.md", "task_spec.md", "definition_of_ready.md", "backlog.md",
            "task_dependency_graph.md", "capability_matrix.md", "industry_execution_plan.md",
            "ui_cloud_execution_plan.md",
        ):
            if marker not in text:
                fail(errors, f"AGENT_PROMPT.md does not reference required control document: {marker}")


def validate_workflows(errors: list[str]) -> None:
    ci = ROOT / ".github/workflows/ci.yml"
    evidence = ROOT / ".github/workflows/foundation-evidence.yml"

    if not ci.is_file():
        fail(errors, "Missing .github/workflows/ci.yml")
    else:
        text = read(ci)
        for marker in [
            "pull_request", "cancel-in-progress: true", "foundation-validation",
            ".github/scripts/validate_foundation.py", "cargo check", "cargo test",
            "npm run build", "secret-scan", "github.event.pull_request.head.sha",
        ]:
            if marker not in text:
                fail(errors, f"CI workflow missing required foundation gate marker: {marker}")
        if "push:\n    branches: [main, foundation/v2]" in text:
            fail(errors, "CI workflow must not run duplicate push validation for foundation/v2")

    if not evidence.is_file():
        fail(errors, "Missing .github/workflows/foundation-evidence.yml")
    else:
        text = read(evidence)
        for marker in [
            "push:", "branches: [foundation/v2]", "gh run list", "--commit",
            "event pull_request", "github.sha", "git ls-remote origin refs/heads/foundation/v2",
            ".github/scripts/emit_foundation_evidence.py", "actions/upload-artifact@v4",
        ]:
            if marker not in text:
                fail(errors, f"Foundation evidence workflow missing required marker: {marker}")


def main() -> int:
    errors: list[str] = []
    validate_required_files(errors)
    validate_markers(errors)
    task_ids = validate_backlog(errors)
    validate_task_references(errors, task_ids)
    validate_markdown_links(errors)
    validate_agent_contracts(errors)
    validate_workflows(errors)

    if errors:
        print("FOUNDATION VALIDATION: FAIL")
        for error in errors:
            print(f"::error::{error}")
        return 1

    print(f"FOUNDATION VALIDATION: PASS ({len(task_ids)} unique backlog task IDs verified)")
    print("Structural validation passed; semantic architecture/regulatory correctness remains subject to explicit review gates.")
    return 0


if __name__ == "__main__":
    sys.exit(main())
