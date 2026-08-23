# AGENT READING ROADMAP — Zylo

This roadmap reduces startup overhead without weakening repository contracts.

## Tier 1 — Startup core (read before every implementation task)

1. `V2_RULES.md` — mandatory agent boundaries from the previous POS audit
2. `AGENT_SYSTEM.md`
3. `PROJECT_STATUS.md`
4. `ARCHITECTURE.md`
5. `PRODUCT_STRATEGY.md` when scope, MVP, market, pricing, onboarding or product priority could be affected
6. `FOUNDATION_READINESS_STATES.md` when readiness/gate state matters
7. `EXECUTION_PLAN.md`
8. `TASK_SPEC.md`
9. `DEFINITION_OF_READY.md`
10. Inspect the actual repository state, branch, recent commits, and affected implementation.

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
- `REFERENCE_CATALOG.md` for website, UI, mobile, design-system, or other tasks that may benefit from an approved external reference

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

## Execution-over-planning rule

Once the assigned task is Ready and its required decisions are resolved, implementation takes priority over creating additional planning material. Do not create new planning, strategy, architecture, or process documents unless the assigned task, an approved ADR, or a failing verification gate explicitly requires one.

The product roadmap remains global and multi-industry. The approved first validation scope is only an MVP sequencing constraint and must never be treated as a permanent product restriction.
