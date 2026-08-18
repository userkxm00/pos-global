# SYNC SPECIFICATION

## Goal
Synchronize offline desktop operations with cloud services without duplicate financial transactions or silent data loss.

## Event identity
Every outbox event has:

- event_id
- organization_id
- branch_id
- device_id
- aggregate_type
- aggregate_id
- event_type
- schema_version
- idempotency_key
- occurred_at
- created_at
- payload
- status
- attempt_count
- last_error

## Transactional outbox
The business mutation and its outbox record are created in one local transaction. A committed business operation without a corresponding retryable event is a defect.

## Idempotency
Cloud application of an event must be safe to retry. A stable unique idempotency key prevents duplicate application.

## Ordering
Ordering is enforced only where the domain requires it. Per-aggregate sequence/version may be used. Independent aggregates should not be artificially serialized.

## Retry
Use bounded exponential backoff with jitter, classify permanent versus transient errors, and preserve failed events for recovery/diagnostics.

## Conflict policy
- Financial transactions: never last-write-wins.
- Stock movements: append/resolve by domain rules; never silently overwrite history.
- Product metadata: versioned conflict policy may be last-writer or explicit merge where approved.
- Pricing/configuration: versioned with explicit precedence.
- Customer profile: field-level or versioned merge where safe.

## Offline behavior
The POS must continue approved local workflows while disconnected. The UI must clearly show offline/sync state without blocking safe local transactions.

## Recovery
A restart must resume unsent events. Corrupt or permanently rejected events are quarantined with actionable diagnostics; they are not deleted silently.

## ACK semantics
An ACK means the cloud accepted the event identity and business application. The client records the ACK idempotently. Replaying an ACKed event must be harmless.

## Observability
Expose last successful sync, pending count, failed count, last error category, device identity, and correlation IDs to authorized operators/admins without exposing secrets.

## Testing matrix
Test disconnect, reconnect, duplicate send, timeout after server commit, out-of-order events, concurrent devices, rejected event, restart during sync, database backup/restore, and clock skew.
