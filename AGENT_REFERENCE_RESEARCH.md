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

## 10. Flutter Billing App — POS and hardware reference

Source: `https://github.com/Dinesh-Sowndar/flutter_billing_app`

License status: the repository metadata currently does **not** declare a license. Treat the repository as **reference-only** and do not copy code, assets, or dependencies until a source-licensing review confirms permission.

Role:
- offline-first POS checkout UX;
- barcode/QR scanning workflows;
- thermal receipt printing patterns;
- shop settings and receipt customization;
- product and billing feature boundaries;
- feature-first Clean Architecture organization;
- hardware integration seams for scanners/printers.

Use for:
- POS checkout UX review;
- barcode/hardware task design;
- receipt/printer adapter requirements;
- offline user-flow comparisons.

Do not adopt Flutter/Hive/Bloc/GetIt from this project as Zylo runtime technology. Zylo's Tauri/Rust/SQLite contracts remain authoritative.

## 11. Alibaba Page Agent — web GUI agent reference

Source: `https://github.com/alibaba/page-agent`

Reviewed license: MIT.

Role:
- in-page GUI-agent concepts;
- text/DOM-based UI grounding without screenshot dependence;
- natural-language action execution;
- SaaS copilot patterns;
- accessibility and smart-form-filling ideas;
- optional MCP/browser control patterns.

Use when designing future Zylo web-portal/admin copilot capabilities or natural-language UI assistance. Do not use it as the POS-domain authority and do not make it a dependency of the desktop POS core without an explicit architecture decision.

## 12. EverShop — commerce module/extension architecture reference

Source: `https://github.com/evershopcommerce/evershop`

Reviewed license: GPL-3.0.

Role:
- modular commerce architecture;
- extension/module boundaries;
- theme/customization separation;
- React/TypeScript commerce workflows;
- developer-facing extensibility patterns.

Use to research how a broad commerce platform can separate core capabilities from industry-specific extensions. Because EverShop is GPL-3.0, **do not copy or adapt its code into the closed-source commercial Zylo product without a separate legal review**. Architecture ideas alone may be studied without treating the repository as implementation authority.

## 13. Alibaba Open Code Review — code-review and quality-gate reference

Source: `https://github.com/alibaba/open-code-review`

Reviewed license: Apache-2.0.

Role:
- hybrid deterministic + LLM-assisted code review;
- precise line-level review findings;
- repository-level context for reviews;
- security-oriented rule patterns;
- multi-language static checks coordinated with agent review.

Use when evolving Zylo's agent verification/review loop, PR review policy, evidence capture, and deterministic-vs-agentic quality gates. Do not treat its ruleset or product architecture as authoritative for Zylo.

## 14. Alibaba Designable — configurable UI/form-builder reference

Source: `https://github.com/alibaba/designable`

Reviewed license: MIT.

Role:
- configurable visual/form editing patterns;
- schema-driven UI composition;
- designable React component concepts;
- extensible form/editor architecture.

Use for future Zylo capabilities such as customizable business settings, schema-driven forms, configurable dashboards, and admin-side UI composition. Do not introduce the library as a runtime dependency unless a separate architecture decision approves it.

## 15. Alibaba ChatUI — assistant/chat UI reference

Source: `https://github.com/alibaba/ChatUI`

License: not recorded in this registry because the repository metadata was not independently verified during this review.

Role:
- conversational assistant interface patterns;
- message composition and rendering;
- agent status/progress UX;
- future Zylo admin/customer copilot UI research.

Treat as reference-only until licensing is explicitly verified. Do not copy code or assets before that review.

## 16. Alibaba Formily — dynamic forms/configuration reference

Source: `https://github.com/alibaba/formily`

Role:
- schema-driven dynamic forms;
- field composition and validation patterns;
- reusable form logic across business contexts;
- patterns applicable to industry-specific product attributes and business configuration.

Use as research for Zylo's dynamic onboarding, custom attributes, configurable settings, and industry-specific forms. Do not make Formily a runtime dependency without a dedicated architecture decision and license review.

## 17. Dinesh Responsive Dashboard Admin Panel — dashboard UI reference

Source: `https://github.com/Dinesh-Sowndar/flutter_responsive_dashboard_admin_panel_ui`

License status: repository metadata currently does **not** declare a license. Treat as reference-only until licensing is verified.

Role:
- responsive dashboard composition;
- admin-panel navigation and density patterns;
- dashboard cards and analytical layouts;
- desktop/web back-office UI inspiration.

Do not adopt Flutter-specific runtime architecture. Use only as a visual/product-reference source under Zylo's React/Tauri UI contracts.

## 18. Dinesh Food App UI — restaurant UI reference

Source: `https://github.com/Dinesh-Sowndar/flutter_food_app_ui`

License status: repository licensing was not independently verified during this review. Treat as reference-only until verified.

Role:
- restaurant/food ordering UI inspiration;
- menu/category presentation;
- product customization and modifier-like interaction patterns;
- food-service visual hierarchy useful for Restaurant/Café use cases.

Use only for design/domain research. Do not copy code or assets without licensing review.

## Authority and safety rules

1. Repository contracts remain the highest authority.
2. Approved ADRs and task acceptance criteria outrank external references.
3. External projects may inspire patterns but cannot redefine financial truth, authorization, schema semantics, regulatory scope, provider selection, licensing, or release gates.
4. Never copy code from a research repository into Zylo without checking the source license and obtaining the required approval.
5. Never claim that a pattern is production-proven for Zylo merely because it exists in an external project.
6. When external research materially affects an implementation task, record the source, reviewed revision where available, the adopted idea, and rejected alternatives in task evidence or an ADR.
7. Repositories with unknown/unconfirmed licensing are reference-only until legal provenance is established.
