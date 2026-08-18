# E2E GOLDEN FLOWS

These workflows are release-blocking when applicable.

## G1 — First setup
Create account → organization → business preset → branch → register → user → permissions → product → ready.

## G2 — Normal cash sale
Login → open shift → scan product → cart → exact total → cash payment → receipt → stock decrement → cash movement → audit.

## G3 — Split payment
Cart → split tender → verify remaining amount → complete → each payment recorded → sale settled once.

## G4 — Offline sale
Disconnect → sell → restart application → verify local sale → reconnect → sync → verify one cloud sale and one stock effect.

## G5 — Retry after timeout
Submit sale → simulate server acknowledgement lost → retry same idempotency key → verify exactly one sale.

## G6 — Refund
Find sale → authorize refund → refund selected quantity → reverse stock/payment/debt → audit → verify refundable balance.

## G7 — Exchange
Original sale → select return → replacement item → calculate net difference → linked refund/sale → inventory and cash reconcile.

## G8 — Purchase receiving
Create purchase → receive → stock increases → cost recorded → supplier balance updated → audit.

## G9 — Inventory count
Start count → enter actual quantity → approve difference → movement created → balance reconciles.

## G10 — Permission denial
Cashier attempts manager-only refund/stock adjustment → Rust rejects → no mutation → audit/security event where appropriate.

## G11 — License
Activate valid license → bind device → verify entitlement offline → simulate expiry/revocation → enforce grace/restriction according to policy.

## G12 — Update
Release signed update → client detects → verifies signature → downloads → waits for safe point → installs → restarts → data remains intact.

## G13 — Crash recovery
Crash during selected critical workflow → restart → verify transaction is either fully committed or fully absent according to atomicity contract; no partial financial state.
