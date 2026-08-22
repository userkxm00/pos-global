# POS GLOBAL — DETAILED EXECUTION PLAN

This document expands the master roadmap into an agent-executable sequence. It is intentionally implementation-oriented. It does not replace `EXECUTION_PLAN.md`; it operationalizes it.

## Phase 0 — Foundation Gate

### Deliverables
- architecture and domain boundaries
- schema and migration rules
- agent operating system
- task specification/ready/done contracts
- domain contracts
- security model
- sync specification
- product/UI/release specifications
- CI and testing strategy
- backlog/state tracking

### Required verification
- frontend build
- Rust check/test
- migration apply on empty DB
- migration upgrade test
- secret scan
- dependency audit
- architecture/document consistency review

### Exit
No unresolved critical architecture/security/financial decision. PR is reviewable and reproducible.

## Phase 1 — Identity & Tenancy

Order: organization → branch → device/register → cloud identity → local user/session → roles → permissions → Rust authorization → RLS → tests.

Acceptance: two organizations cannot read or mutate each other's data; branch-scoped permissions work; unauthorized privileged commands fail outside the UI.

## Phase 2 — Product & Inventory

Order: product identity → categories/brands → SKU/barcode → units → variants/matrix → weighted quantities → batch/expiry → serial/IMEI → warranty → locations → balances → movement ledger → transfers → adjustments → counts.

Acceptance: every mutation is atomic and ledger-backed; variant/weight/batch/serial combinations work without hardcoded industry forks.

## Phase 3 — Sales & Cash

Order: pricing/tax policy → cart → sale domain → payment abstraction → cash shift → sale completion → receipt → customer/debt → void → refund → exchange.

Acceptance: exact totals; split payments; stock/cash/debt reconciliation; idempotent retries; crash atomicity; permission checks.

## Phase 4 — Purchasing & Profitability

Order: supplier → purchase order → receiving → supplier invoice/payment → cost policy → COGS → valuation → margin/reporting.

Acceptance: inventory cost and historical COGS reconcile to ledger records.

## Phase 5 — Customers & Loyalty

Order: customer profile → groups → credit policy → debt ledger → payments → loyalty ledger → earn/redeem/reverse/expire → customer pricing.

Acceptance: no direct balance edits without ledger evidence.

## Phase 6 — Offline Sync

Order: device identity → outbox → idempotency → transport → ACK → retry → conflict resolver → quarantine/recovery → observability.

Acceptance: offline sale survives restart and reconnects exactly once; duplicate, timeout-after-commit, out-of-order and concurrent-device scenarios pass.

## Phase 7 — Industry Modules

Build shared capabilities first, then module workflows. Each module must prove that shared sales/inventory/financial invariants remain intact.

Initial coverage: Retail, Fashion, Grocery, Electronics, Automotive, Pharmacy/Medical Retail, Furniture/Home, Hardware/Building, Restaurant, Café, Fast Food, Bakery/Food Production, Repair, Service, Rental, Salon/Beauty, Wholesale/B2B, Hospitality, Events, and Custom capability composition.

Do not build a separate codebase per industry.

## Phase 8 — Licensing

Order: plan/entitlement model → signed license schema → activation → device binding → offline verification/grace → revocation → reset → administration → abuse/tamper tests.

Acceptance: forged/replayed/expired/revoked licenses are handled according to policy; valid offline use works within approved grace.

## Phase 9 — Website & Billing

Order: brand/domain decision → marketing site → pricing → account portal → billing provider decision → checkout → license delivery → download/support/docs.

Commercial/legal/provider choices require their own ADR before production integration.

## Phase 10 — Hardware

Implement provider-neutral interfaces first, then device adapters. Scanner → printer → drawer → scale → label printer → customer display.

Acceptance: device failure cannot corrupt an already committed transaction.

## Phase 11 — Reports & Operations

Build reports from authoritative transaction/ledger data. Required families: sales, inventory, valuation, COGS, profit, cash, tax, debt, customer, product, branch, audit, exports.

Every report must document its source-of-truth query/model.

## Phase 12 — Release Engineering

Order: packaging → code signing → protected CI secrets → release workflow → staged releases → updater metadata/signatures → safe install → rollback/recovery.

Acceptance: signed artifact installs, updater verifies signature, application restarts safely, database remains compatible.

## Phase 13 — Production Hardening

Performance, crash/error monitoring, backups, restore drills, migration upgrade tests, security review, dependency audit, accessibility, RTL/i18n, E2E, pilot readiness.

## Phase 14 — Launch

Private beta → monitored pilot → paid beta → production. Define support, incident response, release rollback, data recovery, and customer communication before paid production.

## Phase gate protocol

At every phase:

1. freeze scope;
2. verify prerequisites;
3. execute tasks in dependency order;
4. run required tests;
5. review security/data implications;
6. update acceptance matrix;
7. record evidence;
8. update agent state;
9. obtain gate approval;
10. only then unlock the next phase.
