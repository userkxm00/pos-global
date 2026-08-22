# AGGREGATE BEHAVIOR EXAMPLES — Zylo

These examples clarify observable behavior for major domain aggregates. They are examples of repository contracts, not alternate API specifications. The current approved contracts remain authoritative.

## Sale — happy path

Given an open shift owned by the operator and enough stock:

`request(idempotency_key=K, item=P, quantity_milli=1000, unit_price_minor=500)`

Expected:

- one sale is created;
- total is exactly `500` minor units;
- stock decreases by exactly `1000` quantity-milli;
- one stock movement is recorded;
- one payment/result record is created when payment is part of the command;
- one outbox event is created;
- retrying the same key returns the same logical sale/result without duplicate side effects.

## Sale — rejection paths

- closed shift → reject, no sale, no stock change;
- shift owned by another operator → reject, no side effects;
- insufficient stock → reject and roll back all transaction side effects;
- same idempotency key with incompatible request → deterministic conflict/error;
- negative/invalid quantity → reject before mutation.

## Customer optionality

A normal point-of-sale sale may omit a customer when the business policy allows anonymous sales. Customer debt/credit behavior must only occur when the selected payment/credit workflow explicitly requires it.

## Return

A return is a compensating business transaction. It must preserve original history, reference the original sale/lines where applicable, restore or otherwise adjust stock according to the approved return policy, and never delete the original sale.

## Inventory adjustment

An adjustment requires an explicit reason and creates traceable stock history. Direct mutation of on-hand quantity without a corresponding ledger/movement record is not allowed.

## Authentication

A failed login/PIN attempt must not reveal whether another sensitive account exists. Authorization decisions remain enforced at the trusted Rust/service boundary rather than relying on UI state.

## Sync duplicate delivery

If an outbox event is delivered more than once, the receiving operation must use its idempotency/identity contract so that applying the same event twice does not duplicate financial or stock side effects.

## Payment provider lifecycle

A provider-neutral payment flow should distinguish at least:

`initiated → pending/verified → captured/succeeded → failed/cancelled → refunded/reversed`

The exact provider contract is decided by the payment/provider ADR. Never invent provider semantics when the provider has not been selected.

## License/update

License verification and application updates are separately signed trust domains. A valid application update must not be accepted merely because a license artifact is valid, and vice versa.

## Evidence rule

For each aggregate implemented by an agent, the task evidence should identify the applicable happy path, rejection path, persistence/side-effect assertions, and any contract cases that remain intentionally unimplemented.
