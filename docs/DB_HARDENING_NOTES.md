# Database hardening notes

The applied migrations are append-only. Migration 008 consolidates legacy duplicate non-variant inventory rows before enforcing the intended inventory identity with partial unique indexes. Migration 009 removes the redundant product barcode index because the UNIQUE(barcode) constraint already supplies the lookup index.

Organization ownership remains intentionally nullable until the organization/auth phases define the authoritative tenant assignment rules.
