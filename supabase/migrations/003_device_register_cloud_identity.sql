-- Supabase Migration: 003_device_register_cloud_identity.sql
-- F1.21 — Device / Register Cloud Identity
-- Append-only migration: adds device identifier columns, pairing lifecycle state,
-- domain check constraints, pairing coherence rules, active device uniqueness indexes,
-- and supporting performance indexes to public.registers.
--
-- INVARIANTS: 001_phase1_identity_and_rls.sql and 002_organization_branch_member_schema.sql
-- are immutable and untouched.

-- ============================================================
-- 1. Device Identity Columns on public.registers
-- ============================================================

ALTER TABLE public.registers
    ADD COLUMN IF NOT EXISTS device_identifier TEXT;

ALTER TABLE public.registers
    ADD COLUMN IF NOT EXISTS device_pairing_status TEXT NOT NULL DEFAULT 'unpaired';

ALTER TABLE public.registers
    ADD COLUMN IF NOT EXISTS device_paired_at TIMESTAMPTZ;

ALTER TABLE public.registers
    ADD COLUMN IF NOT EXISTS device_last_seen_at TIMESTAMPTZ;

-- ============================================================
-- 2. Domain Check Constraints on public.registers
-- ============================================================

-- Name validation: non-empty, non-whitespace, max 255 characters
ALTER TABLE public.registers
    DROP CONSTRAINT IF EXISTS chk_registers_name;

ALTER TABLE public.registers
    ADD CONSTRAINT chk_registers_name
    CHECK (length(trim(name)) > 0 AND length(name) <= 255);

-- Code validation: non-empty, alphanumeric with dashes/underscores/dots, 1-50 characters
ALTER TABLE public.registers
    DROP CONSTRAINT IF EXISTS chk_registers_code;

ALTER TABLE public.registers
    ADD CONSTRAINT chk_registers_code
    CHECK (length(trim(code)) > 0 AND length(code) <= 50 AND code ~ '^[a-zA-Z0-9_.-]+$');

-- Device Identifier validation: optional, 3-128 characters, alphanumeric with safe delimiters
ALTER TABLE public.registers
    DROP CONSTRAINT IF EXISTS chk_registers_device_identifier;

ALTER TABLE public.registers
    ADD CONSTRAINT chk_registers_device_identifier
    CHECK (device_identifier IS NULL
           OR (length(trim(device_identifier)) >= 3
               AND length(device_identifier) <= 128
               AND device_identifier ~ '^[a-zA-Z0-9_.:-]+$'));

-- Device Pairing Status validation: finite state machine domain
ALTER TABLE public.registers
    DROP CONSTRAINT IF EXISTS chk_registers_pairing_status;

ALTER TABLE public.registers
    ADD CONSTRAINT chk_registers_pairing_status
    CHECK (device_pairing_status IN ('unpaired', 'paired', 'revoked'));

-- Pairing Coherence validation:
-- - 'paired' requires a non-null device_identifier
-- - 'unpaired' requires device_identifier to be null
-- - 'revoked' allows preserving historical device_identifier for audit
ALTER TABLE public.registers
    DROP CONSTRAINT IF EXISTS chk_registers_pairing_coherence;

ALTER TABLE public.registers
    ADD CONSTRAINT chk_registers_pairing_coherence
    CHECK (
        (device_pairing_status = 'paired' AND device_identifier IS NOT NULL)
        OR (device_pairing_status = 'unpaired' AND device_identifier IS NULL)
        OR (device_pairing_status = 'revoked')
    );

-- ============================================================
-- 3. Device Identity Uniqueness Rules
-- ============================================================

-- A physical device cannot be paired to multiple registers in the same organization simultaneously
DROP INDEX IF EXISTS public.uq_registers_org_device;

CREATE UNIQUE INDEX uq_registers_org_device
    ON public.registers (organization_id, device_identifier)
    WHERE device_identifier IS NOT NULL AND device_pairing_status = 'paired';

-- A physical device cannot be actively paired across different organizations simultaneously
DROP INDEX IF EXISTS public.uq_registers_global_active_device;

CREATE UNIQUE INDEX uq_registers_global_active_device
    ON public.registers (device_identifier)
    WHERE device_identifier IS NOT NULL AND device_pairing_status = 'paired';

-- ============================================================
-- 4. Supporting Performance Indexes
-- ============================================================

-- Fast lookup by hardware device identifier
CREATE INDEX IF NOT EXISTS idx_registers_device_id
    ON public.registers (device_identifier)
    WHERE device_identifier IS NOT NULL;

-- Fast lookup of active registers within a branch
CREATE INDEX IF NOT EXISTS idx_registers_branch_active
    ON public.registers (branch_id, is_active);

-- Fast lookup of pairing lifecycle states within an organization
CREATE INDEX IF NOT EXISTS idx_registers_pairing_status
    ON public.registers (organization_id, device_pairing_status);
