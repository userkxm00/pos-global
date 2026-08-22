# POS Global — Frappe Reference Research Addendum

This document curates Frappe organization repositories that are useful as research references for Zylo. They are **not runtime dependencies** and do not override Zylo repository contracts, ADRs, domain rules, security rules, licensing decisions, or release gates.

## Existing curated references

The original curated Frappe set remains authoritative for the references already selected for Zylo: `erpnext`, `frappe`, `books`, `print_designer`, `frappe-ui`, `datatable`, `insights`, `builder`, `agent`, `mcp`, `event_streaming`, `ecommerce_integrations`, `webshop`, `payments`, `erpnext_ui_tests`, `semgrep-rules`, `tally_migrator`, and the archived `hospitality` domain reference.

## Deep-review extension

The broader Frappe organization review produced a second curated set of additional references. Read `AGENT_REFERENCE_RESEARCH_FRAPPE_ADDENDUM_2.md` when a task touches any of these areas:

- dynamic/schema-driven forms;
- test orchestration and release regression;
- accounting onboarding/chart of accounts;
- tax-provider adapters and jurisdiction packages;
- country/localization architecture;
- café/food-service workflows;
- Shopify/storefront/commerce synchronization;
- backup/recovery and observability;
- release orchestration;
- React API-client patterns;
- marketplace/extension ecosystems;
- API contract/spec-first design;
- skill packaging/discovery;
- agent/worker orchestration;
- LLM-readable documentation;
- pricing/estimation support;
- business-platform ecosystem patterns.

The second set is deliberately kept separate so the main research registry remains usable and does not become an unbounded list of repositories.

## Authority and licensing rules

1. Zylo repository contracts and approved ADRs remain authoritative.
2. External Frappe repositories are research references, not implementation dependencies.
3. Do not copy code, assets, prompts, or designs until the exact source license and compatibility with Zylo's commercial model are reviewed.
4. GPL-licensed ERPNext material must not be copied/adapted into the closed-source Zylo application without separate legal approval.
5. Jurisdiction repositories are case studies, not legal authority. Country-specific implementation decisions require Zylo's approved jurisdiction research package and authoritative sources.
6. When Frappe research materially changes an implementation decision, record the source, reviewed revision where available, adopted pattern, rejected alternatives, and licensing rationale in task evidence or an ADR.
