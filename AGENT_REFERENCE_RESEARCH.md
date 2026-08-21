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

## 6. Kimi Code Desktop — desktop coding-agent product reference

Source: `https://github.com/Leonxlnx/kimi-code-desktop`

Role:
- resumable project/chat UX;
- Git-centric project workflows;
- terminal + local preview integration;
- skills and subagent UX;
- secure local orchestration boundaries;
- configurable density/theme/accessibility behavior;
- signed in-app update UX;
- deterministic local verification commands;
- release/update evidence patterns.

Use when reviewing the agent-development environment, not as a POS runtime dependency. Do not copy Kimi-specific product logic or credentials handling into Zylo without an explicit task/ADR.

## 7. Stitch Agent Skills — UI/design workflow reference

Source: `https://github.com/Leonxlnx/stitch-skills`

Role:
- design-to-implementation workflows;
- prompt enhancement for UI generation;
- `DESIGN.md` design-system synthesis;
- React component conversion patterns;
- design-token consistency validation;
- shadcn/ui integration guidance.

Use for UI/design tasks when a Stitch-based workflow is explicitly selected. It must remain subordinate to `UI_SPEC.md`, `AGENT_EXTERNAL_SKILLS.md`, accessibility, RTL/i18n and POS keyboard-first requirements.

## 8. Taste Blocks — UI component provenance and licensing reference

Source: `https://github.com/Leonxlnx/taste-blocks`

Role:
- component registry/provenance patterns;
- exact source/revision/path tracking;
- license and third-party-notice tracking;
- modification records;
- registry/build verification discipline.

Use this as a reference for Zylo's third-party UI component provenance policy. When an external UI component is materially adopted, record source, revision, path, license compatibility, notices and modifications. Do not copy components blindly.

## 9. Agentic AI Prompt Research — agent-pattern research

Source: `https://github.com/Leonxlnx/agentic-ai-prompt-research`

Role:
- planner/implementer/reviewer coordination patterns;
- task decomposition and verification loops;
- permission/risk classification;
- context/state management ideas.

Reference-only. The repository describes reconstructed/public-observation material; do not treat it as official vendor prompts or authoritative product architecture.

## Authority and safety rules

1. Repository contracts remain the highest authority.
2. Approved ADRs and task acceptance criteria outrank external references.
3. External projects may inspire patterns but cannot redefine financial truth, authorization, schema semantics, regulatory scope, provider selection, licensing, or release gates.
4. Never copy code from a research repository into Zylo without checking the source license and obtaining the required approval.
5. Never claim that a pattern is production-proven for Zylo merely because it exists in an external project.
6. When external research materially affects an implementation task, record the source, reviewed revision where available, the adopted idea, and rejected alternatives in task evidence or an ADR.
