# ARCHITECTURE.md — POS Global Platform Architecture

> **Status:** Architecture baseline / pre-production  
> **Current internal project name:** `POS Global`  
> **Brand status:** `Zylo` is not adopted until brand/domain/trademark screening is complete.  
> **Rule:** This document is the source of truth for architectural decisions. Any material change must be recorded here before implementation.

## 0. Executive Architecture

The product is designed as a **global commerce operating platform**, not as a collection of industry-specific POS screens.

```text
Desktop Application
      Tauri 2
         │
 ┌───────┴────────┐
 React/TypeScript  Rust Core
       │              │
       └── Commands ──┘
              │
        SQLite / Local DB
              │
       Outbox / Sync
              │
      Supabase / Cloud
              │
   License / Customer Platform
```

### Core principles

1. **Offline-first:** selling must not depend on an internet connection.
2. **Rust is the security boundary:** authorization and financial rules are enforced in Rust.
3. **React is presentation:** it never accesses SQLite or privileged OS APIs directly.
4. **Financial operations are atomic.**
5. **Industry presets are configuration, not hardcoded product types.**
6. **Capabilities are reusable primitives:** Matrix, Weight, Batch, Expiry, Serial, Warranty, IMEI, Dimensions, etc.
7. **Domain modules stay separated:** Retail, Restaurant, Service, Rental, Wholesale and Hospitality.
8. **History is immutable:** corrections use compensating transactions.
9. **Claims require evidence:** tests, logs or reproducible verification.
10. **Migrations are append-only.**

## 1. Technology Stack

- Tauri 2
- Rust stable
- SQLite
- React
- TypeScript
- Vite
- Supabase/Postgres for cloud identity, coordination and backend services

SQLite is the operational source of truth for the desktop terminal while offline.

Required database properties:

- foreign keys enabled
- WAL where appropriate
- busy timeout
- prepared statements
- migrations
- integrity checks
- backup/restore
- deterministic transaction boundaries

## 2. Layered Architecture

```text
Presentation
    ↓
Tauri Commands
    ↓
Application Services
    ↓
Domain Rules
    ↓
Repositories / Database
    ↓
SQLite
```

React must not execute SQL, open SQLite, decide authorization, calculate financial truth independently, or validate licenses independently.

Commands must validate request shape, establish authenticated context, enforce permissions, call application/domain logic, and return typed results.

Application services include `SaleService`, `RefundService`, `InventoryService`, `PurchaseService`, `ShiftService`, `CustomerDebtService`, `LicenseService`, and `SyncService`.

## 3. Global Commerce Model

### Industry ≠ Product Type

Never model every industry as a product enum. Use:

```text
Industry Preset
      ↓
Business Capabilities
      ↓
Product Capabilities
      ↓
Domain Modules
```

### Industry presets

The platform is designed for General Retail, Convenience/Grocery, Fashion, Shoes, Jewelry, Beauty/Cosmetics, Pharmacy/Medical Retail, Electronics, Mobile/Computers, Appliances, Furniture/Home, Hardware/Tools, Building Materials, Automotive Parts, Books/Stationery, Toys/Baby, Sports/Fitness, Pet, Agriculture/Garden, Food/Beverage Retail, Bakery, Butcher, Seafood, Produce, Frozen Food, Wholesale/Distribution, B2B Supplies, Restaurant, Fast Food, Café, Pizzeria, Juice/Ice Cream, Food Truck, Catering, Repair, Phone/Computer Repair, Auto Workshop, Appliance Repair, Tailor, Rental, Salon/Barber/Spa, Printing/Copy, Events/Tickets, Hotel/Hospitality, and Custom/Other.

These are onboarding presets only; they must not become hardcoded database product enums.

## 4. Capability Architecture

Reusable capabilities include:

- identity: SKU, barcode, brand, manufacturer, model, supplier SKU
- variants: color, size, material, style, capacity, flavor, scent, matrix
- quantity: piece, pack, set, weight, volume, length, area, custom units/conversions
- inventory: stock, reorder point, bin, batch/lot, expiry, FEFO, serial, IMEI, asset ID
- commercial: price tiers, wholesale/customer-group pricing, branch pricing, promotions, discounts, deposits, tax category
- lifecycle: warranty, condition, recall, replacement/return policy
- physical: weight, dimensions, volume, packaging/shipping dimensions
- media: multiple product images

`custom_attributes` is an escape hatch for exceptional metadata only. Core money, stock, tax, payment, serial and batch data must remain structured.

## 5. Authentication and Authorization

### Supabase Auth = online identity

Supabase handles account identity, email/password, OTP/MFA/session/JWT and cloud identity.

### Local POS security = operational authorization

Rust + SQLite handle local POS users, roles, permissions, cashier PIN, local session, offline login and shift authorization.

```text
Supabase Identity
      ↓
Local User
      ↓
Role / Permissions
      ↓
Rust Authorization
      ↓
POS operation
```

The desktop client must never contain a Supabase secret/service-role key.

## 6. Offline-first and Sync

Every retryable write has an idempotency key. Important local mutations generate outbox events inside the same transaction as the business mutation.

```text
Local transaction
  ├─ business state
  ├─ stock movement
  └─ outbox event
          ↓
       sync queue
          ↓
       cloud apply
          ↓
          ACK
```

Cloud unavailability must not block normal POS operations. Conflicts are resolved by explicit domain rules, not arbitrary last-write-wins for financial records.

## 7. Money and Financial Truth

Production financial truth must not use floating point. Amounts use integer minor units or an explicitly reviewed exact-decimal model, with currency recorded alongside amounts.

Examples:

```text
EUR 12.50 → 1250 minor units
DZD 1500  → 1500 minor units
```

Tax, discount, payment, refund, debt, cash and COGS calculations must use the same exact-money policy.

## 8. Inventory Ledger

Every stock change must have a traceable movement. Sale, refund, purchase receipt, adjustment, transfer, damage, loss and count reconciliation must be atomic with the related business operation.

A direct stock quantity update without a corresponding ledger movement is a defect unless it is part of the same atomic operation that creates the movement.

## 9. Licensing

License signing is separate from hashing. The license service uses a public-key signature scheme selected and reviewed before production. The private license signing key never ships with the application.

License concerns include activation, device limits, revocation, offline grace, replay protection, clock manipulation, entitlement/plan, device reset and audit.

## 10. Auto Update

The application uses the official Tauri updater architecture.

```text
GitHub Actions
    ↓
Build + Sign
    ↓
Release artifacts + latest.json
    ↓
Desktop checks
    ↓
Signature verification
    ↓
Download/install at safe point
```

The updater signing key is distinct from license signing and authentication secrets. Updates must never force a restart during an active financial operation.

## 11. Cloud Boundary

Supabase is the initial managed backend. It may provide Auth, Postgres/RLS, sync coordination, license metadata, customer/admin portal data and server-side functions.

The POS does not become dependent on Supabase for local selling, cash handling, inventory or hardware operation.

## 12. Hardware Boundary

Barcode scanners, thermal printers, cash drawers, scales, label printers and customer displays are accessed through a hardware abstraction layer. Device-specific logic must not leak into core sales rules.

## 13. Testing and Evidence

Required gates include:

- Rust formatting/lint/tests
- TypeScript typecheck/build
- migration tests
- transaction rollback tests
- authorization tests
- offline/reconnect tests
- sync idempotency/conflict tests
- license tamper tests
- backup/restore tests
- E2E tests for critical workflows

A feature is not declared complete without actual evidence.

## 14. Architectural Change Policy

Any material change to security, financial truth, schema, licensing, sync, cloud boundary, dependencies, or platform support requires an Architecture Decision Record and tests before merge.
