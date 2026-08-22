# POS Global — Additional Frappe Reference Research

This addendum contains only newly curated Frappe references discovered during the broader organization review. They are **research references only**, not runtime dependencies, and they do not override Zylo contracts, ADRs, domain rules, security rules, licensing decisions, or release gates.

## 1. `frappe/frappe-forms` — dynamic forms and configurable business UI

Source: `https://github.com/frappe/frappe-forms`

Study schema-driven/dynamic forms, reusable field definitions, validation, configurable data-entry workflows, and form metadata. Relevant to Zylo product custom attributes, industry-specific fields, onboarding, and configurable business workflows. Do not introduce it as a runtime dependency; compare patterns against Zylo's React contracts.

## 2. `frappe/test-orchestrator` — test orchestration reference

Source: `https://github.com/frappe/test-orchestrator`

Study orchestration of automated verification across test suites and environments. Relevant to Zylo's evidence-driven task execution, regression layers, phase gates, and future CI/E2E orchestration. Use as process research only.

## 3. `frappe/chart_of_accounts_builder` — accounting onboarding reference

Source: `https://github.com/frappe/chart_of_accounts_builder`

Study guided creation/configuration of a chart of accounts and accounting setup flows. Relevant to Zylo accounting onboarding and business/jurisdiction presets. Do not copy financial rules; Zylo accounting contracts remain authoritative.

## 4. `frappe/taxjar_integration` — external tax-provider adapter reference

Source: `https://github.com/frappe/taxjar_integration`

Study how a tax provider is kept behind an integration boundary. Relevant to Zylo's provider-neutral tax-engine contract and future jurisdiction/provider adapters. The repository is a reference only; tax correctness must come from Zylo's own approved jurisdiction contracts.

## 5. `frappe/erpnext_gst_compliance` — jurisdiction-tax package reference

Source: `https://github.com/frappe/erpnext_gst_compliance`

Study the separation of jurisdiction-specific tax/compliance behavior from shared commerce logic. Relevant as an example for country packs. This is India-specific research and must not be treated as an authority for Algeria, France/EU, or any other jurisdiction.

## 6. `frappe/erpnext_local` — localization architecture case study

Source: `https://github.com/frappe/erpnext_local`

Study country/localization packaging and separation of localized requirements from broader ERP behavior. Relevant to Zylo's jurisdiction-adapter architecture and future country packs.

## 7. Localization case-study family: `erpnext_france`, `erpnext_usa`, `erpnext_uae`, `erpnext_italy`, `erpnext_south_africa`, `erpnext_ksa`

Sources:
- `https://github.com/frappe/erpnext_france`
- `https://github.com/frappe/erpnext_usa`
- `https://github.com/frappe/erpnext_uae`
- `https://github.com/frappe/erpnext_italy`
- `https://github.com/frappe/erpnext_south_africa`
- `https://github.com/frappe/erpnext_ksa`

Treat these as **one localization case-study family**, not as six independent product dependencies. Study how jurisdiction-specific tax, invoicing, reporting, and compliance concerns can be isolated from shared business logic. Some repositories may be archived; verify current status before using any example as a current pattern. Never infer Algerian or EU legal requirements from these examples without authoritative sources.

## 8. `frappe/cafe` — café/food-service vertical reference

Source: `https://github.com/frappe/cafe`

Study café-specific product/menu, ordering, service, and vertical workflow concepts. Relevant to Zylo's Café/Restaurant/Fast Food capability composition. Use only as domain research; do not fork Zylo's shared sales/inventory/financial core for hospitality workflows.

## 9. `frappe/erpnext_shopify` — storefront/product/order synchronization reference

Source: `https://github.com/frappe/erpnext_shopify`

Study ecommerce connector boundaries, product synchronization, order synchronization, and mapping between external storefront entities and an internal commerce model. Relevant to future Zylo Web/ecommerce integrations.

## 10. `frappe/erpnext_shopify_broker` — integration/broker boundary reference

Source: `https://github.com/frappe/erpnext_shopify_broker`

Study broker-style separation between external commerce systems and core ERP/business workflows. Relevant to Zylo connector isolation and retry/idempotency design.

## 11. `frappe/shopping_cart` — web commerce flow reference

Source: `https://github.com/frappe/shopping_cart`

Study storefront cart and checkout-oriented flows as a product/UX reference for future Zylo Web, while keeping transactional truth in Zylo's own sales domain.

## 12. `frappe/offsite_backups` — backup/recovery operations reference

Source: `https://github.com/frappe/offsite_backups`

Study backup scheduling, offsite retention, recovery workflows, and operational backup boundaries. Relevant to Zylo's production-hardening and restore-drill requirements.

## 13. `frappe/raven` — error monitoring/observability reference

Source: `https://github.com/frappe/raven`

Study error reporting, observability workflows, diagnostics, and operator-facing incident visibility. Relevant to Zylo crash/error monitoring without coupling the desktop app to the Frappe stack.

## 14. `frappe/release_manager` — release process reference

Source: `https://github.com/frappe/release_manager`

Study release orchestration and operational release workflows. Relevant to Zylo staged releases, updater delivery, release evidence, and rollback procedures.

## 15. `frappe/release_tests` — release regression reference

Source: `https://github.com/frappe/release_tests`

Study regression-oriented release acceptance and compatibility verification. Relevant to Zylo upgrade testing and release gates.

## 16. `frappe/frappe-react-sdk` — React API integration reference

Source: `https://github.com/frappe/frappe-react-sdk`

Study typed/API-client organization and React-facing data-access patterns as a reference only. Do not introduce Frappe APIs or SDKs into Zylo; Tauri IPC and approved cloud adapters remain authoritative.

## 17. `frappe/shop` — commerce/store platform reference

Source: `https://github.com/frappe/shop`

Study broader commerce/store workflows and product/store boundaries that may inform future Zylo Web and merchant-facing commerce surfaces.

## 18. `frappe/marketplace` — extension marketplace reference

Source: `https://github.com/frappe/marketplace`

Study marketplace concepts for a possible future Zylo ecosystem of plugins, integrations, industry packs, or capabilities. This does not authorize a marketplace feature in the current roadmap.

## 19. `frappe/press-api-spec` — contract/spec-first API reference

Source: `https://github.com/frappe/press-api-spec`

Study explicit API contracts/specification-first development and how API surfaces are documented separately from implementation. Relevant to Zylo cloud/service boundaries.

## 20. `frappe/skills` — skill packaging/discovery reference

Source: `https://github.com/frappe/skills`

Study packaging, organization, and discovery of agent skills. Relevant to future Zylo agent-skill lifecycle design. Do not add external skills to the runtime desktop application merely because the repository is useful as research.

## 21. `frappe/press-agent-manager` — agent orchestration reference

Source: `https://github.com/frappe/press-agent-manager`

Study orchestration/management patterns for agents and operational workers. Relevant to future Zylo agent operations and delegated execution, but not required for the current bounded-task model.

## 22. `frappe/press-compute-agent` and `frappe/press-compute-orchestrator` — worker orchestration references

Sources:
- `https://github.com/frappe/press-compute-agent`
- `https://github.com/frappe/press-compute-orchestrator`

Study worker lifecycle, delegation, orchestration, and recovery boundaries for future agent/automation infrastructure. Reference only; do not import these operational assumptions into the desktop POS core.

## 23. `frappe/llms_txt` — documentation/agent discoverability reference

Source: `https://github.com/frappe/llms_txt`

Study structured documentation exposure intended for machine/LLM consumption. Relevant to future Zylo developer documentation and agent-readable project knowledge.

## 24. `frappe/erpnext_price_estimation` — pricing/estimation reference

Source: `https://github.com/frappe/erpnext_price_estimation`

Study estimation and pricing-support concepts that may inform Zylo quotations, pricing analysis, and future commercial workflows. Do not treat its formulas as Zylo's costing or pricing policy.

## 25. `frappe/indiaos` — business-platform ecosystem reference

Source: `https://github.com/frappe/indiaos`

Study platform-level integration and ecosystem patterns for localized business software. Lower-priority research; not a core POS dependency.

## 26. `frappe/pulse` — operational monitoring reference

Source: `https://github.com/frappe/pulse`

Study operator-facing monitoring/notification workflows that may inform future Zylo operational dashboards and incident surfaces.

## 27. `frappe/erpnext_local` + jurisdiction repositories — one rule

Use localization repositories as **case studies**, never as legal authority. Country-specific implementation choices must come from Zylo's approved jurisdiction research package and authoritative sources.

## Research-use and licensing guardrails

1. These repositories are research references only; none are runtime dependencies.
2. Repository availability or popularity does not authorize copying code, designs, assets, prompts, or data.
3. Verify the exact license and current repository status before any reuse of code or assets.
4. Jurisdiction repositories are examples of architecture/packaging, not legal advice or tax authority.
5. If a reference materially changes a Zylo implementation decision, record the source, reviewed revision where available, adopted pattern, rejected alternatives, and license/compliance reasoning in task evidence or an ADR.
