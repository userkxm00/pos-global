# AGENT READING ROADMAP — Zylo

This roadmap reduces startup overhead without weakening repository contracts.

## Tier 1 — Startup core (read before every implementation task)

1. `AGENT_SYSTEM.md`
2. `PROJECT_STATUS.md`
3. `ARCHITECTURE.md`
4. `FOUNDATION_READINESS_STATES.md` when readiness/gate state matters
5. `EXECUTION_PLAN.md`
6. `TASK_SPEC.md`
7. `DEFINITION_OF_READY.md`
8. Inspect the actual repository state, branch, recent commits, and affected implementation.

## Tier 2 — Task context (read only what the assigned task needs)

- `DOMAIN_CONTRACTS.md` for the affected domain
- `DATABASE_RULES.md` when storage/schema is affected
- `SECURITY_MODEL.md` for auth, authorization, secrets, licensing, sync, privileged APIs
- `SYNC_SPEC.md` for offline/sync/conflict work
- `UI_SPEC.md` and `UI_CLOUD_EXECUTION_PLAN.md` for UI/cloud tasks
- `INDUSTRY_EXECUTION_PLAN.md` and `CAPABILITY_MATRIX.md` for industry/capability work
- `RELEASE_SPEC.md` for updater, signing, packaging, release, rollback, or production tasks
- `COMMERCIAL_PROVIDER_MATRIX.md` for provider decisions
- `PHASE_0_5_DOMAIN_FINALIZATION.md` / `PHASE_0_6_COMMERCIAL_REGULATORY_FINALIZATION.md` when the task touches the decisions defined there
- `AGENT_EXTERNAL_SKILLS.md` only for UI/design or agent-tooling work

## Tier 3 — Targeted research

Read external-reference documents only when the task is materially related to them:

- `AGENT_REFERENCE_RESEARCH.md`
- `AGENT_REFERENCE_RESEARCH_FRAPPE.md`
- `AGENT_REFERENCE_RESEARCH_FRAPPE_ADDENDUM_2.md`
- regulatory research packages

External research is never product authority.

## Tier 4 — Gate, communication, and evidence files

Read when validating, completing, blocking, or handing off a task/gate:

- `FOUNDATION_EVIDENCE.md`
- `ACCEPTANCE_MATRIX.md`
- `AGENT_COMMUNICATION_PROTOCOL.md`
- `BLOCKING_CRITERIA.md`
- `EVIDENCE_TEMPLATE.md`
- `MILESTONE_CHECKPOINTS.md`
- task evidence/artifacts
- relevant ADRs

These files define reporting, blocking, evidence, and milestone mechanics. They do not authorize a new implementation task.

## Anti-paralysis rule

Do not read the entire repository documentation tree by default. Expand from Tier 1 only as required by the assigned task.

The agent must never interpret a reading list as permission to start another task.
