# AGENT STATE

> This file is operational state, not a substitute for Git history or CI evidence.
> Reconciled and updated for F2.11 Stock Ledger implementation on 2026-09-07.

## Current

- Current Phase: Phase 2 — Product & inventory core
- Current Milestone: F2.11 — Stock Ledger
- Milestone Status: F2.11 IMPLEMENTED (branch `feature/f2-11-stock-ledger`, awaiting CI verification)
- Branch: `feature/f2-11-stock-ledger`
- Authoritative origin/main SHA: `990816bb6581ccc1bc5470205abee3e86deda22f`
- Last Completed Action: F2.11 Stock Ledger implementation completed according to ADR-0013. Migration 020 created, StockLedgerService implemented, IPC commands exposed, comprehensive test suite created.
- Current Blocker: None
- Next Authorized Action: Awaiting CI verification / review for F2.11.
- Exact F2.11 Scope Implemented:
  - ADR-0013 accepted (`docs/adr/0013-f2-11-stock-ledger-architecture-semantics.md`)
  - Migration `020_stock_ledger_and_spatial_balances.sql` registered in `MIGRATIONS` array in `src-tauri/src/db/mod.rs`
  - Stock ledger domain engine in `src-tauri/src/stock/mod.rs`
  - Stock IPC commands in `src-tauri/src/commands/stock.rs` registered in `src-tauri/src/commands/mod.rs` and `main.rs`
  - Comprehensive test suite in `src-tauri/src/tests/stock_ledger_tests.rs` registered in `src-tauri/src/tests/mod.rs`
- Protected Scope (STRICTLY PRESERVED / UNTOUCHED):
  - F2.12–F2.15: Transfers, adjustments, stock count reconciliation
  - F2.19 / F7.03: Variable-weight barcode parsing and scale label printing
  - F2.24: Serial / IMEI / Warranty UI (React frontend)
  - Phase 3: Sales and cash transactions (`src-tauri/src/commands/sales.rs` remains frozen)
  - Phase 4: Purchasing, receiving (GRN), and supplier batch association
  - Phase 10: Hardware scale/scanner device drivers / protocols
- Latest Validation State:
  - `npm test`: PASSED locally (100% test pass)
  - `npm run build`: PASSED locally (zero TypeScript errors, production build succeeded)
  - `cargo check/test`: BLOCKED locally due to host toolchain lacking MSVC `link.exe` (delegated to GitHub Actions CI per AGENT_STATE.md / ENG-001)
  - `git diff --check origin/main`: Verified (only F2.11 authorized scope changed)
- Important Decisions:
  - ADR-0006: Domain, Commercial, and Regulatory Finalization
  - ADR-0007: F2.05 Cartesian Variant Matrix Generation & SKU Architecture Semantics
  - ADR-0008: F2.06 Weighted Products Architecture & Calculation Semantics
  - ADR-0009: F2.07 Batches, Expiry Dates & FEFO Architecture & Semantics
  - ADR-0010: F2.08 Serial / IMEI / Assets Architecture & Semantics
  - ADR-0011: F2.09 Warranty Architecture & Lightweight Core Semantics
  - ADR-0012: F2.10 Locations & Bins Architecture & Semantics
  - ADR-0013: F2.11 Stock Ledger Architecture & Semantics
- Lessons: Active lessons ENG-001 through ENG-007 in `.agents/memory/lessons/`.

## Evidence Ledger

| Date | Task | Check | Result | Evidence |
|---|---|---|---|---|
| 2026-08-18 | Foundation documentation | Repository/PR review | PASS | GitHub PR #1 |
| 2026-08-18 | Frontend baseline | npm build | PASS | PR CI run |
| 2026-08-18 | Rust baseline | cargo check/test | PASS | PR #3 CI run |
| 2026-08-18 | Migration verification | fresh DB + repeatability + rollback + exact-money column tests | PASS | PR #3 CI run |
| 2026-08-23 | Post-merge exact-head verification | authoritative foundation-gate-evidence | PASS | Run #79 (`8f5cdfe`) |
| 2026-08-27 | Phase 1 (F1.01–F1.25) | Identity, Organization, Permissions, Auth & RLS | PASS | Merged across PRs #43–#63 |
| 2026-08-28 | F2.01 Product CRUD | SQLite product domain, IPC, money/tax invariants | PASS | PR #64 merged (`d54d319`) |
| 2026-08-28 | F2.02 Categories/Brands/Mfrs | Domain catalog, hierarchical categories, IPC | PASS | PR #65 merged (`9a7df7f`) |
| 2026-08-29 | F2.03 SKU & Barcode | Multi-barcode, check digit (EAN/UPC/Code128), collision checks | PASS | PR #66 merged (`c4fffe0`) |
| 2026-08-29 | F2.04 Units & Conversions | UOM, dimensions, multi-hop BFS conversion, migration 013 | PASS | PR #67 merged (`44d063c`) |
| 2026-09-02 | F2.05 Variants & Matrix | Cartesian generation, migration 014, SKU generator, audit | PASS | PR #73 merged (`98cbb9b`); CI #33716462109, SonarCloud Passed |
| 2026-09-03 | F2.06 Weighted Products | PR #74 merged into main; merge commit 51eae14 | PASS | PR #74 merged (`51eae14`); 35 tests passing on main |
| 2026-09-03 | F2.07 Batches & Expiry | PR #75 merged into main; merge commit 5e525ec | PASS | PR #75 merged (`5e525ec`); 36 tests passing on main |
| 2026-09-03 | F2.08 Serial / IMEI / Assets | PR #76 merged into main; merge commit 341b54b | PASS | PR #76 merged (`341b54b`); 36 tests passing on main, exact-head CI #33782675305 green |
| 2026-09-04 | F2.09 Warranty Core & Index | PR #77 merged into main; merge commit 05b9fed; 481 Rust tests pass in CI; SonarCloud/CodeQL/Supabase clean | PASS | PR #77 merged (`05b9fed`); exact-head CI #33851161602 Job #100962272791 green |
| 2026-09-07 | F2.10 Locations & Bins | PR #78 merged into main; merge commit c882a8b; discrete two-entity model, composite same-branch FK, anti-existence leakage protection | PASS | PR #78 merged (`c882a8b`); exact-head CI green |
| 2026-09-07 | F2.11 Stock Ledger | ADR-0013 accepted; Migration 020; StockLedgerService single write authority; 8 partial unique indexes; immutable movements | PASS (local) | Local npm test & build clean; awaiting remote CI |

## Known Blockers

- Local host environment lacks MSVC C++ Build Tools (`link.exe` / Windows SDK `kernel32.lib`) and MinGW GCC (`gcc.exe`); local cargo test execution fails during build-script linking for dependencies (`proc-macro2`, `ring`, `serde_core`). Full Rust test execution is delegated to GitHub Actions CI per ENG-001 and TESTING_GUIDE.md.
- Reference implementations in `src-tauri/src/commands/sales.rs` remain strictly frozen until Phase 3.
- Hardware scale drivers are deferred to Phase 10; F2.06 is domain core only.

## Handoff

F2.11 (Stock Ledger) implementation is complete on branch `feature/f2-11-stock-ledger`.
Next action: Push branch and verify remote GitHub Actions CI execution.
Do not begin F2.12 or subsequent milestones without explicit user authorization.
