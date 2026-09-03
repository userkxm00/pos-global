# AGENT STATE

> This file is operational state, not a substitute for Git history or CI evidence.
> Reconciled and updated for F2.06 pre-commit audit on 2026-09-03.

## Current

- Current Phase: Phase 2 — Product & inventory core
- Current Milestone: F2.06 — Weighted products
- Milestone Status: PR_OPEN_CI_TRIAGE (PR #74 open; CodeRabbit normalized result weights, Greptile P1 unit catalog boundary, and unauth session assertion remediated)
- Branch: `feature/f2-06-weighted-products` (local workspace) / `origin/feature/f2-06-weighted-products` (remote)
- HEAD: `a209272be7dfcb2453db9782cf9e4deb26b021a5`
- Remote HEAD: `a209272be7dfcb2453db9782cf9e4deb26b021a5` (`origin/feature/f2-06-weighted-products`)
- PR: #74 (open, https://github.com/userkxm00/pos-global/pull/74)
- PR HEAD: `a209272be7dfcb2453db9782cf9e4deb26b021a5`
- Working Tree: Forensic remediation verified; awaiting commit authorization
- Last Completed Action: Commit a209272 pushed; exact CI test failure analyzed (test harness assertions); CodeRabbit normalized result weights and Greptile mass boundary resolved; tests updated
- Current Blocker: Local environment lacks MSVC C++ Build Tools (`link.exe` / `kernel32.lib`) and MinGW GCC (`gcc.exe`); authoritative cargo test execution runs in remote GitHub Actions CI.
- Next Authorized Action: Awaiting review of the completed remediation
- Exact F2.06 Scope Implemented:
  - Migration file `015_weighted_products.sql` created with table `product_weight_configs` (no redundant index, no premature hardware columns)
  - Registered `015_weighted_products` in `MIGRATIONS` array in `src-tauri/src/db/mod.rs`
  - Strict mass dimension enforcement (`UnitDimension::Mass`) via canonical relationship `products.unit_type -> units.code COLLATE NOCASE`
  - Tare weight subtraction and non-negative net weight validation (`gross >= tare >= 0`)
  - Checked integer half-up price calculation (`floor((net_weight_milli * unit_price_minor + 500) / 1000)`)
  - Exact integer cross-unit metric mass normalization (kg <-> g with zero floating-point math)
  - Tauri IPC commands (`set_product_weight_config`, `get_product_weight_config`, `delete_product_weight_config`, `calculate_weighted_item`) behind permission checks
  - Unit and integration test suite with 31 test functions in `src-tauri/src/tests/weighted_tests.rs`
- Protected Future Scope (STRICTLY PRESERVED / UNTOUCHED):
  - F2.07: Batches / expiry / FEFO
  - F2.08: Serial / IMEI / assets
  - F2.09: Warranty
  - F2.10–F2.15: Locations, bins, stock ledger, transfers, adjustments, stock count reconciliation
  - F2.19 / F7.03: Variable-weight barcode parsing (EAN-13 prefixes 20-29) and scale label printing
  - F2.22: Weighted-product entry UX (React POS UI)
  - Phase 3: Sales and cash transactions (reference slice `src-tauri/src/commands/sales.rs` remains frozen)
  - Phase 10: Hardware scale device drivers / protocols (RS-232, OPOS, USB HID, CAS, Toledo)
- Latest CI State:
  - Main Merge Head (`98cbb9b`): CI Run #33716462109 SUCCESS, Foundation Gate Evidence Run #33716462107 SUCCESS, CodeQL Run #33716462092 SUCCESS
- Latest Review State: SonarCloud Quality Gate PASSED (PR #73 merged). Clean base for F2.06.
- Important Decisions:
  - ADR-0006: Domain, Commercial, and Regulatory Finalization
  - ADR-0007: F2.05 Cartesian Variant Matrix Generation & SKU Architecture Semantics
  - ADR-0008: F2.06 Weighted Products Architecture & Calculation Semantics
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
