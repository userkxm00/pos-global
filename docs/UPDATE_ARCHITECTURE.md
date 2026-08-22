# Auto Update Architecture

## Flow
GitHub Actions → build → sign artifacts → GitHub Release → signed update metadata → desktop checks → verifies signature → downloads → installs at a safe point → restarts.

## Rules
- Updates are cryptographically signed.
- The updater private key never enters source control or the application bundle.
- Update signing and license signing use different key pairs.
- The app must not restart while a critical POS transaction is active.
- Offline clients continue operating normally and check again when connectivity returns.
- Update UI supports later/ignore where policy permits.
- Release artifacts must be reproducible enough to identify the exact source commit and release version.

## Failure handling
A failed download or verification does not alter the installed application. A failed installation must leave the previous working version intact according to the platform updater guarantees.

## Production secrets
Store updater signing private material only in protected CI secrets. Put only the public verification key and release endpoint configuration in the application.
