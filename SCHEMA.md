# Canonical Database Schema

## Tenancy
`organizations/businesses → branches → registers/devices` is the tenant boundary. Every operational row must be attributable to an organization/business and, where relevant, a branch/register/device.

## Identity
`users`, `permissions`, `role_permissions`, `user_permissions`, `local_sessions` map Supabase identity to local POS authorization. Supabase user id is an external identity reference, not a replacement for local operational authorization.

## Product
A product is a commercial definition. Variant combinations are represented separately. Capabilities determine whether the product needs barcode, matrix, size, color, weight, batch, expiry, serial/IMEI, warranty, dimensions or other behavior.

## Inventory
Use `stock_movements` as the audit ledger. A current stock balance is a derived/cacheable state. Movement reasons include sale, refund, purchase receipt, adjustment, transfer, damage, loss and opening balance.

## Sales
A sale contains immutable business identity, branch/register/user context, lines, pricing/tax/discount totals, payments and references to inventory/cash/customer operations. Financial fields use integer minor units as authoritative values.

## Cash
Shifts represent a register session. Opening balance, cash-in/out, payments, refunds and closing reconciliation must be ledger-backed and auditable.

## Purchasing
Suppliers, purchase orders, receiving, supplier balances and purchase costs must be independent from sales while sharing the inventory/cost ledger.

## Offline sync
`outbox_events` records a durable event in the same local transaction as the business mutation. `idempotency_keys` prevents replay from creating duplicate operations.

## Industry model
`industry_presets` are configuration bundles. `capabilities` are reusable product/business/transaction behaviors. `modules` are larger workflows such as Restaurant, Rental or Service. A business may enable several modules and a product may enable several capabilities.

## Migration policy
Migrations are append-only. Never edit an applied migration. Schema changes require a new migration plus migration tests and a compatibility note.
