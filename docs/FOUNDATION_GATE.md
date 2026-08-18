# Foundation Gate

This gate must be closed before merging the foundation into `main` or allowing feature work to proceed without review.

## 1. Tenancy and identity
- Organization, branch, register and device boundaries are explicit.
- Supabase Auth owns online identity.
- Local user/session records support POS operation and offline mode.
- Roles and permissions are server-enforced and locally enforced where required.
- RLS isolates every tenant-owned cloud row.

## 2. Financial integrity
- Monetary values use integer minor units or an explicitly exact decimal representation.
- Currency is explicit on financial records.
- No `f64`/floating point is used as financial truth.
- Sales, payments, refunds, cash movements and debts are auditable and transactional.

## 3. Inventory integrity
- Stock changes are represented as immutable movements.
- Sale, refund, purchase receipt, adjustment and transfer have defined movement types.
- Negative stock policy is explicit per business configuration.
- Matrix, weighted, batch, expiry, serial/IMEI and warranty capabilities are composable.

## 4. Offline and sync
- Local transactions commit without network access.
- Outbox records are created in the same transaction as the business mutation.
- Sync is idempotent.
- Retries are safe.
- Conflicts have explicit policies.
- No duplicate sale/payment can be produced by retry.

## 5. Security
- No secrets/private signing keys are committed.
- Supabase service/secret keys never enter the desktop client.
- Tauri capabilities are least-privilege.
- RLS is enabled for tenant data.
- Sensitive actions are permission checked in Rust/domain services.
- Audit events cover privileged financial and administrative actions.

## 6. Licensing and updates
- License signing key is separate from updater signing key.
- License verification works offline within defined entitlement/grace rules.
- Updater verifies signed artifacts before installation.
- Updates do not interrupt an active sale/transaction.
- Production signing keys exist only in protected secret stores.

## 7. Build and test evidence
- TypeScript typecheck passes.
- Rust check/test passes on the supported toolchain.
- All migrations apply cleanly to an empty database.
- Migration repeatability is tested.
- Core transaction rollback tests pass.
- CI is required for protected branches.

## Merge rule
If any mandatory item above lacks implementation or evidence, the Foundation Gate is **not closed**.
