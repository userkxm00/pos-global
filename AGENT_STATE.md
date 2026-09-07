# AGENT STATE

> This file is operational state, not a substitute for Git history or CI evidence.
> Reconciled and updated for F2.09 post-merge reconciliation on 2026-09-04.

## Current

- Current Phase: Phase 2 — Product & inventory core
- Current Milestone: F2.10 — Locations / Bins
- Milestone Status: F2.10 IN PROGRESS (PR #78 Open)
- Branch: `feature/f2-10-locations-bins`
- Branch Status: PR #78 open (`https://github.com/userkxm00/pos-global/pull/78`), implementation complete, undergoing remote CI and review gate reconciliation
- Latest Merged PR: PR #77 (`https://github.com/userkxm00/pos-global/pull/77`)
- Authoritative Merge Commit SHA: `05b9fed42fa1a30d97a7f1f6c08d19f1a515d917`
- Authoritative origin/main SHA: `05b9fed42fa1a30d97a7f1f6c08d19f1a515d917`
- Last Completed Action: Remediated review findings on PR #78 for mutation anti-existence leakage, SQLite LIKE wildcard escaping, cognitive complexity refactoring, and composite foreign key enforcement.
- Current Blocker: None
- Next Authorized Action: Await final remote review gates and user merge authorization; do not merge PR #78 autonomously.
- Exact F2.09 Scope Merged:
  - ADR-0011 accepted (`docs/adr/0011-f2-09-warranty-architecture-semantics.md`)
  - Migration `018_warranty.sql` registered in `MIGRATIONS` array in `src-tauri/src/db/mod.rs`
  - Warranty domain engine in `src-tauri/src/warranty/mod.rs`
  - Warranty IPC commands in `src-tauri/src/commands/warranty.rs` registered in `main.rs`
  - Test suite in `src-tauri/src/tests/warranty_tests.rs` (13 tests in CI covering all requirements)
- Protected Scope (STRICTLY PRESERVED / UNTOUCHED):
  - F2.10–F2.15: Locations, bins, stock ledger, transfers, adjustments, stock count reconciliation
  - F2.19 / F7.03: Variable-weight barcode parsing and scale label printing
  - F2.24: Serial / IMEI / Warranty UI (React frontend)
  - Phase 3: Sales and cash transactions (`src-tauri/src/commands/sales.rs` remains frozen)
  - Phase 4: Purchasing, receiving (GRN), and supplier batch association
  - Phase 10: Hardware scale/scanner device drivers / protocols
- Latest Validation State:
  - `cargo fmt --check`: PASSED in CI
  - `npm test`: PASSED in CI
  - `npm run build`: PASSED in CI
  - `validate_foundation.py`: PASSED in CI
  - `git diff --check origin/main`: PASSED (zero diff, clean worktree)
  - Authoritative exact-head post-merge CI: PASS (Run #33851161602 Job #100962272791; Run #33851161629; 11/11 checks green)
- Important Decisions:
  - ADR-0006: Domain, Commercial, and Regulatory Finalization
  - ADR-0007: F2.05 Cartesian Variant Matrix Generation & SKU Architecture Semantics
  - ADR-0008: F2.06 Weighted Products Architecture & Calculation Semantics
  - ADR-0009: F2.07 Batches, Expiry Dates & FEFO Architecture & Semantics
  - ADR-0010: F2.08 Serial / IMEI / Assets Architecture & Semantics
  - ADR-0011: F2.09 Warranty Architecture & Lightweight Core Semantics
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

## Known Blockers

- Local host environment lacks MSVC C++ Build Tools (`link.exe` / Windows SDK `kernel32.lib`) and MinGW GCC (`gcc.exe`); local cargo test execution fails during build-script linking for dependencies (`proc-macro2`, `ring`, `serde_core`). Full Rust test execution is delegated to GitHub Actions CI per ENG-001 and TESTING_GUIDE.md.
- Reference implementations in `src-tauri/src/commands/sales.rs` remain strictly frozen until Phase 3.
- Hardware scale drivers are deferred to Phase 10; F2.06 is domain core only.

## Handoff

F2.10 (Locations / Bins) implementation is complete on branch `feature/f2-10-locations-bins` with PR #78 open (`https://github.com/userkxm00/pos-global/pull/78`).
Remediated review findings:
- Enforced branch scope requirement on `list_bins_impl` to prevent unscoped cross-branch bin enumeration.
- Enforced airtight anti-existence leakage on `get_location_impl` and `get_bin_impl` returning `Ok(None)`.
- Enforced unified anti-existence leakage on all 7 branch-scoped mutation commands (`update_location_impl`, `deactivate_location_impl`, `reactivate_location_impl`, `create_bin_impl`, `update_bin_impl`, `deactivate_bin_impl`, `reactivate_bin_impl`) returning `not found or inaccessible for this session`.
- Escaped SQLite LIKE wildcards (`%`, `_`, `\`) in `list_locations` and `list_bins` via `crate::db::escape_like_pattern` with `ESCAPE '\\'`.
- Enforced permission-first check on all location/bin mutation commands prior to DB queries.
- Added tri-state serde deserialization for `parent_id` and `location_type` in `UpdateLocationInput`.
- Enforced active parent validation when generic `update_location` reactivates a child location.
- Enforced database-level same-branch hierarchy isolation in Migration 019 via composite foreign key `(parent_id, branch_id) REFERENCES locations(id, branch_id) ON DELETE RESTRICT` supported by unique index `idx_locations_id_branch_id`.
- Removed redundant `idx_locations_parent_id` index in favor of covering `idx_locations_parent_branch`.
- Reduced cognitive complexity in `validate_parent_hierarchy` (3 <= 15) and `update_location` (3 <= 15) and applied idiomatic `AsRef::as_ref`.
- Added database-level regression test `test_database_composite_foreign_key_same_branch_constraint` and IPC anti-leakage regression tests.
Awaiting validation, push, and PR CI/review gates reconciliation.
