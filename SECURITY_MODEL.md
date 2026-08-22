# SECURITY MODEL

## Security boundaries

1. React UI is untrusted presentation code.
2. Tauri/Rust is the local privileged boundary.
3. SQLite is protected local state, not a security authority by itself.
4. Supabase Auth provides cloud identity.
5. RLS provides cloud tenant isolation.
6. License verification and update verification use separate signing keys.

## Threats and required controls

### Credential leakage
Control: publishable client key only in desktop app; secrets/service-role keys server-side only; `.env` ignored; CI secrets used for private material.

### Offline PIN brute force
Control: rate limiting/backoff, secure local secret derivation/storage, lockout policy, audit events, and recovery path.

### Modified client / UI bypass
Control: privileged commands enforce authorization in Rust; server RLS independently protects cloud data.

### Local database tampering
Control: integrity checks, signed/validated application state where appropriate, audit trails, safe recovery/backup, and never trusting client-calculated financial totals.

### License forgery/replay
Control: public-key signatures, device binding/entitlement, expiry/grace, nonce or unique license identity, revocation, clock-tamper handling, audit.

### Malicious update
Control: Tauri update signatures, isolated private update key, release permissions, protected GitHub secrets, staged releases, rollback/recovery.

### Replay / duplicate transaction
Control: idempotency key + unique database constraint + transactional outbox.

### Sync conflict
Control: explicit versioning and domain conflict policies; never blindly use last-write-wins for financial truth.

### Privilege escalation
Control: centralized permission definitions, role/permission tests, deny-by-default for privileged operations, branch/organization scope checks.

### Data exposure
Control: RLS, minimal data access, server-side authorization, audit, no sensitive data in logs, secure transport.

## Secret classes

Never commit:

- Supabase secret/service-role keys
- database passwords
- JWT signing secrets
- license private signing keys
- updater private signing keys
- platform certificates/private keys
- payment provider secret keys

## Logging

Never log passwords, tokens, private keys, full payment credentials, or unnecessary personal data. Use correlation IDs and structured safe metadata.

## Security gate

Any authentication, authorization, licensing, sync, payment, update-signing, or secret-management change requires security tests before completion.
