# POS Global — Master Execution Plan

## Mission
Build a commercial, global, offline-first POS and store-management platform that supports many business types through composable capabilities and modules. The system must be safer, more testable and more maintainable than Mellah-POS-V2.

## Absolute rules
- Never mark work Done without executable evidence.
- Never use floating point as authoritative financial truth.
- Never require network access to complete a local POS transaction.
- Never trust UI-only authorization.
- Never mutate stock without a ledger/movement record.
- Never make sync retry create duplicate business transactions.
- Never ship secrets, service-role keys, license private keys or updater private keys.
- Never modify an applied migration; add a new migration.
- Never add dependencies without a documented reason and compatibility/security review.
- Never allow an agent to invent a major financial, regulatory, security, schema, synchronization or licensing rule silently.

## Phase 0 — Foundation Gate
Deliver: architecture, domain model, schema, migration runner, exact-money policy, units policy, tenant model, capability/module model, security boundaries, Supabase contract, updater contract, CI contract and testing strategy.
Exit: empty database migrations apply cleanly and foundation documents agree with each other.

## Phase 0.5 — Domain Finalization Gate
Deliver: frozen money/rounding rules, quantity/unit rules, costing policy and interface, tax-engine contract, pricing/promotion semantics, payment abstraction, refund/exchange semantics, cash/debt/loyalty ledgers, sync conflict matrix, hardware abstraction, industry capability taxonomy and license/entitlement boundary. See `PHASE_0_5_DOMAIN_FINALIZATION.md`.
Exit: every shared business primitive has an explicit contract; unresolved decisions are explicitly marked `DECISION REQUIRED`; no implementation agent is allowed to guess critical business behavior.

## Phase 0.6 — Commercial & Regulatory Finalization Gate
Deliver: license/business model, entitlement policy, billing-provider abstraction and eligibility review, POS payment-provider strategy, launch-market scope, jurisdiction adapter architecture, Algeria regulatory research package, France/EU expansion scope, website commercial lifecycle and regulatory evidence policy. See `PHASE_0_6_COMMERCIAL_REGULATORY_FINALIZATION.md`.
Exit: launch markets are explicitly approved; provider choices have an evidence/eligibility checklist; no global compliance claim depends on an undocumented assumption.

## Phase 0.7 — Agent Readiness Gate
Deliver: complete agent operating system, master/planner/implementer/reviewer prompts, granular backlog, task specification, Definition of Ready/Done, ADR protocol, evidence protocol, golden E2E flows, acceptance matrix and persistent agent state.
Exit: a fresh implementation agent can enter the repository, identify exactly one unblocked task, execute it without inventing architecture, produce evidence, update state, and stop at a gate.

## Phase 1 — Identity and tenancy
Organization, branch, register, device, user, roles, permissions, local sessions, Supabase Auth integration and RLS tenant isolation.
Exit: authenticated users can access only authorized organization/branch data; privileged actions are rejected in Rust when unauthorized.

## Phase 2 — Product and inventory core
Products, categories, SKU/barcode, units/conversions, variants/matrix, weighted items, batch/expiry/FEFO, serial/IMEI, warranty, stock balances and immutable stock movements.
Exit: every stock mutation is atomic, traceable and recoverable.

## Phase 3 — Sales and cash
Cart, sale transaction, sale lines, taxes, discounts, price lists, payments, split payments, change, customer account, debt, cash shifts, receipt data, void, refund and exchange.
Exit: sale/payment/stock/cash invariants pass including crash and retry tests.

## Phase 4 — Purchasing and profitability
Suppliers, purchase orders, receiving, supplier invoices/payments, cost layers/average cost policy, COGS, margin and valuation.
Exit: historical sale cost is deterministic and profit reports reconcile to ledger data.

## Phase 5 — Customers and loyalty
Customer profiles, groups, credit limits, debt ledger, payments, loyalty earn/redeem/reversal/expiry and customer pricing.
Exit: all balance changes are ledger-backed and auditable.

## Phase 6 — Offline sync
Transactional outbox, event versions, idempotency keys, retry policy, conflict rules, device identity, sync checkpoints, recovery and observability.
Exit: offline sales sync exactly once; duplicate/retry/conflict tests pass.

## Phase 7 — Industry modules
Use capabilities rather than separate products. Initial module families: Retail, Fashion, Grocery, Electronics, Automotive, Pharmacy, Furniture, Hardware, Restaurant, Café, Fast Food, Bakery, Repair, Rental, Salon, Wholesale, Hospitality, Events, Services and other configurable commerce workflows.
Exit: each module has domain tests and does not corrupt the shared inventory/sales/financial invariants.

## Phase 8 — Licensing
Signed license format, activation, device entitlement, plans, expiry/grace, revocation, offline verification, device reset and server-side license administration. License signing key is isolated from updater signing key.
Exit: tamper/replay/offline/clock and entitlement tests pass.

## Phase 9 — Website and billing
Marketing site, pricing, account portal, checkout, downloads, license management, documentation and support entry points. Billing provider must be selected after the Phase 0.6 commercial/legal review.
Exit: a customer can acquire, activate and manage a license without manual database intervention.

## Phase 10 — Hardware
Barcode scanner, receipt printer, cash drawer, label printer, supported scales and optional customer display through stable Rust abstractions. Device failures must not corrupt transactions.

## Phase 11 — Reporting and operations
Sales, inventory, valuation, COGS, profit, cash, tax, debt, customer, product and branch reports. Add export and audit views.

## Phase 12 — Release engineering
Windows/macOS/Linux builds, code signing, GitHub Actions, staged releases, signed Tauri updater artifacts, update availability UI, safe-install point, rollback/recovery strategy and release notes.

## Phase 13 — Production hardening
Performance, crash/error monitoring, backups, restore drills, migration rollback strategy, security review, dependency audit, accessibility, RTL/i18n, E2E and pilot stores.

## Phase 14 — Launch
Private beta → monitored pilot → paid beta → production. Keep rollback, incident response and support procedures ready.

## Definition of Ready
A task may start only when its objective, dependencies, acceptance criteria, security impact, database impact, affected contracts and required evidence are known. Critical financial/regulatory/security/schema/provider decisions must be resolved or explicitly blocked before implementation.

## Definition of Done
A phase is complete only when code, migrations, tests, security checks, failure paths, documentation and acceptance evidence exist and CI passes. A passing build alone is never sufficient.
