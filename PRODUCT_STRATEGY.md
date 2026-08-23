# POS Global — Product Strategy & MVP Gate

## Product direction — global by design

POS Global remains a **global, multi-industry POS platform**. The MVP is only the first commercial validation scope; it is not a permanent limitation of the product, database, architecture, or roadmap.

The long-term product uses shared core contracts plus **industry presets + capabilities + domain modules**. The implementation agent must never hard-code the MVP vertical into the core in a way that blocks other industries or mixed stores.

## Current approved MVP validation scope

**Initial validation vertical: Fashion / Clothing / Footwear / Accessories.**

This scope is deliberately close to a concrete retail workflow so the team can validate the shared POS core using real scenarios such as size/color variants, variant matrices, SKU/barcode management, variant-level inventory, prices, discounts, returns/exchanges, customers, branches, shifts and operational reports.

This does **not** turn POS Global into a fashion-only product.

### Explicit boundary

Do not replace the generic product model with fashion-specific core fields, use `product_type = clothing` or equivalent as the main industry architecture, remove future industries, or design identity, permissions, inventory, sales, financial, sync or API contracts as fashion-only.

Use `industry preset + capabilities + domain module`, consistent with `V2_RULES.md`.

## Remaining product decisions

These must be explicitly approved in an ADR or dated product decision before broader commercial expansion:

1. Initial launch market and regulatory scope.
2. Initial ICP / merchant segment.
3. Primary reason-to-buy versus realistic alternatives.
4. Final MVP capability set.
5. Explicit MVP non-goals.
6. Business model.
7. Pricing hypothesis.
8. MVP/pilot milestone and acceptance criteria.

Agents may analyze options but may not invent these decisions.

## MVP principles

### Core before breadth

The MVP reuses the long-term shared foundation: exact money, tenant boundaries, branch/device identity, Rust authorization, stock ledger, idempotency, offline-first transaction safety, explicit sync contracts, auditability and i18n/RTL readiness.

### Narrow validation, global architecture

Only the first validation scope is narrow. The architecture remains global and modular from day one.

### Mobile is future-compatible, not automatically MVP

Desktop remains the primary POS execution surface unless explicitly changed. Shared contracts, sync events, identity, permissions, financial invariants, device identity and versioning must remain consumable by future Android/iOS clients.

### Integrations are boundary-first

Define stable integration boundaries before selecting providers. Do not add a provider dependency without a documented reason, alternatives review, security/license review, platform impact review and tests.

## MVP customer workflow

```text
Create business
    ↓
Choose Fashion / Clothing / Footwear / Accessories preset
    ↓
Configure branch + currency + language
    ↓
Create first user / role
    ↓
Import or add products
    ↓
Configure size/color variants where applicable
    ↓
Set opening stock
    ↓
Open shift / selling session
    ↓
Complete first sale
    ↓
View confirmation / basic report
```

The onboarding path is a product requirement, not an accidental result of implementation.

## Data migration

MVP should provide a safe import path for a typical legacy retail workflow, with validation, preview, error reporting and a safe failure path. Target data includes products/SKU/barcode, variants, customers, opening stock, prices and suppliers where required.

## Global readiness without global overbuild

The architecture should be ready for localization/RTL, locale-aware dates/numbers/currencies, multiple currencies, timezones, jurisdiction-specific tax/compliance adapters, versioned contracts and sync. Jurisdiction-specific behavior remains evidence-driven and gated by the commercial/regulatory plan.

## Competitive positioning

Before launch, maintain a lightweight competitive record for the selected market. It must answer: **Why should the first merchant choose POS Global instead of the most realistic alternative?** The answer must influence MVP scope, onboarding, pricing and messaging.

## Milestones

### MVP Ready
- approved MVP scope, market and ICP
- core fashion validation workflow implemented
- financial/security/inventory invariants tested
- onboarding usable
- import path tested when in scope
- CI/security green
- evidence package complete

### Pilot Ready
- representative real-world workflows
- recovery/failure tests
- backup/restore drills appropriate to pilot scope
- support/incident procedure
- release/install/update path
- required observability

### Commercial Ready
- approved pricing/packaging
- appropriate licensing/entitlement behavior
- website acquisition/download lifecycle
- support entry points
- launch regulatory evidence
- release and rollback procedure

## Timeline policy

Use milestone gates rather than invented calendar promises. Dates become commitments only after scope, dependencies, owner and capacity are known.

## Product expansion rule

```text
Validate Fashion / Clothing / Footwear / Accessories
    ↓
Strengthen shared core
    ↓
Add highest-value capability/module
    ↓
Validate with real users
    ↓
Expand industries/geographies
    ↓
Add mobile companion and broader distribution
```

The Phase 7 industry roadmap remains the long-term global product roadmap. It is not MVP scope.
