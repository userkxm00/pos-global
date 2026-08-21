# MASTER BACKLOG

Tasks are intentionally granular. An agent must not skip a gate to reach a later phase. The backlog is the executable index; detailed workflow decomposition lives in `UI_CLOUD_EXECUTION_PLAN.md` and `INDUSTRY_EXECUTION_PLAN.md`.

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
- F0.7.10 verify UI/cloud execution plan
- F0.7.11 verify industry execution plan
- F0.7.12 verify capability matrix
- F0.7.13 verify task dependency graph
- F0.7.14 Agent Readiness Gate review

## Phase 1 — Identity & tenancy

### Domain/cloud
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
- F1.19 Supabase Auth adapter hardening
- F1.20 organization/branch/member cloud schema
- F1.21 device/register cloud identity
- F1.22 RLS tenant isolation verification
- F1.23 privileged server functions where required
- F1.24 auth/session integration tests
- F1.25 cross-tenant negative tests

### UI/UX
- F1.11 app shell/navigation/layout
- F1.12 onboarding wizard: organization → branch → register/device
- F1.13 authentication screens and session lifecycle
- F1.14 local PIN/lock/session timeout UI
- F1.15 organization/branch/register context switcher
- F1.16 roles/permissions administration UI
- F1.17 offline/online/sync status indicator
- F1.18 authorization/error-state UX

## Phase 2 — Product & inventory

### Domain
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

### UI/data/cloud
- F2.16 product list/search/filter
- F2.17 product editor
- F2.18 category/brand/manufacturer management
- F2.19 barcode/SKU editor and scanner entry
- F2.20 unit/conversion editor
- F2.21 matrix/variant editor grid
- F2.22 weighted-product entry UX
- F2.23 batch/expiry/FEFO UX
- F2.24 serial/IMEI/warranty UX
- F2.25 locations/transfers/adjustments UI
- F2.26 stock-count/reconciliation workflow UI
- F2.27 cloud product projection schema
- F2.28 product/media storage policy
- F2.29 stock projection/read model
- F2.30 sync-safe inventory projections

## Phase 3 — Sales & cash

### Domain
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

### UI/cloud
- F3.14 cashier workspace shell
- F3.15 product lookup/barcode/cart UX
- F3.16 pricing/discount explanation UI
- F3.17 customer selector and quick customer creation
- F3.18 payment modal and method selection
- F3.19 split-payment/change UX
- F3.20 cash shift open/close/count UI
- F3.21 receipt preview/print/reprint UX
- F3.22 sale history/void/refund/exchange screens
- F3.23 offline/retry/idempotency UX
- F3.24 cloud sale event ingestion
- F3.25 payment reconciliation event model
- F3.26 audit event ingestion

## Phase 4 — Purchasing & profitability

- F4.01 supplier model
- F4.02 purchase order
- F4.03 receiving
- F4.04 supplier invoice/payment
- F4.05 cost policy
- F4.06 COGS
- F4.07 valuation
- F4.08 profitability reports
- F4.09 supplier workspace
- F4.10 purchase order/receiving UI
- F4.11 costing/valuation explanation views

## Phase 5 — Customers & loyalty

- F5.01 customer profile
- F5.02 customer groups
- F5.03 credit limits
- F5.04 debt ledger
- F5.05 debt payments
- F5.06 loyalty ledger
- F5.07 rewards/reversal/expiry
- F5.08 customer/debt workspace
- F5.09 loyalty configuration/history UI

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
- F6.10 sync center/queue/errors/conflicts UI
- F6.11 device registration/sync checkpoint UI
- F6.12 recovery/quarantine workflow UI

## Phase 7 — Industry modules

Industry work is decomposed in `INDUSTRY_EXECUTION_PLAN.md`. Each family follows capability → workflow → UI/commands → transaction invariants → tests → acceptance evidence.

### Retail
- F7.01.01 general retail preset
- F7.01.02 catalog/search workflow
- F7.01.03 promotions/basic price rules
- F7.01.04 returns/exchanges workflow
- F7.01.05 multi-location retail views
- F7.01.06 acceptance/evidence

### Fashion/Shoes
- F7.02.01 fashion preset
- F7.02.02 size/color/material attributes
- F7.02.03 matrix grid operations
- F7.02.04 seasonal/collection metadata
- F7.02.05 variant-aware purchasing/returns
- F7.02.06 acceptance/evidence

### Grocery/Convenience/Produce
- F7.03.01 grocery preset
- F7.03.02 weighted/variable quantity workflow
- F7.03.03 batch/expiry/FEFO workflow
- F7.03.04 weighed-price validation
- F7.03.05 waste/shrinkage workflow
- F7.03.06 acceptance/evidence

### Electronics/Mobile/Appliances
- F7.04.01 electronics preset
- F7.04.02 serial/IMEI intake and lookup
- F7.04.03 warranty linkage
- F7.04.04 asset/customer handoff
- F7.04.05 return/DOA workflow
- F7.04.06 acceptance/evidence

### Automotive
- F7.05.01 automotive preset
- F7.05.02 vehicle/customer association
- F7.05.03 parts/SKU compatibility metadata
- F7.05.04 workshop/service order composition
- F7.05.05 labor + parts billing
- F7.05.06 acceptance/evidence

### Pharmacy/Medical retail
- F7.06.01 pharmacy preset boundary
- F7.06.02 batch/expiry/FEFO
- F7.06.03 controlled-product capability gate
- F7.06.04 prescription/regulated workflow adapter boundary
- F7.06.05 jurisdiction-specific compliance checks
- F7.06.06 acceptance/evidence

### Furniture/Home
- F7.07.01 furniture preset
- F7.07.02 dimensions/material/options
- F7.07.03 deposits/holds
- F7.07.04 delivery/fulfillment status
- F7.07.05 warranty/returns linkage
- F7.07.06 acceptance/evidence

### Hardware/Building
- F7.08.01 hardware preset
- F7.08.02 units/packaging/conversions
- F7.08.03 cut-to-measure workflow
- F7.08.04 B2B pricing/credit
- F7.08.05 supplier/stock workflow
- F7.08.06 acceptance/evidence

### Restaurant/Café/Fast Food
- F7.09.01 restaurant/café preset
- F7.09.02 tables/sections/floor plan
- F7.09.03 open orders and table transfer
- F7.09.04 modifiers/options/notes
- F7.09.05 order routing
- F7.09.06 kitchen display system adapter
- F7.09.07 courses/hold/fire workflow
- F7.09.08 split/merge bills and service charges
- F7.09.09 tips/gratuity policy adapter
- F7.09.10 recipes/ingredient consumption
- F7.09.11 waste/comps/void policy
- F7.09.12 acceptance/evidence

### Bakery/Food production
- F7.10.01 bakery preset
- F7.10.02 recipes/BOM
- F7.10.03 ingredient consumption
- F7.10.04 batch production
- F7.10.05 yield/waste tracking
- F7.10.06 expiry/FEFO
- F7.10.07 acceptance/evidence

### Repair/Service
- F7.11.01 service preset
- F7.11.02 service ticket/intake
- F7.11.03 customer asset/device
- F7.11.04 diagnosis/status workflow
- F7.11.05 parts + labor lines
- F7.11.06 estimate → approval → work → completion
- F7.11.07 warranty/return workflow
- F7.11.08 acceptance/evidence

### Rental
- F7.12.01 rental preset
- F7.12.02 asset inventory/condition
- F7.12.03 availability calendar
- F7.12.04 reservation/hold
- F7.12.05 contract/check-out
- F7.12.06 return/check-in/condition
- F7.12.07 deposit/late fee policy adapter
- F7.12.08 acceptance/evidence

### Salon/Beauty
- F7.13.01 salon preset
- F7.13.02 services/catalog
- F7.13.03 appointments/calendar
- F7.13.04 staff/resource assignment
- F7.13.05 product + service ticketing
- F7.13.06 packages/memberships
- F7.13.07 acceptance/evidence

### Wholesale/B2B
- F7.14.01 wholesale preset
- F7.14.02 customer/company profiles
- F7.14.03 price lists/tiers
- F7.14.04 credit limits/terms
- F7.14.05 quotations/orders
- F7.14.06 delivery/invoice workflow
- F7.14.07 acceptance/evidence

### Hospitality
- F7.15.01 hospitality preset
- F7.15.02 guest/customer profile
- F7.15.03 room/resource inventory abstraction
- F7.15.04 reservation integration boundary
- F7.15.05 charges/folios interface
- F7.15.06 POS charge-posting adapter
- F7.15.07 acceptance/evidence

### Events
- F7.16.01 events preset
- F7.16.02 event/catalog setup
- F7.16.03 ticket/registration inventory
- F7.16.04 reservations/holds
- F7.16.05 check-in/scan adapter
- F7.16.06 refunds/cancellations
- F7.16.07 acceptance/evidence

### Custom capability composer
- F7.17.01 capability selection model
- F7.17.02 dependency validation
- F7.17.03 incompatible-capability rules
- F7.17.04 plan/jurisdiction enforcement
- F7.17.05 onboarding preview
- F7.17.06 acceptance/evidence

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
- F8.10 activation/onboarding license screen
- F8.11 plan/entitlement viewer
- F8.12 offline grace/expiry state
- F8.13 device management
- F8.14 license recovery/reset UI

## Phase 9 — Website & billing

- F9.01 marketing site
- F9.02 pricing
- F9.03 account portal
- F9.04 checkout provider integration
- F9.05 license delivery
- F9.06 downloads
- F9.07 support/docs
- F9.08 public marketing shell
- F9.09 pricing/comparison pages
- F9.10 customer authentication
- F9.11 account/subscription portal
- F9.12 checkout result/recovery pages
- F9.13 license/download portal
- F9.14 documentation/support/status pages
- F9.15 billing webhook ingestion and entitlement projection

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

## Backlog invariants

Every implementation task must link to its detailed contract, dependency prerequisites, acceptance criteria and evidence requirements. A large feature family is never represented by one vague task when it contains independent domain/UI/cloud workflows.
