# POS Global — Project Status

> Internal working name: POS Global. Brand name remains unconfirmed pending trademark/domain screening.

## Current state

The repository is in **Foundation Gate**. The goal is to build a commercial, offline-first, multi-industry POS platform rather than repeat the architectural mistakes of Mellah-POS-V2.

### Architecture decisions
- Desktop: Tauri 2 + Rust.
- UI: React + TypeScript.
- Local source of truth for POS operations: SQLite.
- Cloud: Supabase for identity, PostgreSQL cloud data, RLS, sync coordination and future portal services.
- Authentication: Supabase Auth for online identity; local POS authorization and offline sessions remain in the Rust/domain boundary.
- Financial amounts: integer minor units; floating-point financial truth is prohibited.
- Inventory: immutable movement/ledger model; balances are derived/cacheable state.
- Sync: transactional outbox + idempotency; offline operation is a first-class requirement.
- Updates: signed Tauri updates distributed through GitHub Releases/Actions.
- License signing and update signing use separate private keys.

## Not yet production-ready
Core business modules, full RLS policies, cloud sync implementation, license server, billing website, hardware adapters, complete reporting, E2E testing, code signing and production deployment still require implementation and evidence.

## Evidence rule
A feature is not considered Done because code exists. It is Done only when implementation, tests, migration checks, failure-path checks, and acceptance evidence exist for its scope.
