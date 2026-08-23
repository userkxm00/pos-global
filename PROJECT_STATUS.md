# POS Global — Project Status

> Internal working name: POS Global. Proposed product brand: **Zylo**, pending trademark/domain screening and final commercial approval.

## Current state

The repository has completed the Foundation Gate on the exact pre-state-synchronization `main` head. The foundation is technically verified; the state-synchronization commit now requires its own exact-head Foundation Gate run before the project advances to `AGENT_IMPLEMENTATION_READY`.

### Readiness state

`FOUNDATION_DESIGNED` → **`FOUNDATION_VERIFIED`** → `AGENT_IMPLEMENTATION_READY`.

The Foundation Gate is authoritative for the exact `main` head. The last completed evidence run was Foundation Gate Evidence #79 on commit `8f5cdfe9e96c7a2fab5d4afaeef36f3bc1499098`, with an uploaded exact-head evidence artifact.

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

Foundation Gate Evidence #79 completed successfully on the exact `main` head `8f5cdfe9e96c7a2fab5d4afaeef36f3bc1499098`. The run verified exact-head identity, frontend build, production dependency audit, Tauri Linux dependencies/icon generation, Rust formatting/check/Clippy/tests, foundation specification validation, secret scanning, and uploaded authoritative exact-head evidence.

A prior CI run proved the frontend build but the Rust job failed during Tauri context generation because the expected application icon was absent. The foundation now includes a deterministic SVG icon source and CI generates the required Tauri icon set before Rust verification. This is a foundation-build repair, not an excuse to mark the previous failure as passed.

The migration-test PR also added real SQLite migration, repeatability, exact-money-column and rollback tests. Those passed on the PR head; the post-merge exact-head run above independently verified the resulting main state.

## Reference implementation boundary

`src-tauri/src/commands/sales.rs` contains a deliberately retained **seed/reference slice** of sale behavior used to validate critical transaction, idempotency, stock-ledger, exact-money, and branch-scoping invariants during Foundation work.

This code **does not mean Phase 3 is complete, does not unlock Phase 3 tasks, and is not authorization for an agent to continue expanding sales functionality during Foundation or Phase 1/2 work**. The implementation is frozen unless an explicitly assigned task targets it.

The reference slice must not be treated as the final authorization model. In particular, `create_sale` currently receives user/branch/shift identifiers through its request boundary and validates their relationships to the open shift; it does not replace the future authenticated-session and Rust authorization context defined for Phase 1. Phase 1 authorization work must close this boundary before the sale path is treated as production-ready.

The authoritative Phase 3 completion criteria remain the Phase 3 backlog, task dependencies, acceptance criteria, tests, and evidence. Existing reference code never changes phase readiness or grants permission to skip earlier tasks.

## Not yet production-ready

Core business modules, full RLS policies, cloud sync implementation, license server, billing website, hardware adapters, complete reporting, E2E testing, code signing and production deployment still require implementation and evidence.

## Evidence rule

A feature is not considered Done because code exists. It is Done only when implementation, tests, migration checks, failure-path checks, security checks, and acceptance evidence exist for its scope.
