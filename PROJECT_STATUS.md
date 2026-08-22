# POS Global — Project Status

> Internal working name: POS Global. Proposed product brand: **Zylo**, pending trademark/domain screening and final commercial approval.

## Current state

The repository is in **Foundation Verification**. The agent-executable blueprint is designed, and the remaining work is evidence-based verification of the exact `foundation/v2` head.

### Readiness state

`FOUNDATION_DESIGNED` → **verification in progress** → `FOUNDATION_VERIFIED` → `AGENT_IMPLEMENTATION_READY`.

The project must not be treated as `AGENT_IMPLEMENTATION_READY` until the exact `foundation/v2` head has current green CI and an uploaded `foundation-gate-evidence-<commit-sha>` artifact.

### Architecture decisions
- Desktop: Tauri 2 + Rust.
- UI: React + TypeScript.
- Local source of truth for POS operations: SQLite.
- Cloud: Supabase for identity, PostgreSQL/RLS, sync coordination and future portal services.
- Authentication: Supabase Auth for online identity; local POS authorization and offline sessions remain in the Rust/domain boundary.
- Financial amounts: integer minor units; floating-point financial truth is prohibited.
- Inventory: immutable movement/ledger model; balances are derived/cacheable state.
- Sync: transactional outbox + idempotency; offline operation is a first-class requirement.
- Updates: signed Tauri updates distributed through GitHub Releases/Actions.
- License signing and update signing use separate private keys.

## Foundation verification history

A prior CI run proved the frontend build but the Rust job failed during Tauri context generation because the expected application icon was absent. The foundation now includes a deterministic SVG icon source and CI generates the required Tauri icon set before Rust verification. This is a foundation-build repair, not an excuse to mark the previous failure as passed.

The migration-test PR also added real SQLite migration, repeatability, exact-money-column and rollback tests. Those passed on the PR head; the resulting post-merge `foundation/v2` commit still requires its own exact-head CI/evidence run.

## Not yet production-ready

Core business modules, full RLS policies, cloud sync implementation, license server, billing website, hardware adapters, complete reporting, E2E testing, code signing and production deployment still require implementation and evidence.

## Evidence rule

A feature is not considered Done because code exists. It is Done only when implementation, tests, migration checks, failure-path checks, security checks, and acceptance evidence exist for its scope.
