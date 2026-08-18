# Security Scan Policy

## Foundation requirement

Security checks must produce visible evidence. A workflow may report findings, but it must not silently suppress high or critical findings.

## Dependency review

The project uses the package manager's audit facilities and Rust dependency auditing where available. Findings are classified as:

- **Blocker:** exploitable critical/high issue in production-relevant code with no approved mitigation.
- **Review:** medium/low issue, development-only issue, or issue mitigated by configuration/usage.
- **Accepted:** documented risk with owner, rationale, and review date.

`npm audit` findings must be recorded with package, severity, advisory, dependency path, exposure (runtime/dev), and disposition.

## Secrets

No secret belongs in source control, frontend bundles, SQLite fixtures, migrations, documentation examples, or CI logs.

Public Supabase publishable/client keys may be present only where the architecture explicitly permits them; service-role keys, database passwords, updater private keys, billing secrets, signing keys, and OAuth client secrets must remain in protected secret storage.

GitHub Secret Protection/secret scanning should be enabled for the repository when the account/repository plan supports it. The CI baseline also performs deterministic local checks for obvious private-key and credential patterns so the foundation does not depend on a GitHub UI setting.

## CI behavior

Security reporting may continue after a non-blocking audit step, but the final Foundation Gate must fail if an unresolved blocker exists. A green build must never be achieved by deleting, ignoring, or weakening the security check.
