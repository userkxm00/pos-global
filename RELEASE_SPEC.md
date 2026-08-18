# RELEASE SPECIFICATION

## Environments

`local → CI → staging/dev → internal beta → private beta → production`.

Production credentials and signing keys are never reused in development.

## Versioning

Use Semantic Versioning for application releases. Database schema versions are migration identifiers and are independent from app versions.

## Release gates

Before production release:

- CI green;
- dependency/security review complete;
- migrations tested from supported upgrade versions;
- signed artifacts generated;
- installer artifacts inspected;
- updater metadata/signatures verified;
- release notes prepared;
- rollback/recovery plan ready;
- critical E2E flows pass.

## Auto update

Tauri updater artifacts are cryptographically signed. Update signing keys are isolated from license signing keys. Updates are downloaded/installed only after signature verification and only at a safe application state.

Never restart while a financial transaction, printing operation, or other non-interruptible workflow is in progress.

## Database compatibility

Every app release that changes the schema must define migration direction, backup behavior, compatibility expectations, and recovery procedure. An updater must not install an application that cannot safely open the user's database.

## Staged rollout

Release to internal/beta users before production. Monitor errors and rollback when a release creates unacceptable regressions.

## Rollback

Rollback must account for both binary version and database migration compatibility. Never promise binary rollback if the newer migration is irreversible without a documented recovery path.

## Secrets

GitHub Actions secrets are used for private signing material. No signing private key is committed. Production release permissions should be limited to protected branches/tags/environments.
