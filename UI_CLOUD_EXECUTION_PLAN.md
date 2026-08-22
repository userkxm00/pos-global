# POS Global — UI & Cloud Execution Plan

The implementation agent must treat UI, local Rust commands, and Supabase/cloud work as coordinated layers. A UI task is not complete when it only renders; a cloud task is not complete when RLS exists but the local offline path is broken.

## UI rules

1. React is presentation/state orchestration, never the authority for money, stock, permissions or licensing.
2. Every privileged screen maps to a typed Tauri command.
3. Every async workflow defines loading, empty, success, validation-error, authorization-error, offline, conflict and retry states where applicable.
4. Every destructive/financial action has explicit confirmation and an audit reason where required.
5. Keyboard-first POS flows are mandatory for cashier-critical screens.
6. RTL and locale formatting are first-class; hardcoded text and currency formatting are prohibited.
7. UI tests cover critical workflows in addition to domain tests.

## Phase 1 UI tasks

- `F1.11` app shell/navigation/layout
- `F1.12` onboarding wizard: organization → branch → register/device
- `F1.13` authentication screens and session lifecycle
- `F1.14` local PIN/lock/session timeout UI
- `F1.15` organization/branch/register context switcher
- `F1.16` roles/permissions administration UI
- `F1.17` offline/online/sync status indicator
- `F1.18` authorization and error-state UX

## Phase 1 cloud tasks

- `F1.19` Supabase Auth adapter
- `F1.20` organization/branch/member cloud schema
- `F1.21` device/register cloud identity
- `F1.22` RLS tenant isolation
- `F1.23` privileged server functions where required
- `F1.24` auth/session integration tests
- `F1.25` cross-tenant negative tests

## Phase 2 UI tasks

- `F2.16` product list/search/filter
- `F2.17` product editor
- `F2.18` category/brand/manufacturer management
- `F2.19` barcode/SKU editor and scanner entry
- `F2.20` unit/conversion editor
- `F2.21` matrix/variant editor grid
- `F2.22` weighted-product entry UX
- `F2.23` batch/expiry/FEFO UX
- `F2.24` serial/IMEI/warranty UX
- `F2.25` locations/transfers/adjustments UI
- `F2.26` stock-count/reconciliation workflow UI

## Phase 2 cloud/data tasks

- `F2.27` cloud product projection schema
- `F2.28` product/media storage policy
- `F2.29` stock projection/read model
- `F2.30` sync-safe inventory projections

## Phase 3 UI tasks

- `F3.14` cashier workspace shell
- `F3.15` product lookup/barcode/cart UX
- `F3.16` pricing/discount explanation UI
- `F3.17` customer selector and quick customer creation
- `F3.18` payment modal and method selection
- `F3.19` split-payment/change UX
- `F3.20` cash shift open/close/count UI
- `F3.21` receipt preview/print/reprint UX
- `F3.22` sale history/void/refund/exchange screens
- `F3.23` offline/retry/idempotency UX

## Phase 3 cloud/coordination tasks

- `F3.24` cloud sale event ingestion
- `F3.25` payment reconciliation event model
- `F3.26` audit event ingestion

## Phase 4–6 shared UI/data tasks

- `F4.09` supplier workspace
- `F4.10` purchase order/receiving UI
- `F4.11` costing/valuation explanation views
- `F5.08` customer/debt workspace
- `F5.09` loyalty configuration/history UI
- `F6.10` sync center/queue/errors/conflicts UI
- `F6.11` device registration/sync checkpoint UI
- `F6.12` recovery/quarantine workflow UI

## Phase 7 industry UI pattern

Every industry module follows a standard stack:

```text
Preset configuration
→ Capability setup
→ List/search/overview screen
→ Detail/workflow screen
→ Transaction command(s)
→ Audit/history view
→ Error/offline/recovery states
→ Tests + evidence
```

Industry-specific UI must consume shared components for products, customers, money, inventory and permissions instead of rebuilding them.

## Phase 8 licensing UI

- `F8.10` activation/onboarding license screen
- `F8.11` plan/entitlement viewer
- `F8.12` offline grace/expiry state
- `F8.13` device management
- `F8.14` license recovery/reset UI

## Phase 9 web/UI tasks

- `F9.08` public marketing shell
- `F9.09` pricing/comparison pages
- `F9.10` customer authentication
- `F9.11` account/subscription portal
- `F9.12` checkout result/recovery pages
- `F9.13` license/download portal
- `F9.14` documentation/support/status pages
- `F9.15` billing webhook ingestion and entitlement projection

## Cloud boundary

Supabase responsibilities may include:

- Auth and account identity
- cloud Postgres schema
- RLS
- sync coordination/read models
- license metadata
- subscription/entitlement projection
- admin/portal data
- audited server-side actions

The desktop app must not rely on Supabase availability for local selling, cash, inventory or hardware. Cloud functions are adapters around the domain, not the domain itself.

## Cloud security checklist

Every cloud table/function must define:

- tenant ownership
- branch scope if applicable
- authenticated role
- RLS policy
- service-role-only operations where applicable
- audit requirements
- data retention
- indexes/query access path
- sync/idempotency impact

## Webhooks

Webhooks must be:

- authenticated/verified according to the provider contract;
- idempotently applied;
- persisted before side effects where required;
- replay-safe;
- observable;
- translated into internal events rather than leaking provider payloads into domain code.
