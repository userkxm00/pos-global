# POS Global — Agent Reference Research

This document records external projects that are useful as architecture/research references for the autonomous implementation-agent system. They are **not product/runtime dependencies** and are not authoritative over repository contracts.

## 1. GoClaw — high-value agent architecture reference

Source: `https://github.com/nextlevelbuilder/goclaw`

Role:
- agent execution pipeline design;
- prompt-mode and context management patterns;
- working/episodic/semantic memory layering;
- agent teams and delegation patterns;
- permission/risk controls;
- multi-tenant isolation patterns;
- observability and tracing ideas;
- event-driven workers, deduplication and retry patterns;
- provider adapters;
- desktop/local-agent packaging and updater patterns.

Use as research when evolving:
- `AGENT_SYSTEM.md`;
- `AGENT_PROMPT.md`;
- planner/implementer/reviewer orchestration;
- agent memory/state design;
- tool permissions and approvals;
- agent observability;
- future agent skill registry architecture.

Do not copy GoClaw code into POS Global merely because a pattern is useful. Do not treat GoClaw's architecture, prompts, provider choices, or product assumptions as authoritative for Zylo.

Important licensing note: the upstream README currently presents GoClaw under a non-commercial Creative Commons license. Treat it as a research reference and do not copy code into this commercial product without a separate legal review.

## 2. GoClaw Docs — operational reference

Source: `https://github.com/nextlevelbuilder/goclaw-docs`

Role:
- agent/system-prompt organization patterns;
- context-file conventions;
- team/delegation and handoff concepts;
- permissions and RBAC concepts;
- skills and MCP integration patterns;
- hooks and quality-gate concepts;
- context pruning;
- usage/quota and cost tracking;
- observability/deployment/security checklists.

Use for research and design review only. The repository's implementation and product assumptions do not override POS Global contracts.

## 3. SkillX — future skill-registry reference

Source: `https://github.com/nextlevelbuilder/skillx`

Role:
- skill discovery and catalog concepts;
- semantic + keyword search;
- ranking/evaluation signals;
- skill ratings and usage reporting;
- CLI/plugin marketplace ideas;
- future skill versioning and provenance patterns.

Use for a future Zylo agent-skill registry/marketplace concept. Do not add SkillX itself as a runtime dependency to the POS application.

## 4. AgentWiki Skills — optional knowledge-vault reference

Source: `https://github.com/nextlevelbuilder/agentwiki-skills`

Role:
- document/knowledge retrieval workflows;
- CLI vs MCP tool selection;
- hybrid search and knowledge-graph patterns;
- CI-safe agent tooling and credential handling.

Use only if the project later adopts an external knowledge-vault workflow. Do not require AgentWiki for normal product implementation.

## 5. AgentBrain CLI — optional admin/knowledge tooling reference

Source: `https://github.com/nextlevelbuilder/agentbrain-cli`

Role:
- tenant-aware CLI patterns;
- auth token/refresh handling;
- organization and permission operations;
- audit/usage/cost tooling;
- structured CLI output and operational diagnostics.

Use as an operational tooling reference only; do not copy service-specific implementations.

## Authority and safety rules

1. Repository contracts remain the highest authority.
2. Approved ADRs and task acceptance criteria outrank external references.
3. External projects may inspire patterns but cannot redefine financial truth, authorization, schema semantics, regulatory scope, provider selection, licensing, or release gates.
4. Never copy code from a research repository into Zylo without checking the source license and obtaining the required approval.
5. Never claim that a pattern is production-proven for Zylo merely because it exists in an external project.
6. When external research materially affects an implementation task, record the source, date/reviewed revision where available, the adopted idea, and any rejected alternatives in task evidence or an ADR.
