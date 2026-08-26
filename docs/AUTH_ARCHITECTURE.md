# Authentication and Authorization Architecture

## Decision
Use **Supabase Auth for online identity** and a **local Rust/domain authorization layer for POS operations**. Do not build a general-purpose identity provider from scratch.

## Supabase owns
- Account creation and sign-in
- Email verification and password recovery
- MFA/OTP where enabled
- Cloud session/JWT lifecycle
- Owner/admin cloud identity

## POS owns
- Local users linked to Supabase identities
- Roles and permissions
- Cashier PIN/session for fast POS operation
- Offline session rules
- Shift/register authorization
- Sensitive action authorization

## Security boundary
The React UI can request an action but cannot authorize it. Rust/domain services validate the authenticated local user, branch/register context and permission before changing money, stock, cash, users or configuration.

## Offline mode
When cloud identity cannot be refreshed, an existing approved local session may continue only within explicit device/session policy. Passwords and Supabase service keys are never stored in the desktop application as recoverable secrets.

## Supabase RLS
Every tenant-owned cloud table requires RLS and policies based on organization/branch membership. The publishable client key is not an authorization bypass; RLS is mandatory.

## Never ship
- `SUPABASE_SECRET_KEY`
- `service_role` keys
- database passwords
- JWT signing secrets
- license private keys
- updater private signing keys

## Context Switcher Boundary (F1.15)
The Organization / Branch / Register context switcher in the client shell orchestrates UI context presentation only. Active operational context is constrained strictly to existing entities returned by authoritative domain commands (`list_organizations`, `list_branches`, `list_registers`).
