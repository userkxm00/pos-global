-- Migration 018: Warranty tracking index
-- Implements ADR-0011 (F2.09 Lightweight Core)

CREATE INDEX IF NOT EXISTS idx_serial_numbers_warranty_expires
ON serial_numbers(warranty_expires_at)
WHERE warranty_expires_at IS NOT NULL;
