# POS Global

Commercial, offline-first, multi-industry POS and store-management platform.

> The public brand is intentionally not frozen yet. Do not use a production domain, package identifier, license namespace or marketing asset until brand/trademark/domain screening is complete.

## Stack
- Tauri 2 + Rust desktop core
- React + TypeScript UI
- SQLite local operational database
- Supabase Auth + PostgreSQL/RLS for cloud identity and data
- GitHub Actions + signed Tauri releases for distribution and updates

## Non-negotiable principles
1. Offline-first: a store continues selling without internet.
2. Sales, payments, cash and stock mutations are atomic.
3. Integer minor units are authoritative for money.
4. Organization/branch/register boundaries are explicit and enforced.
5. Industries enable reusable capabilities; they are not separate hard-coded applications.
6. A feature is not Done without implementation and evidence.
7. Secrets and private signing keys never enter the repository or desktop client.

## Development order
1. Foundation Gate
2. Identity / organization / branch / register / permissions
3. Product / units / capabilities / inventory
4. Sales / payments / cash / customers / debt / refund
5. Purchasing / COGS / pricing / tax / loyalty
6. Offline outbox / sync / conflict recovery
7. Industry modules
8. Licensing / website / billing
9. Hardware / reporting
10. Signed releases / auto-update

Read `ARCHITECTURE.md`, `SCHEMA.md`, `EXECUTION_PLAN.md`, `PROJECT_STATUS.md`, `V2_RULES.md`, and `docs/FOUNDATION_GATE.md` before implementing features.