# POS Global — Frappe Reference Research Addendum

This addendum curates Frappe organization repositories that are useful as research references for Zylo. They are **not runtime dependencies** and do not override Zylo repository contracts, ADRs, domain rules, security rules, licensing decisions, or release gates.

## 1. `frappe/erpnext` — ERP and business-domain benchmark

Source: `https://github.com/frappe/erpnext`

Study for accounting, sales/order lifecycle, inventory/warehouses/replenishment, purchasing/suppliers, assets, projects, and enterprise workflow boundaries. ERPNext is an open-source ERP with broad accounting, order-management, inventory and operational coverage. Treat it as a domain benchmark, not a Zylo architecture authority. Do not copy/adapt its code into closed-source Zylo without legal review.

## 2. `frappe/frappe` — metadata-driven framework and extensibility reference

Source: `https://github.com/frappe/frappe`

Reviewed license: MIT from the upstream README.

Study metadata/semantic modeling, role-based permissions, configurable forms/views, admin interfaces, REST/API patterns, and application extensibility. Do not adopt the framework itself; Zylo's Tauri/Rust/SQLite architecture remains authoritative.

## 3. `frappe/books` — desktop/offline accounting + POS reference

Source: `https://github.com/frappe/books`

Study desktop accounting UX, offline-first operation, SQLite persistence, POS/accounting boundaries, double-entry accounting, invoicing, payments, financial reports, and cross-platform desktop release. This is a particularly high-value reference because its product shape overlaps several Zylo requirements while using a different desktop stack. Verify source licensing before any code or asset reuse.

## 4. `frappe/print_designer` — document/receipt customization reference

Source: `https://github.com/frappe/print_designer`

Study printable document abstraction, receipt/invoice templates, business branding, layout customization, and separation of rendering from transaction logic. Use it to inform Zylo's document-rendering layer.

## 5. `frappe/frappe-ui` — business-app design-system reference

Source: `https://github.com/frappe/frappe-ui`

Study reusable components, forms, dialogs, navigation, tables, dense business-app layouts and responsive admin interactions. Vue-specific implementation is not adopted; use only as product/UI research alongside Zylo's React contracts.

## 6. `frappe/datatable` — large-data table reference

Source: `https://github.com/frappe/datatable`

Study dense product/inventory/sales tables, sorting/filtering/selection, keyboard interaction, column configuration and performance for large datasets. Use for UX and acceptance criteria for data-heavy Zylo screens.

## 7. `frappe/insights` — analytics/reporting reference

Source: `https://github.com/frappe/insights`

Study analytical dashboards, report exploration, business metrics and separation of analytics UX from transactional screens. Use for future Zylo reporting/analytics tasks.

## 8. `frappe/builder` — visual customization reference

Source: `https://github.com/frappe/builder`

Study visual page composition and configurable business-facing interfaces for future web/portal customization. Do not introduce a visual-builder runtime into the desktop POS core without a dedicated ADR.

## 9. `frappe/agent` — agent/product integration reference

Source: `https://github.com/frappe/agent`

Study practical agent integration and workflow boundaries in a business platform. Reference only; Zylo Agent contracts remain authoritative.

## 10. `frappe/mcp` — MCP integration reference

Source: `https://github.com/frappe/mcp`

Study structured tool exposure and MCP boundaries for future Zylo Copilot/automation. Never expose privileged Zylo actions through MCP without explicit authorization and security review.

## 11. `frappe/event_streaming` — event-driven/synchronization reference

Source: `https://github.com/frappe/event_streaming`

Study event publication/consumption and asynchronous workflows. Use only as research against Zylo's existing outbox, sync queue and branch conflict contracts.

## 12. `frappe/ecommerce_integrations` — commerce connector reference

Source: `https://github.com/frappe/ecommerce_integrations`

Study external commerce integrations, product/order synchronization and marketplace/storefront connector boundaries for future Zylo Web/e-commerce tasks.

## 13. `frappe/webshop` — storefront reference

Source: `https://github.com/frappe/webshop`

Study online product catalogs, categories/search, storefront/customer flows and the desktop POS ↔ web-commerce boundary. Do not turn storefront assumptions into POS-domain rules.

## 14. `frappe/payments` — payment abstraction reference

Source: `https://github.com/frappe/payments`

Study provider abstraction, payment workflow boundaries and external payment integrations. Zylo payment providers remain governed by its own provider matrix, security review and commercial decisions.

## 15. `frappe/erpnext_ui_tests` — UI acceptance-testing reference

Source: `https://github.com/frappe/erpnext_ui_tests`

Study end-to-end business workflows and regression coverage for permissions, sales, inventory and reports. Use it to strengthen Zylo's evidence-driven UI testing strategy.

## 16. `frappe/semgrep-rules` — deterministic security/quality reference

Source: `https://github.com/frappe/semgrep-rules`

Study static security rules and dangerous-pattern detection that complement LLM review. Use alongside Alibaba Open Code Review research, never as the sole security authority.

## 17. `frappe/tally_migrator` — migration/import reference

Source: `https://github.com/frappe/tally_migrator`

Study legacy accounting-data migration, import validation/transformation boundaries, and future Zylo onboarding/import tooling.

## 18. `frappe/hospitality` — hospitality-domain reference

Source: `https://github.com/frappe/hospitality`

The current Frappe organization metadata lists this repository as archived. Use only as historical/domain research for hospitality-specific entities and vertical-module separation.

## Authority and licensing rules

1. Zylo repository contracts and approved ADRs remain authoritative.
2. External Frappe repositories are research references, not implementation dependencies.
3. Do not copy code, assets, prompts, or designs until the exact source license and compatibility with Zylo's commercial model are reviewed.
4. GPL-licensed ERPNext material must not be copied/adapted into the closed-source Zylo application without separate legal approval.
5. When Frappe research materially changes an implementation decision, record the source, reviewed revision where available, adopted pattern, rejected alternatives, and licensing rationale in task evidence or an ADR.
