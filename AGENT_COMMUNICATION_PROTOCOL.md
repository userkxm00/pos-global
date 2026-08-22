# AGENT COMMUNICATION PROTOCOL — Zylo

## Purpose

This protocol standardizes how the implementation agent communicates task state, questions, blockers, verification, and evidence. It supplements `AGENT_SYSTEM.md`; it does not override repository contracts.

## 1. One task, one handoff

Every assigned task produces one final structured handoff. The agent must stop after the assigned task and must not select or start the next task without explicit authorization.

## 2. Required task status messages

Use one of:

- `[IN_PROGRESS]` — task is actively being implemented.
- `[BLOCKED]` — a hard blocker prevents safe progress.
- `[DECISION_REQUIRED]` — implementation can proceed only after an explicit product/architecture/security/financial/regulatory/provider decision.
- `[COMPLETE]` — acceptance criteria and required evidence are complete.
- `[UNVERIFIED]` — implementation exists but required verification could not be executed.
- `[REJECTED]` — implementation violates an approved contract and must be corrected.

Do not use “done”, “ready”, or “works” without the corresponding status and evidence.

## 3. Required completion handoff

```text
TASK: <exact task ID>
STATUS: <status>
SUMMARY: <what changed>
FILES_CHANGED: <list>
MIGRATIONS: <none/list>
TESTS: <commands + exact results>
SECURITY: <impact/checks>
EVIDENCE: <commit/CI/artifact links>
BLOCKERS: <none/list>
DECISIONS_REQUIRED: <none/list>
KNOWN_LIMITATIONS: <none/list>
NEXT_TASK: <recommended task only — NOT AUTHORIZATION>
```

## 4. Questions

When the agent needs clarification, do not silently assume a critical answer.

```text
[DECISION_REQUIRED] <task ID>
Question: <one precise question>
Why it matters: <impact>
Options considered: <short list>
Current contract(s): <references>
Recommended option: <only if safe>
```

Critical security, financial, regulatory, schema, sync, licensing, and provider questions block implementation until explicitly resolved.

## 5. Blocking message

```text
[BLOCKED] <task ID>
Blocker: <precise blocker>
Category: <contract|security|financial|regulatory|schema|sync|licensing|provider|dependency|environment>
Required to unblock: <exact input/decision/evidence>
Cannot safely proceed because: <reason>
```

The agent must not invent a workaround for a hard blocker.

## 6. Evidence rule

Evidence must identify:

- exact commit or PR;
- exact commands/checks run;
- exact results;
- relevant artifacts or logs;
- important manual review findings.

A claim that cannot be reproduced is `UNVERIFIED`.

## 7. State updates

At every task or phase gate, update `AGENT_STATE.md` with:

- current task/status;
- evidence reference;
- blockers;
- exact next task recommendation.

`AGENT_STATE.md` is operational state only. Git history and CI remain authoritative evidence.

## 8. Communication integrity

Never claim:

- a CI job passed when it was not run;
- a test passed when only a compile succeeded;
- a decision was approved when it was merely suggested;
- a regulatory rule is compliant without authoritative evidence.

When evidence is missing, say `UNVERIFIED`.
