#!/usr/bin/env python3
"""Emit machine-readable Foundation Gate evidence for an explicitly verified commit."""

from __future__ import annotations

import json
import os
from datetime import datetime, timezone
from pathlib import Path

repository = os.environ["GITHUB_REPOSITORY"]
commit_sha = os.environ.get("FOUNDATION_EVIDENCE_SHA", os.environ["GITHUB_SHA"])
workflow_run_id = os.environ.get("FOUNDATION_EVIDENCE_RUN_ID", os.environ["GITHUB_RUN_ID"])
workflow_run_url = os.environ.get(
    "FOUNDATION_EVIDENCE_RUN_URL",
    f"{os.environ['GITHUB_SERVER_URL']}/{repository}/actions/runs/{workflow_run_id}",
)
event = os.environ.get("FOUNDATION_EVIDENCE_EVENT", os.environ["GITHUB_EVENT_NAME"])
ref = os.environ.get("FOUNDATION_EVIDENCE_REF", os.environ["GITHUB_REF"])

out = Path("foundation-evidence.json")
out.write_text(
    json.dumps(
        {
            "repository": repository,
            "commit_sha": commit_sha,
            "workflow_run_id": workflow_run_id,
            "workflow_run_url": workflow_run_url,
            "event": event,
            "ref": ref,
            "verified_at_utc": datetime.now(timezone.utc).isoformat(),
            "note": "This artifact records the exact branch-head commit whose CI validation completed successfully.",
        },
        indent=2,
        sort_keys=True,
    )
    + "\n",
    encoding="utf-8",
)
print(out.read_text(encoding="utf-8"))
