# AGENT STATE

> This file is operational state, not a substitute for Git history or CI evidence.
> Reconciled and updated for F2.07 PR #75 remediation on 2026-09-03.

## Current

- Current Phase: Phase 2 — Product & inventory core
- Current Milestone: F2.07 — Batches, expiry dates & FEFO
- Milestone Status: PR #75 OPEN / REMEDIATION AUDITED
- Branch: `feature/f2-07-batches-expiry-fefo`
- HEAD: `f71f914820ffd5060851c882ca64649600170f6b`
- Remote HEAD: `f71f914820ffd5060851c882ca64649600170f6b` (`origin/feature/f2-07-batches-expiry-fefo`)
- PR: #75 (https://github.com/userkxm00/pos-global/pull/75)
- Working Tree: Disambiguated glob command imports in batch_tests.rs
- Last Completed Action: Disambiguated batch command test imports, verified migration 016 duplicate guards and atomic rollback
- Current Blocker: None
- Next Authorized Action: Commit and push import disambiguation to origin
- Exact F2.07 Scope Implemented:
  - ADR-0009 created and accepted (`docs/adr/0009-f2-07-batch-expiry-fefo-architecture-semantics.md`)
  - Migration file `016_batches_and_expiry.sql` created rebuilding `product_batches` with nullable `expiry_date`, integer milli precision (`quantity_milli INTEGER NOT NULL CHECK (quantity_milli >= 0)`), legacy `quantity REAL` removed, partial unique indexes for case-insensitive batch numbers, and fail-closed pre-validation
  - Registered `016_batches_and_expiry` in `MIGRATIONS` array in `src-tauri/src/db/mod.rs`
  - Domain engine in `src-tauri/src/batch/mod.rs` implementing orthogonal capability checks (`is_batch_tracked`, `is_expiry_required`, `is_fefo_enabled`), calendar date validation with leap year handling, lifecycle status transitions with terminal depleted/recalled states, and deterministic read-only FEFO planning (`plan_fefo_allocation`)
  - Tauri IPC commands in `src-tauri/src/commands/batch.rs` (`create_product_batch`, `get_product_batch`, `list_product_batches`, `update_batch_status`, `plan_fefo_allocation`) with branch/tenant scope enforcement
  - Comprehensive unit/integration/migration test suite with 25 test functions in `src-tauri/src/tests/batch_tests.rs`
- Protected Future Scope (STRICTLY PRESERVED / UNTOUCHED):
  - F2.08: Serial / IMEI / assets
  - F2.09: Warranty
  - F2.10–F2.15: Locations, bins, stock ledger, transfers, adjustments, stock count reconciliation
  - F2.19 / F7.03: Variable-weight barcode parsing (EAN-13 prefixes 20-29) and scale label printing
  - F2.23: Batch / Expiry / FEFO UI (React frontend)
  - Phase 3: Sales and cash transactions (reference slice `src-tauri/src/commands/sales.rs` remains frozen)
  - Phase 4: Purchasing, receiving (GRN), and supplier batch association
  - Phase 10: Hardware scale device drivers / protocols
- Latest Validation State:
  - `cargo fmt --check`: PASSED
  - `npm test`: PASSED
  - `npm run build`: PASSED
  - `validate_foundation.py`: PASS (372 unique backlog task IDs verified)
  - `git diff --check origin/main`: PASSED
  - Note: Windows MSVC linker unavailable locally; authoritative Rust compilation/tests deferred to GitHub Actions
- Important Decisions:
  - ADR-0006: Domain, Commercial, and Regulatory Finalization
  - ADR-0007: F2.05 Cartesian Variant Matrix Generation & SKU Architecture Semantics
  - ADR-0008: F2.06 Weighted Products Architecture & Calculation Semantics
  - ADR-0009: F2.07 Batches, Expiry Dates & FEFO Architecture & Semantics
- Session Continuity: Reconstructed following power loss; branch fast-forwarded and verified against `origin/main`
- Lessons: Active lessons ENG-001 through ENG-007 in `.agents/memory/lessons/`. Candidate: Post-Merge Remote Main Reconciliation Before Feature Branch Inception

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
| 2026-09-03 | F2.06 Initialization | Fast-forward local main to 98cbb9b, checkout feature/f2-06-weighted-products | PASS | Local HEAD `98cbb9b` |
| 2026-09-03 | F2.06 ADR-0008 | Record decisions: dedicated table, defer barcode, integer math | PASS | `docs/adr/0008-f2-06-weighted-products-architecture-semantics.md` |
| 2026-09-03 | F2.06 Migration 015 | DDL created & registered in MIGRATIONS array | CREATED / REGISTERED | `015_weighted_products.sql` & `db/mod.rs` |
| 2026-09-03 | F2.06 Rust Formatting | cargo fmt --manifest-path src-tauri/Cargo.toml --all -- --check | PASS | 0 diffs, 100% formatted |
| 2026-09-03 | F2.06 Rust Check/Test | cargo check / cargo test | UNVERIFIED LOCALLY — ENVIRONMENT LIMITATION | Host lacks link.exe/gcc for build scripts; delegated to CI |
| 2026-09-03 | F2.06 Frontend Tests | npm test | PASS | 9 suites passed, 0 failures |
| 2026-09-03 | F2.06 Frontend Build | npm run build | PASS | tsc + vite build 0 errors |
| 2026-09-03 | F2.06 Foundation Gate | python -u .github/scripts/validate_foundation.py | PASS | 372 unique tasks validated |
| 2026-09-03 | F2.06 Git Whitespace | git diff --check origin/main | PASS | clean |
| 2026-09-03 | F2.06 Weighted Products | PR #74 merged into main; merge commit 51eae14 | PASS | PR #74 merged; 35 tests passing on main |

## Known Blockers

- Local host environment lacks MSVC C++ Build Tools (`link.exe` / Windows SDK `kernel32.lib`) and MinGW GCC (`gcc.exe`); local cargo test execution fails during build-script linking for dependencies (`proc-macro2`, `ring`, `serde_core`). Full Rust test execution is delegated to GitHub Actions CI per ENG-001 and TESTING_GUIDE.md.
- Reference implementations in `src-tauri/src/commands/sales.rs` remain strictly frozen until Phase 3.
- Hardware scale drivers are deferred to Phase 10; F2.06 is domain core only.

## Handoff

When commit and PR authorization is granted:
1. Stage and commit the 10 files (5 tracked modified, 5 untracked new) under `feat(weighted): implement F2.06 weighted products domain, migration 015, commands, and tests`.
2. Push branch `feature/f2-06-weighted-products` to `origin`.
3. Open pull request into `main`.
4. Monitor live GitHub Actions CI matrix runs and SonarCloud Quality Gate.
