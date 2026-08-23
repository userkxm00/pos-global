# POS Global — Product Strategy & MVP Gate

## Purpose

This document complements the engineering `EXECUTION_PLAN.md`. It answers the product questions that an implementation plan alone cannot answer: who we are building for first, what must be sold first, why a customer should choose POS Global, what is explicitly out of MVP scope, and what evidence is required before expanding the product surface.

## Current status

The product direction is intentionally **not fully decided yet**. Critical market, ICP, MVP industry, launch geography, pricing model and primary reason-to-buy must be explicitly approved before the implementation agent is allowed to expand beyond the shared foundation and the tasks already authorized by the repository state.

Until those decisions are approved:

- Do not implement multiple industry modules merely because they exist in the roadmap.
- Do not treat the Phase 7 industry list as MVP scope.
- Do not add integrations solely because they appear in competitor products.
- Do not move mobile into the MVP without explicit product approval.
- Do not invent pricing, launch geography, target customer, or product positioning.

## MVP decision record — required before commercial build expansion

The following must be approved and recorded in an ADR or a dated product decision:

1. **Initial launch market** — country/region and regulatory scope.
2. **Initial ICP** — the first merchant/customer segment we are optimizing for.
3. **Initial industry** — the first industry or tightly bounded industry group.
4. **Primary reason-to-buy** — one clear problem POS Global solves materially better than realistic alternatives.
5. **MVP capabilities** — the smallest set that allows a real merchant to operate and pay for the product.
6. **Explicit non-goals** — features and industries that will not be built for MVP.
7. **Business model** — subscription, perpetual, hybrid, freemium, or another explicitly approved model.
8. **Pricing hypothesis** — an initial testable price/packaging hypothesis; not a permanent commitment.
9. **Launch milestone** — a measurable MVP/pilot target, not an invented deadline without evidence.

## MVP principles

### 1. Core before breadth

The MVP should reuse the same shared foundation used by the long-term product:

- exact financial representations
- tenant/organization boundaries
- branch/device identity
- authoritative Rust authorization
- inventory ledger
- idempotent transaction boundaries
- offline-first local transaction safety
- explicit sync contracts
- auditability
- i18n/RTL readiness

The goal is to avoid building a disposable MVP that must later be rewritten.

### 2. Narrow industry scope

The roadmap may contain many industry families, but MVP must select one tightly bounded commercial scope. Additional industries remain roadmap work until the MVP proves product/market fit signals.

### 3. Mobile is future-compatible, not automatically MVP

The desktop application remains the current primary POS execution surface unless the product decision explicitly changes that.

From the beginning, shared contracts must remain consumable by future mobile clients. This means APIs/contracts, sync events, identity, permissions, financial invariants, device identity and versioning must not be designed as desktop-only.

The mobile companion remains a later release unless an approved MVP decision promotes a specific mobile workflow.

### 4. Integrations are boundary-first

The product should define stable integration boundaries before committing to every provider. Initial MVP integration scope must be approved explicitly.

Potential integration families include:

- accounting/export
- e-commerce
- payments
- import/export
- messaging/notifications

Do not add a provider dependency without a documented reason, eligibility/security/licensing review, and tests.

## MVP customer workflow

A first merchant should be able to reach a first successful transaction through a short, testable onboarding path:

```text
Create business
    ↓
Choose initial business/industry preset
    ↓
Configure branch + currency + language
    ↓
Create first user / role
    ↓
Import or add initial products
    ↓
Set opening stock where applicable
    ↓
Open shift / selling session
    ↓
Complete first sale
    ↓
View basic confirmation/report
```

The actual steps may evolve, but the first-run experience must be treated as a product requirement, not left as an accidental result of implementation.

## Data migration requirement

MVP should have a safe path for importing at least the data needed to replace a typical legacy workflow. Exact source formats are a product decision, but the architecture should support:

- products/SKU/barcode
- customers
- opening stock
- prices
- suppliers where required
- basic legacy balances where explicitly supported

Import must support validation, preview, error reporting and a safe failure path. Do not silently discard malformed records.

## Global readiness without global overbuild

“Global” is a product direction, not permission to implement every jurisdiction immediately.

The product architecture should be ready for:

- localization and RTL
- locale-aware date/number/currency formatting
- multiple currencies
- timezone handling
- jurisdiction-specific tax/compliance adapters
- versioned contracts and sync

However, jurisdiction-specific behavior must remain evidence-driven and gated by the commercial/regulatory plan.

## Competitive positioning

Before launch, the product owner must maintain a lightweight competitive record covering the alternatives most relevant to the selected MVP market.

The purpose is not to copy competitors. It is to answer:

> Why should the first merchant choose POS Global instead of the most realistic alternative?

The answer must be concrete enough to influence MVP scope, onboarding, pricing and messaging.

## Milestone model

Use milestone gates instead of speculative calendar promises.

### MVP Ready

Required:
- approved MVP scope
- approved target market and ICP
- core business workflow implemented
- required security and financial invariants tested
- onboarding path usable
- data import path tested if in scope
- CI/security checks green
- evidence package complete

### Pilot Ready

Required:
- representative real-world workflows
- recovery/failure tests
- backup/restore drills appropriate to pilot scope
- support/incident procedure
- release/install/update path
- instrumentation/observability appropriate to the pilot

### Commercial Ready

Required:
- approved pricing/packaging
- licensing/entitlement behavior appropriate to the chosen model
- website acquisition/download lifecycle
- support entry points
- legal/regulatory evidence for the launch scope
- release and rollback procedure

## Timeline policy

The project should track target dates or durations only after scope and capacity are known. A date written without an owner, dependency, acceptance criteria and current capacity is a hypothesis, not a commitment.

## Product expansion rule

After MVP, expand in this order unless evidence shows otherwise:

```text
Prove first market
    ↓
Strengthen shared core
    ↓
Add highest-value capability/module
    ↓
Validate with real users
    ↓
Expand industry/geography
    ↓
Add mobile companion and broader distribution
```

Never expand the roadmap merely because an industry or integration sounds attractive.

## Decision authority

Product, regulatory, pricing, launch-market and competitive decisions require explicit human approval. An implementation agent may analyze options and provide evidence, but may not invent the final decision.
