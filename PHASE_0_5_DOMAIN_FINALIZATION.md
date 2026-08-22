# POS Global — Phase 0.5: Domain Finalization

Status: REQUIRED BEFORE AUTONOMOUS IMPLEMENTATION
Owner: Product + Architecture
Purpose: freeze the business rules that must not be invented by an implementation agent.

## 0. Mission

Phase 0.5 converts architectural principles into explicit domain contracts. The implementation agent may implement these contracts, but may not silently change financial, inventory, transaction, synchronization, authorization, or licensing semantics.

If a rule is marked `DECISION REQUIRED`, the agent must stop before implementing the affected task and create a clarification/ADR rather than guessing.

## 1. Non-negotiable invariants

- Money is exact; no floating point is authoritative financial truth.
- Every stock mutation creates an immutable movement/ledger record.
- Sales, refunds, exchanges, purchases and cash operations are atomic at the local transaction boundary.
- Retryable commands carry an idempotency key and must be safe to repeat.
- Historical financial truth is corrected with compensating transactions, never by mutating history.
- Offline local selling remains possible when cloud services are unavailable.
- Tenant isolation is enforced server-side with RLS and locally with Rust authorization.
- A product category/industry never changes the meaning of shared financial primitives.

## 2. Money and rounding contract

### 2.1 Storage

Use integer minor units for currencies whose minor-unit model is supported, plus explicit currency metadata. Do not use `f32`/`f64` as authoritative values.

Required concepts:
- currency code
- minor-unit scale
- amount_minor
- rounding mode
- calculation precision
- display precision

### 2.2 Calculation order

The pricing engine must define and test a deterministic order. Default contract:

1. Resolve base price.
2. Resolve quantity/unit conversion.
3. Apply eligible line-level discounts.
4. Compute taxable base according to jurisdiction rules.
5. Calculate tax using the configured tax policy and rounding mode.
6. Apply transaction-level adjustments where permitted.
7. Produce the final payable total.
8. Persist the calculation inputs and resulting amounts on the transaction.

The exact order for jurisdiction-specific tax/discount combinations must be encoded in the tax/promotion policy, not invented by the UI.

### 2.3 Rounding

Rounding must be explicit, deterministic and tested at line, tax and transaction levels. Never recompute historical totals from today's configuration.

## 3. Quantity and unit contract

Products may be sold by:
- discrete count
- weight
- volume
- length/area where applicable
- service quantity

Each stock-affecting quantity must have a canonical inventory unit. Sales/purchase units may convert to that canonical unit through a versioned conversion rule.

Rules:
- conversions must be deterministic;
- precision must be explicit;
- stock cannot silently become fractional when the product policy forbids it;
- weighted products require scale/manual-entry validation rules;
- unit conversion changes must not rewrite historical transactions.

## 4. Inventory costing contract

### Decision

The architecture must support multiple costing strategies through a costing service interface.

Initial default for general retail: **Weighted Average Cost**, subject to final accounting/legal validation for each launch jurisdiction. The data model must not make FIFO impossible.

Supported strategy targets:
- weighted average
- FIFO
- specific identification for serialized/high-value goods

### Rules

- Cost at sale time comes from the costing ledger/state valid for that transaction, not from the product's current cost field.
- Returns reverse the cost impact of the referenced sale according to the original costing record.
- Exchanges create linked compensating/fulfilling transactions.
- Serialized items use specific identity where applicable.
- Inventory valuation must reconcile to stock ledger and costing state.
- Cost changes never rewrite historical COGS.

## 5. Tax engine contract

Tax is a versioned jurisdiction service, not a hardcoded global percentage.

Tax rule inputs may include:
- jurisdiction
- tax registration status
- customer type
- product/category tax classification
- transaction type
- place of supply
- effective-from/effective-to
- tax-inclusive/exclusive mode
- exemption/reverse-charge status

Tax rules must be immutable once used by a posted transaction. New rates/rules are new versions.

### Launch research baseline

Algeria official DGI guidance currently states a normal VAT rate of 19% and a reduced rate of 9% for the real/simplified regimes, and describes different taxable events for goods and services. This is source material for the Algeria adapter, not a universal global rule.

France/EU must be modeled as a jurisdiction family with country-specific rates/rules. The European Commission states that EU VAT rates are governed by a common framework while Member States set their rates and categories within that framework.

The agent must not hardcode these values into generic POS logic. Rates and rules belong to jurisdiction configuration/data with effective dates and source metadata.

## 6. Pricing and promotion contract

Pricing must be resolved by a deterministic priority model.

Concepts:
- base price
- price list
- customer group price
- branch price
- quantity tier
- promotion
- coupon/discount
- tax treatment

Promotions must declare eligibility, stacking rules, priority and effective dates. The engine must return an explainable calculation result so the receipt and audit log can explain why a final price was produced.

## 7. Payment abstraction contract

The POS core must never depend directly on one online payment provider.

Payment methods:
- cash
- card terminal
- bank transfer
- manual/other
- customer credit/debt
- online payment where enabled

Required abstraction:

`PaymentMethod -> PaymentProcessorAdapter -> Provider/Hardware`

A provider adapter may be online or terminal-based. A provider timeout after an external commit must be recoverable using provider reference + idempotency/reconciliation; it must never create a second local sale.

Provider selection is a commercial decision, not a core-domain decision.

## 8. Refund and exchange contract

Refunds reference the original sale and lines where possible.

A refund must:
- authorize the action;
- create a compensating financial transaction;
- create stock return movements when goods are returned;
- reverse/adjust COGS according to the original cost record;
- reverse applicable loyalty effects according to the loyalty policy;
- preserve the original sale unchanged;
- be auditable.

An exchange is a linked return + fulfillment flow with explicit net payment/refund.

## 9. Cash contract

A register shift contains:
- opening balance
- cash movements
- sales cash receipts
- refunds
- paid-in/paid-out
- safe drops where supported
- expected closing balance
- counted closing balance
- variance

Cash balance is ledger-derived. Never silently overwrite the balance to hide a variance.

## 10. Customer debt contract

Debt is a ledger, not a mutable balance field.

Events include:
- credit sale
- debt payment
- refund adjustment
- manual authorized adjustment
- write-off where legally/business-policy permitted

Every balance must reconcile to the ledger.

## 11. Loyalty contract

Loyalty is a ledger with:
- earn
- redeem
- reverse
- expire
- authorized adjustment

A refund/exchange must define whether earned points are reversed. Historical point events remain auditable.

## 12. Sync conflict contract

Do not use one global conflict strategy.

| Aggregate | Default strategy |
|---|---|
| Sale | immutable event; no merge |
| Refund | immutable compensating event |
| Cash movement | immutable ledger |
| Debt movement | immutable ledger |
| Loyalty movement | immutable ledger |
| Inventory movement | immutable ledger + deterministic reconciliation |
| Product metadata | version/field-aware merge policy |
| Product price | version conflict; require explicit resolution where concurrent edits matter |
| Customer profile | field/version-aware merge |
| Configuration | versioned last-writer policy only where safe |

Financial and stock truth must never be resolved by naive last-write-wins.

## 13. Industry capability contract

Industry is a preset for capabilities/modules. Product type is per-product behavior. Shared primitives remain stable.

Capabilities include, as applicable:
- matrix variants
- weighted selling
- batches/expiry
- serial/IMEI
- warranty
- recipes/BOM
- tables/orders/KDS
- service tickets/labor
- rental assets/contracts
- B2B pricing/credit
- reservations/events
- appointments
- manufacturing-lite/food production

An industry preset may enable defaults, but a business may override allowed capabilities according to its plan and jurisdiction.

## 14. Hardware contract

Hardware is behind stable interfaces. Initial interfaces:
- barcode scanner
- receipt printer
- cash drawer
- weighing scale
- label printer
- customer display

The POS transaction must not be committed twice because a printer/scanner/device operation failed. Hardware failure after a committed sale is a fulfillment/recovery problem, not a reason to create a second sale.

## 15. License/entitlement contract

Separate:
- identity
- subscription/billing
- license
- entitlement
- device activation

The desktop app validates a signed license/entitlement locally within an offline grace policy. Server-side revocation/plan changes are synchronized when online.

License signing key and updater signing key are distinct trust roots.

## 16. Decision protocol

When a task requires a new financial, regulatory, security, schema, sync, provider, or licensing decision:

1. Stop implementation.
2. Identify the decision and affected invariants.
3. Research authoritative sources where the issue is jurisdiction/provider dependent.
4. Create/update an ADR.
5. Update the relevant contract/spec.
6. Add acceptance tests.
7. Only then implement.

## 17. Exit gate

Phase 0.5 is complete only when:
- money rules are frozen;
- quantity/unit rules are frozen;
- costing strategy interface + initial default are frozen;
- tax engine contract is frozen;
- pricing/promotion semantics are frozen;
- payment abstraction is frozen;
- refund/exchange semantics are frozen;
- cash/debt/loyalty ledger semantics are frozen;
- sync conflict matrix exists;
- hardware abstraction exists;
- industry capability taxonomy is frozen;
- license/entitlement boundary is frozen;
- all unresolved decisions are explicitly listed as `DECISION REQUIRED` rather than silently guessed.
