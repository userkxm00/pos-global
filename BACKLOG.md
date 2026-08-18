# MASTER BACKLOG

Tasks are intentionally granular. An agent must not skip a gate to reach a later phase.

## Phase 0 — Foundation

- F0.01 reconcile architecture/schema/execution plan
- F0.02 finalize database rules and migration policy
- F0.03 finalize domain contracts
- F0.04 finalize security model
- F0.05 finalize offline/sync contract
- F0.06 finalize agent operating system
- F0.07 verify CI: frontend + Rust + migrations + tests
- F0.08 dependency/security audit
- F0.09 Foundation Gate review
- F0.10 merge Foundation/v2 to main

## Phase 0.5 — Domain Finalization

- F0.5.01 freeze money, currency and rounding rules
- F0.5.02 freeze quantity, unit and conversion rules
- F0.5.03 freeze inventory costing strategy and costing interface
- F0.5.04 freeze tax engine contract and jurisdiction model
- F0.5.05 freeze pricing, discounts and promotion semantics
- F0.5.06 freeze payment abstraction and reconciliation semantics
- F0.5.07 freeze refund and exchange semantics
- F0.5.08 freeze cash ledger and shift semantics
- F0.5.09 freeze customer debt ledger semantics
- F0.5.10 freeze loyalty ledger semantics
- F0.5.11 freeze sync conflict matrix by aggregate
- F0.5.12 freeze hardware abstraction contracts
- F0.5.13 freeze industry capability taxonomy
- F0.5.14 freeze license/entitlement boundary
- F0.5.15 write ADRs for unresolved critical decisions
- F0.5.16 Domain Finalization Gate review

## Phase 0.6 — Commercial & Regulatory Finalization

- F0.6.01 freeze license/business model
- F0.6.02 define entitlement and plan model
- F0.6.03 evaluate SaaS billing providers and seller eligibility
- F0.6.04 freeze billing-provider abstraction
- F0.6.05 evaluate POS payment-provider strategy by launch market
- F0.6.06 define Algeria regulatory adapter research package
- F0.6.07 define France/EU regulatory adapter scope
- F0.6.08 define jurisdiction metadata/source/effective-date model
- F0.6.09 define regulated-industry launch policy
- F0.6.10 freeze website commercial lifecycle
- F0.6.11 document regulatory evidence and review policy
- F0.6.12 Commercial & Regulatory Gate review

## Phase 0.7 — Agent Readiness

- F0.7.01 verify AGENT_SYSTEM
- F0.7.02 verify master/planner/implementer/reviewer prompts
- F0.7.03 verify granular backlog and task IDs
- F0.7.04 verify Definition of Ready/Done
- F0.7.05 verify ADR protocol
- F0.7.06 verify evidence protocol
- F0.7.07 verify golden E2E flows
- F0.7.08 verify acceptance matrix
- F0.7.09 verify persistent agent state
- F0.7.10 Agent Readiness Gate review

## Phase 1 — Identity & tenancy

- F1.01 organization model
- F1.02 branch model
- F1.03 register/device model
- F1.04 Supabase Auth adapter
- F1.05 local user/session model
- F1.06 roles and permissions
- F1.07 Rust authorization middleware
- F1.08 Supabase RLS policies
- F1.09 auth integration tests
- F1.10 tenant-isolation tests

## Phase 2 — Product & inventory

- F2.01 product CRUD
- F2.02 categories/brands/manufacturers
- F2.03 SKU/barcode
- F2.04 units/conversions
- F2.05 variants/matrix
- F2.06 weighted products
- F2.07 batches/expiry/FEFO
- F2.08 serial/IMEI/assets
- F2.09 warranty
- F2.10 locations/bins
- F2.11 stock ledger
- F2.12 transfers
- F2.13 adjustments
- F2.14 stock count/reconciliation
- F2.15 inventory tests

## Phase 3 — Sales & cash

- F3.01 cart domain
- F3.02 pricing/tax/discount engine
- F3.03 sale transaction
- F3.04 payment abstraction
- F3.05 split payments/change
- F3.06 cash shift
- F3.07 receipt model/printing adapter
- F3.08 customer sale/debt
- F3.09 void
- F3.10 refund
- F3.11 exchange
- F3.12 sale idempotency
- F3.13 crash/retry tests

## Phase 4 — Purchasing & profitability

- F4.01 supplier model
- F4.02 purchase order
- F4.03 receiving
- F4.04 supplier invoice/payment
- F4.05 cost policy
- F4.06 COGS
- F4.07 valuation
- F4.08 profitability reports

## Phase 5 — Customers & loyalty

- F5.01 customer profile
- F5.02 customer groups
- F5.03 credit limits
- F5.04 debt ledger
- F5.05 debt payments
- F5.06 loyalty ledger
- F5.07 rewards/reversal/expiry

## Phase 6 — Offline & sync

- F6.01 device identity
- F6.02 outbox
- F6.03 idempotency store
- F6.04 sync transport
- F6.05 ACK/retry
- F6.06 conflict policies
- F6.07 recovery/quarantine
- F6.08 sync observability
- F6.09 multi-device tests

## Phase 7 — Modules

- F7.01 retail
- F7.02 fashion
- F7.03 grocery
- F7.04 electronics
- F7.05 automotive
- F7.06 pharmacy/medical retail
- F7.07 furniture/home
- F7.08 hardware/building
- F7.09 restaurant/café/fast food
- F7.10 bakery/food production
- F7.11 repair/service
- F7.12 rental
- F7.13 salon/beauty
- F7.14 wholesale/B2B
- F7.15 hospitality/events
- F7.16 custom capability composer

## Phase 8 — Licensing

- F8.01 license model
- F8.02 signed license format
- F8.03 activation
- F8.04 device entitlement
- F8.05 offline verification/grace
- F8.06 revocation
- F8.07 device reset
- F8.08 license administration
- F8.09 tamper/replay/clock tests

## Phase 9 — Website & billing

- F9.01 marketing site
- F9.02 pricing
- F9.03 account portal
- F9.04 checkout provider integration
- F9.05 license delivery
- F9.06 downloads
- F9.07 support/docs

## Phase 10 — Hardware

- F10.01 scanner
- F10.02 thermal printer
- F10.03 cash drawer
- F10.04 scale
- F10.05 label printer
- F10.06 customer display
- F10.07 hardware failure tests

## Phase 11 — Reports

- F11.01 sales
- F11.02 inventory
- F11.03 valuation/COGS
- F11.04 profit
- F11.05 cash
- F11.06 tax
- F11.07 debt/customer
- F11.08 branch comparison
- F11.09 export
- F11.10 audit viewer

## Phase 12 — Release

- F12.01 Windows packaging/signing
- F12.02 macOS packaging/signing
- F12.03 Linux packaging
- F12.04 updater signing
- F12.05 update UI/safe install
- F12.06 staged releases
- F12.07 rollback/recovery
- F12.08 release notes
- F12.09 commit production package lockfiles and enforce locked CI installs

## Phase 13 — Hardening

- F13.01 performance profiling
- F13.02 crash/error monitoring
- F13.03 backup/restore drills
- F13.04 migration upgrade tests
- F13.05 dependency/security audit
- F13.06 accessibility
- F13.07 RTL/i18n verification
- F13.08 E2E golden flows
- F13.09 pilot readiness

## Phase 14 — Launch

- F14.01 private beta
- F14.02 pilot stores
- F14.03 paid beta
- F14.04 production launch
- F14.05 incident/support process
