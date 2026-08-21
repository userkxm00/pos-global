# POS Global — External Agent Skills Registry

This registry documents external open-source skills/research that may be used by the implementation agent. They are **not application runtime dependencies**.

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

The upstream project documents support for multiple AI coding environments and multiple frameworks, including React and Tailwind-related workflows. Treat its recommendations as design guidance, not as an architectural dependency. cite-source-needed-in-upstream-review

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

Important: upstream community discussions show framework-specific guidance and some hard visual defaults can require contextual adaptation. The agent must apply the concept, not blindly copy framework-specific snippets into the Tauri/React application.

## 3. Reference-only agent research — Agentic AI Prompt Research

Source: `https://github.com/Leonxlnx/agentic-ai-prompt-research`

Use: reference/research only.

Role:
- study patterns for planner/implementer/reviewer coordination;
- risk/permission classification;
- task decomposition and verification loops;
- context/state management ideas.

Do **not** treat this repository as authoritative implementation guidance. It is explicitly research/reconstructed material and is not a runtime dependency of POS Global.

## 4. Skill classes

### Required
- UI UX Pro Max
- Taste Skill

### Optional / phase-specific
- Brand-oriented Taste workflows when Phase 9 brand work is approved.
- Existing-project redesign workflows when a UI review task explicitly calls for them.

### Reference-only
- Agentic AI Prompt Research.

## 5. Installation policy

External skills are developer/agent tooling, not desktop application dependencies.

Never add an external skill's npm package, Python package, runtime binary, or entire repository to the product solely because the skill is useful to the agent.

Prefer one of:
1. the skill being installed in the agent environment;
2. a source reference/pinned commit in this registry;
3. a small repository-owned adaptation written into `AGENT_SKILLS.md` when the project needs deterministic behavior.

The agent must not silently fetch a moving `main` branch and treat it as reproducible evidence. When a skill meaningfully affects a deliverable, record the source URL and reviewed commit/tag in the task evidence.

## 6. Authority hierarchy

Repository contracts always win:

`Zylo/POS Global specs → approved ADRs → task acceptance criteria → external skills`

External skills may improve implementation quality, but they never change business rules, financial truth, authorization, schema semantics, regulatory scope, or release/security gates.
