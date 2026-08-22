# Security Policy

## Scope

This project is a foundation-stage desktop POS and business-management platform. Security reports affecting authentication, authorization, tenant isolation, local data integrity, synchronization, licensing, updates, secrets, or payment/billing boundaries are considered high priority.

## Reporting

Do not disclose a suspected vulnerability publicly before remediation. Use a private GitHub security advisory/reporting channel when available for the repository. If private reporting is not enabled yet, contact the repository owner privately and include reproduction steps, affected commit/version, impact, and any relevant logs without attaching live credentials.

## Secrets

Never include live API keys, passwords, service-role keys, updater private keys, signing keys, or billing secrets in an issue or pull request. If a secret has been exposed, rotate/revoke it immediately and then report the incident.

## Security guarantees

The project does not claim production security or regulatory compliance until the applicable security, privacy, dependency, E2E, release-signing, and jurisdiction gates are verified.
