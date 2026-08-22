# DOMAIN CONTRACTS

These are cross-module invariants. Feature specifications may extend them but may not weaken them.

## Organization / Branch / Register

Every operational action executes within an organization and, where applicable, branch/register/device context. Cross-tenant access is forbidden. Register state and cash shift state are explicit.

## Product

A product has stable identity and commercial metadata. Variant/matrix, weighted, batch, expiry, serial, IMEI and warranty behavior are capabilities, not mutually exclusive global product categories.

## Inventory

Every quantity change creates a stock movement in the same atomic operation. Balance cannot become negative unless an explicitly configured business policy permits it and the policy is audited. Transfers have source and destination movements.

## Sale

A sale is a business transaction, not merely a UI cart. Completion validates authorization, pricing, taxes, quantities, stock policy, payment, customer/debt rules, then atomically persists sale lines, payments, stock movements, cash/debt effects, and an outbox event.

A retried completion with the same idempotency key must not create a second sale.

## Payment

Payment method, currency, amount, status, and reference are explicit. Split payments are supported by multiple payment records. Change is calculated from exact money. Failed/voided payments do not count as settled funds.

## Refund / Exchange

Refunds reference original sale lines and cannot exceed refundable quantities. Stock and cash/debt effects are compensating movements. Exchanges are modeled as linked sale/refund operations with clear net financial effect.

## Cash

A shift has opening state, movements, closing state, and reconciliation. Cash movements are ledger-backed. Cash drawer state is never inferred only from the UI.

## Debt

Customer debt is a ledger of charges, payments, adjustments, and reversals. A displayed balance is derived from ledger entries.

## Purchase / COGS

Receiving changes inventory atomically. Cost policy (FIFO/weighted average/other approved policy) must be explicit before profitability reporting. Historical COGS cannot change silently because a later purchase changed a current price.

## Loyalty

Earn, redeem, reverse, expire, and adjust events are ledger-backed and idempotent. Refunds reverse applicable rewards according to the approved policy.

## Audit

Security-sensitive and financial actions produce an audit record containing actor, organization, action, target, timestamp, correlation/request identifier, and enough metadata to reconstruct the event without storing secrets.

## Sync

A synced event has stable identity and idempotency. Server acknowledgement does not mean a duplicate local operation may be replayed. Financial conflicts require explicit domain resolution.
