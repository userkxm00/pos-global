# POS Global — External Agent Skills & Research Registry

This registry documents external open-source skills and research references that may be used by the implementation agent. They are **not application runtime dependencies**.

## 1. Required design skill — UI UX Pro Max

Source: `https://github.com/nextlevelbuilder/ui-ux-pro-max-skill`

Pinned source commit reviewed: `bc826e2267a36d98a2dcf5231e16c30ff546770f`
License: MIT

Role in POS Global:
- design-system generation;
- industry-aware UI/UX recommendations;
- accessibility and resilient text/layout guidance;
- dashboard/report visualization guidance;
- responsive and cross-platform UI guidance;
- design pre-flight checks.

Use for:
- Phase 1–7 desktop UI;
- Phase 9 marketing site/customer portal;
- design-system refinement and visual consistency reviews.

Do not use it to override:
- `UI_SPEC.md`;
- product requirements;
- accessibility/security constraints in repository contracts;
- financial/domain behavior;
- approved branding decisions.

Treat its recommendations as design guidance, not as an architectural dependency.

## 2. Required visual-quality skill — Taste Skill

Source: `https://github.com/Leonxlnx/taste-skill`

Pinned source commit reviewed: `843c8dd4d18ccff0d5a9cd4b0b71d7dbf7278293`
License: MIT

Role in POS Global:
- visual hierarchy and composition review;
- anti-generic / anti-slop UI review;
- spacing, typography, density and motion quality;
- redesign/refinement review of existing screens;
- brand-oriented visual consistency when explicitly requested.

Use for:
- design review after a feature is functionally implemented;
- UI polish passes;
- marketing/portal work in Phase 9;
- brand visual consistency work after brand approval.

Do not allow Taste rules to override:
- POS keyboard-first requirements;
- accessibility;
- RTL/i18n;
- semantic/status colors required for validation and operational state;
- product/task acceptance criteria.

## 3. Reference-only agent research — Agentic AI Prompt Research

Source: `https://github.com/Leonxlnx/agentic-ai-prompt-research`

Use: reference/research only.

Role:
- study patterns for planner/implementer/reviewer coordination;
- risk/permission classification;
- task decomposition and verification loops;
- context/state management ideas.

Do **not** treat this repository as authoritative implementation guidance. It is research/reconstructed material and is not a runtime dependency of POS Global.

## 4. Reference-only agent architecture — GoClaw

Source: `https://github.com/nextlevelbuilder/goclaw`

Use: high-value architecture research for the agent system.

Relevant topics:
- multi-stage agent execution pipelines;
- prompt/context modes;
- working/episodic/semantic memory patterns;
- agent teams, delegation and handoff;
- permission/risk controls;
- event-driven workers, deduplication and retry;
- provider adapter boundaries;
- observability/tracing;
- local/desktop agent packaging and update patterns.

Important: GoClaw is a separate product and its README currently presents a non-commercial Creative Commons license. Use it as research only; do not copy code into Zylo without separate legal review.

## 5. Reference-only operational research — GoClaw Docs

Source: `https://github.com/nextlevelbuilder/goclaw-docs`

Use for research into:
- system-prompt/context-file organization;
- team/delegation/handoff concepts;
- permissions and RBAC;
- skills/MCP integration;
- hooks and quality gates;
- context pruning;
- usage/quota and cost tracking;
- deployment/security/observability practices.

The docs are reference material only and do not override Zylo/POS Global contracts.

## 6. Future reference — SkillX

Source: `https://github.com/nextlevelbuilder/skillx`

Use for future design of a Zylo agent-skill registry/catalog:
- skill discovery;
- semantic + keyword search;
- ranking/evaluation signals;
- skill ratings and usage reporting;
- CLI/plugin marketplace concepts;
- versioning and provenance.

Do not add SkillX itself as a runtime dependency to the POS application.

## 7. Optional knowledge-vault reference — AgentWiki Skills

Source: `https://github.com/nextlevelbuilder/agentwiki-skills`

Use only if the project later adopts an external knowledge-vault workflow. Relevant patterns include CLI vs MCP selection, hybrid retrieval, knowledge graphs, CI-safe authentication, and credential-handling rules.

## 8. Optional admin/operational reference — AgentBrain CLI

Source: `https://github.com/nextlevelbuilder/agentbrain-cli`

Use for operational-tooling ideas such as tenant-aware CLI workflows, auth token/refresh handling, organization/permission operations, audit/usage/cost tooling, and structured command output.

It is not a POS application dependency.

## Skill classes

### Required
- UI UX Pro Max
- Taste Skill

### Optional / phase-specific
- Brand-oriented Taste workflows when Phase 9 brand work is approved.
- Existing-project redesign workflows when a UI review task explicitly calls for them.

### Reference-only
- Agentic AI Prompt Research
- GoClaw
- GoClaw Docs
- SkillX
- AgentWiki Skills
- AgentBrain CLI

## Installation and provenance policy

External skills and research projects are agent/developer tooling or references, not desktop application dependencies.

Never add an external project's npm package, Python package, runtime binary, or entire repository to the product solely because the agent uses the project's ideas.

Prefer one of:
1. the skill being installed in the agent environment;
2. a source reference/pinned commit in this registry;
3. a small repository-owned adaptation written into `AGENT_SKILLS.md` when the project needs deterministic behavior.

The agent must not silently fetch moving `main` branches and treat them as reproducible evidence. When an external skill or research project materially affects a deliverable, record the source URL, reviewed revision where available, adopted idea, and any rejected alternatives in task evidence or an ADR.

## Authority hierarchy

Repository contracts always win:

`Zylo/POS Global specs → approved ADRs → task acceptance criteria → external skills/research`

External references may improve implementation quality, but they never change business rules, financial truth, authorization, schema semantics, regulatory scope, provider selection, licensing, or release/security gates.
