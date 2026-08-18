# ADR-0006 — Domain, Commercial and Regulatory Finalization

Status: Accepted for architecture; jurisdiction/provider launch approvals remain explicit gates.
Date: 2026-08-17

## Context

The project is intended to be implemented by an autonomous coding agent. Critical financial, inventory, synchronization, provider and regulatory behavior must therefore be specified before implementation rather than invented during coding.

## Decisions

1. Money uses exact integer/decimal-safe representations; floating point is never authoritative.
2. Inventory is ledger-first and historical cost is preserved.
3. Costing is exposed through a strategy interface; Weighted Average Cost is the initial general-retail default, while FIFO and specific identification remain supported targets.
4. Tax is a versioned jurisdiction engine. Tax rates are data with effective dates, not global constants.
5. POS payment providers are adapter-based and separate from Zylo's own SaaS billing.
6. Refunds/exchanges are compensating transactions linked to original transactions.
7. Cash, debt and loyalty are ledgers.
8. Financial/stock sync conflicts are not resolved with naive last-write-wins.
9. Hardware is behind stable interfaces/adapters.
10. SaaS billing uses a provider-neutral interface. Paddle is the primary candidate for Merchant-of-Record software billing, subject to final seller/entity eligibility and commercial acceptance. Stripe remains an adapter candidate where the legal merchant entity is supported.
11. Regulatory support is jurisdiction-specific. No global compliance claim is allowed without authoritative evidence.
12. Planning recommendation: validate Algeria first, then France, then broader EU expansion; the product owner must approve the launch sequence before production commitments.

## Evidence reviewed

- Algeria DGI VAT guidance, updated 2026-02-23.
- European Commission VAT rates framework.
- French official tax administration/BOFiP VAT guidance.
- Paddle supported-country and tax/Merchant-of-Record documentation.
- Stripe global business availability documentation.

## Consequences

The agent can implement stable core interfaces now. Provider-specific and jurisdiction-specific adapters are implemented only after their research packages are approved. This prevents provider lock-in and prevents accidental legal claims.

## Revisit triggers

Revisit this ADR when:
- the legal company/entity changes;
- launch countries change;
- a billing/payment provider is selected;
- a new regulated industry is enabled;
- accounting/tax policy changes;
- sync semantics change.
