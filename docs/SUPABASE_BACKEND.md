# Supabase Backend Contract

## Development role
The current Supabase project is the development/staging backend. Production must use a separate project and separate secrets.

## Supabase responsibilities
- Auth identity and sessions
- Cloud PostgreSQL data
- Row Level Security
- Storage where required
- Sync coordination and server-side validation
- License metadata/entitlements through protected tables/functions

## Desktop responsibilities
- Offline SQLite operations
- POS transaction execution
- Local authorization and session policy
- Hardware access through Tauri/Rust
- Outbox creation and sync retry

## RLS
Every cloud table containing organization/branch data must have RLS enabled and policies that derive access from authenticated organization/branch membership. A policy must never trust a client-provided organization id alone.

## Secrets
Only the publishable client key is allowed in the desktop client. Secret/service-role keys stay server-side and in protected deployment secrets.

## Environments
Development/staging and production are separate Supabase projects. Migrations are versioned in Git and applied deliberately; production data is never used as a test database.
