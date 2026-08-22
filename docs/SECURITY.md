# Security Baseline

## Secrets
Never commit Supabase secret/service-role keys, database passwords, license private keys, updater private keys or signing passwords. Public publishable keys may be client-visible but are never authorization by themselves.

## Authorization
RLS protects cloud tenant data. Rust/domain services protect local POS operations. UI checks are convenience only.

## Financial actions
Sales, refunds, voids, cash adjustments, debt write-offs, inventory adjustments and user/permission changes require explicit permissions and audit events.

## Tauri
Use least-privilege capabilities, a restrictive CSP, explicit IPC commands and no arbitrary shell/file/network permissions unless a feature has a documented need.

## Updates
Only signed updater artifacts are accepted. License and updater signing keys are independent.

## Data
Backups must be encrypted at rest where supported, access controlled and regularly restored in a test environment. Logs must not contain passwords, tokens, payment secrets or unnecessary personal data.
