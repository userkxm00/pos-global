#!/usr/bin/env python3
"""Emit machine-readable evidence for the exact GitHub Actions commit."""

from __future__ import annotations

import json
import os
from datetime import datetime, timezone
from pathlib import Path

out = Path("foundation-evidence.json")
out.write_text(
    json.dumps(
        {
            "repository": os.environ["GITHUB_REPOSITORY"],
            "commit_sha": os.environ["GITHUB_SHA"],
            "workflow_run_id": os.environ["GITHUB_RUN_ID"],
            "workflow_run_url": (
                f"{os.environ['GITHUB_SERVER_URL']}/{os.environ['GITHUB_REPOSITORY']}"
                f"/actions/runs/{os.environ['GITHUB_RUN_ID']}"
            ),
            "event": os.environ["GITHUB_EVENT_NAME"],
            "ref": os.environ["GITHUB_REF"],
            "verified_at_utc": datetime.now(timezone.utc).isoformat(),
            "note": "This artifact records the exact commit tested by this workflow run.",
        },
        indent=2,
        sort_keys=True,
    )
    + "\n",
    encoding="utf-8",
)
print(out.read_text(encoding="utf-8"))
