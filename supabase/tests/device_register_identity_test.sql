-- Supabase Test Suite: device_register_identity_test.sql
-- F1.21 — Device / Register Cloud Identity Verification Suite
-- Deterministic behavioral test assertions for PostgreSQL 15 / Supabase

BEGIN;

-- ============================================================
-- 1. Setup Standalone Roles and Grants if not present
-- ============================================================

DO $$
BEGIN
    IF NOT EXISTS (SELECT 1 FROM pg_roles WHERE rolname = 'authenticated') THEN
        CREATE ROLE authenticated NOLOGIN NOINHERIT;
    END IF;
    IF NOT EXISTS (SELECT 1 FROM pg_roles WHERE rolname = 'anon') THEN
        CREATE ROLE anon NOLOGIN NOINHERIT;
    END IF;
END $$;

CREATE SCHEMA IF NOT EXISTS auth;
CREATE TABLE IF NOT EXISTS auth.users (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    email TEXT UNIQUE,
    created_at TIMESTAMPTZ DEFAULT now()
);

CREATE OR REPLACE FUNCTION auth.uid()
RETURNS UUID
LANGUAGE sql
STABLE
AS $$
    SELECT NULLIF(current_setting('request.jwt.claim.sub', true), '')::uuid;
$$;

GRANT USAGE ON SCHEMA public, auth TO authenticated, anon;
GRANT ALL ON ALL TABLES IN SCHEMA public TO authenticated;
GRANT SELECT ON ALL TABLES IN SCHEMA public TO anon;
GRANT SELECT ON ALL TABLES IN SCHEMA auth TO authenticated, anon;
GRANT EXECUTE ON ALL FUNCTIONS IN SCHEMA public, auth TO authenticated, anon;

-- ============================================================
-- 2. Test Fixtures Setup
-- ============================================================

DO $$
DECLARE
    org_a UUID := 'f1210001-aaaa-aaaa-aaaa-000000000001';
    org_b UUID := 'f1210001-bbbb-bbbb-bbbb-000000000002';
    branch_a UUID := 'f1210002-aaaa-aaaa-aaaa-000000000001';
    branch_b UUID := 'f1210002-bbbb-bbbb-bbbb-000000000002';
    auth_owner_a UUID := '10000000-0000-0000-0000-000000000001';
    auth_owner_b UUID := '20000000-0000-0000-0000-000000000001';
BEGIN
    -- Auth Users
    INSERT INTO auth.users (id, email) VALUES
        (auth_owner_a, 'f121_owner_a@test.com'),
        (auth_owner_b, 'f121_owner_b@test.com')
    ON CONFLICT (id) DO NOTHING;

    -- Organizations
    INSERT INTO public.organizations (id, name, default_currency, default_language) VALUES
        (org_a, 'F1.21 Test Org A', 'USD', 'en'),
        (org_b, 'F1.21 Test Org B', 'EUR', 'de')
    ON CONFLICT (id) DO NOTHING;

    -- Organization Members
    INSERT INTO public.organization_members (organization_id, user_id, role) VALUES
        (org_a, auth_owner_a, 'owner'),
        (org_b, auth_owner_b, 'owner')
    ON CONFLICT (organization_id, user_id) DO NOTHING;

    -- Branches
    INSERT INTO public.branches (id, organization_id, name, currency, is_active) VALUES
        (branch_a, org_a, 'F1.21 Branch A', 'USD', true),
        (branch_b, org_b, 'F1.21 Branch B', 'EUR', true)
    ON CONFLICT (id) DO NOTHING;
END $$;

-- ============================================================
-- 3. Register Domain Constraints — Name & Code
-- ============================================================

-- Empty name rejected
DO $$
DECLARE
    org_a UUID := 'f1210001-aaaa-aaaa-aaaa-000000000001';
    branch_a UUID := 'f1210002-aaaa-aaaa-aaaa-000000000001';
BEGIN
    INSERT INTO public.registers (organization_id, branch_id, name, code)
    VALUES (org_a, branch_a, '', 'REG-01');

    RAISE EXCEPTION 'SCHEMA VIOLATION: Empty register name was permitted' USING ERRCODE = 'TF001';
EXCEPTION
    WHEN SQLSTATE 'TF001' THEN RAISE;
    WHEN check_violation THEN
        RAISE NOTICE 'PASS: Empty register name correctly rejected';
END $$;

-- Whitespace-only name rejected
DO $$
DECLARE
    org_a UUID := 'f1210001-aaaa-aaaa-aaaa-000000000001';
    branch_a UUID := 'f1210002-aaaa-aaaa-aaaa-000000000001';
BEGIN
    INSERT INTO public.registers (organization_id, branch_id, name, code)
    VALUES (org_a, branch_a, '   ', 'REG-01');

    RAISE EXCEPTION 'SCHEMA VIOLATION: Whitespace-only register name was permitted' USING ERRCODE = 'TF001';
EXCEPTION
    WHEN SQLSTATE 'TF001' THEN RAISE;
    WHEN check_violation THEN
        RAISE NOTICE 'PASS: Whitespace-only register name correctly rejected';
END $$;

-- Name exceeding 255 chars rejected
DO $$
DECLARE
    org_a UUID := 'f1210001-aaaa-aaaa-aaaa-000000000001';
    branch_a UUID := 'f1210002-aaaa-aaaa-aaaa-000000000001';
BEGIN
    INSERT INTO public.registers (organization_id, branch_id, name, code)
    VALUES (org_a, branch_a, repeat('a', 256), 'REG-01');

    RAISE EXCEPTION 'SCHEMA VIOLATION: 256-character register name was permitted' USING ERRCODE = 'TF001';
EXCEPTION
    WHEN SQLSTATE 'TF001' THEN RAISE;
    WHEN check_violation THEN
        RAISE NOTICE 'PASS: 256-character register name correctly rejected';
END $$;

-- Empty code rejected
DO $$
DECLARE
    org_a UUID := 'f1210001-aaaa-aaaa-aaaa-000000000001';
    branch_a UUID := 'f1210002-aaaa-aaaa-aaaa-000000000001';
BEGIN
    INSERT INTO public.registers (organization_id, branch_id, name, code)
    VALUES (org_a, branch_a, 'Valid Register', '');

    RAISE EXCEPTION 'SCHEMA VIOLATION: Empty register code was permitted' USING ERRCODE = 'TF001';
EXCEPTION
    WHEN SQLSTATE 'TF001' THEN RAISE;
    WHEN check_violation THEN
        RAISE NOTICE 'PASS: Empty register code correctly rejected';
END $$;

-- Code with invalid characters rejected (spaces or special symbols)
DO $$
DECLARE
    org_a UUID := 'f1210001-aaaa-aaaa-aaaa-000000000001';
    branch_a UUID := 'f1210002-aaaa-aaaa-aaaa-000000000001';
BEGIN
    INSERT INTO public.registers (organization_id, branch_id, name, code)
    VALUES (org_a, branch_a, 'Valid Register', 'REG 01!');

    RAISE EXCEPTION 'SCHEMA VIOLATION: Invalid register code characters permitted' USING ERRCODE = 'TF001';
EXCEPTION
    WHEN SQLSTATE 'TF001' THEN RAISE;
    WHEN check_violation THEN
        RAISE NOTICE 'PASS: Invalid register code characters correctly rejected';
END $$;

-- ============================================================
-- 4. Device Identifier Format Constraints
-- ============================================================

-- Device identifier too short (< 3 chars) rejected
DO $$
DECLARE
    org_a UUID := 'f1210001-aaaa-aaaa-aaaa-000000000001';
    branch_a UUID := 'f1210002-aaaa-aaaa-aaaa-000000000001';
BEGIN
    INSERT INTO public.registers (organization_id, branch_id, name, code, device_identifier, device_pairing_status)
    VALUES (org_a, branch_a, 'Valid Register', 'REG-D1', 'ab', 'paired');

    RAISE EXCEPTION 'SCHEMA VIOLATION: 2-character device identifier was permitted' USING ERRCODE = 'TF001';
EXCEPTION
    WHEN SQLSTATE 'TF001' THEN RAISE;
    WHEN check_violation THEN
        RAISE NOTICE 'PASS: Short device identifier correctly rejected';
END $$;

-- Device identifier with invalid characters rejected
DO $$
DECLARE
    org_a UUID := 'f1210001-aaaa-aaaa-aaaa-000000000001';
    branch_a UUID := 'f1210002-aaaa-aaaa-aaaa-000000000001';
BEGIN
    INSERT INTO public.registers (organization_id, branch_id, name, code, device_identifier, device_pairing_status)
    VALUES (org_a, branch_a, 'Valid Register', 'REG-D2', 'dev#id$invalid', 'paired');

    RAISE EXCEPTION 'SCHEMA VIOLATION: Invalid device identifier characters permitted' USING ERRCODE = 'TF001';
EXCEPTION
    WHEN SQLSTATE 'TF001' THEN RAISE;
    WHEN check_violation THEN
        RAISE NOTICE 'PASS: Invalid device identifier characters correctly rejected';
END $$;

-- ============================================================
-- 5. Device Pairing Lifecycle State Constraints
-- ============================================================

-- Invalid pairing status rejected
DO $$
DECLARE
    org_a UUID := 'f1210001-aaaa-aaaa-aaaa-000000000001';
    branch_a UUID := 'f1210002-aaaa-aaaa-aaaa-000000000001';
BEGIN
    INSERT INTO public.registers (organization_id, branch_id, name, code, device_pairing_status)
    VALUES (org_a, branch_a, 'Valid Register', 'REG-D3', 'active');

    RAISE EXCEPTION 'SCHEMA VIOLATION: Invalid pairing status was permitted' USING ERRCODE = 'TF001';
EXCEPTION
    WHEN SQLSTATE 'TF001' THEN RAISE;
    WHEN check_violation THEN
        RAISE NOTICE 'PASS: Invalid pairing status correctly rejected';
END $$;

-- ============================================================
-- 6. Pairing Coherence Constraints
-- ============================================================

-- 'paired' status without device_identifier rejected
DO $$
DECLARE
    org_a UUID := 'f1210001-aaaa-aaaa-aaaa-000000000001';
    branch_a UUID := 'f1210002-aaaa-aaaa-aaaa-000000000001';
BEGIN
    INSERT INTO public.registers (organization_id, branch_id, name, code, device_identifier, device_pairing_status)
    VALUES (org_a, branch_a, 'Valid Register', 'REG-D4', NULL, 'paired');

    RAISE EXCEPTION 'SCHEMA VIOLATION: Paired status without device_identifier was permitted' USING ERRCODE = 'TF001';
EXCEPTION
    WHEN SQLSTATE 'TF001' THEN RAISE;
    WHEN check_violation THEN
        RAISE NOTICE 'PASS: Paired status without device_identifier correctly rejected';
END $$;

-- 'unpaired' status with non-null device_identifier rejected
DO $$
DECLARE
    org_a UUID := 'f1210001-aaaa-aaaa-aaaa-000000000001';
    branch_a UUID := 'f1210002-aaaa-aaaa-aaaa-000000000001';
BEGIN
    INSERT INTO public.registers (organization_id, branch_id, name, code, device_identifier, device_pairing_status)
    VALUES (org_a, branch_a, 'Valid Register', 'REG-D5', 'pos-hw-terminal-01', 'unpaired');

    RAISE EXCEPTION 'SCHEMA VIOLATION: Unpaired status with device_identifier was permitted' USING ERRCODE = 'TF001';
EXCEPTION
    WHEN SQLSTATE 'TF001' THEN RAISE;
    WHEN check_violation THEN
        RAISE NOTICE 'PASS: Unpaired status with device_identifier correctly rejected';
END $$;

-- ============================================================
-- 7. Active Device Uniqueness Per Organization
-- ============================================================

DO $$
DECLARE
    org_a UUID := 'f1210001-aaaa-aaaa-aaaa-000000000001';
    branch_a UUID := 'f1210002-aaaa-aaaa-aaaa-000000000001';
    reg1_id UUID;
    reg2_id UUID;
BEGIN
    -- First register with device 'hw-pos-station-01'
    INSERT INTO public.registers (organization_id, branch_id, name, code, device_identifier, device_pairing_status, device_paired_at)
    VALUES (org_a, branch_a, 'Register 1', 'REG-U1', 'hw-pos-station-01', 'paired', now())
    RETURNING id INTO reg1_id;

    -- Second register attempting to pair SAME device in SAME org must fail
    INSERT INTO public.registers (organization_id, branch_id, name, code, device_identifier, device_pairing_status, device_paired_at)
    VALUES (org_a, branch_a, 'Register 2', 'REG-U2', 'hw-pos-station-01', 'paired', now())
    RETURNING id INTO reg2_id;

    RAISE EXCEPTION 'SCHEMA VIOLATION: Duplicate active device in same org was permitted' USING ERRCODE = 'TF001';
EXCEPTION
    WHEN SQLSTATE 'TF001' THEN RAISE;
    WHEN unique_violation THEN
        DELETE FROM public.registers WHERE id = reg1_id;
        RAISE NOTICE 'PASS: Duplicate active device in same org correctly rejected by unique index';
END $$;

-- ============================================================
-- 8. Global Active Device Uniqueness (Cross-Tenant)
-- ============================================================

DO $$
DECLARE
    org_a UUID := 'f1210001-aaaa-aaaa-aaaa-000000000001';
    org_b UUID := 'f1210001-bbbb-bbbb-bbbb-000000000002';
    branch_a UUID := 'f1210002-aaaa-aaaa-aaaa-000000000001';
    branch_b UUID := 'f1210002-bbbb-bbbb-bbbb-000000000002';
    reg_a_id UUID;
    reg_b_id UUID;
BEGIN
    -- Pair hardware to Org A
    INSERT INTO public.registers (organization_id, branch_id, name, code, device_identifier, device_pairing_status, device_paired_at)
    VALUES (org_a, branch_a, 'Register A', 'REG-GA1', 'hw-global-terminal-99', 'paired', now())
    RETURNING id INTO reg_a_id;

    -- Attempt to pair SAME hardware actively to Org B must fail
    INSERT INTO public.registers (organization_id, branch_id, name, code, device_identifier, device_pairing_status, device_paired_at)
    VALUES (org_b, branch_b, 'Register B', 'REG-GB1', 'hw-global-terminal-99', 'paired', now())
    RETURNING id INTO reg_b_id;

    RAISE EXCEPTION 'SCHEMA VIOLATION: Cross-tenant duplicate active device was permitted' USING ERRCODE = 'TF001';
EXCEPTION
    WHEN SQLSTATE 'TF001' THEN RAISE;
    WHEN unique_violation THEN
        DELETE FROM public.registers WHERE id = reg_a_id;
        RAISE NOTICE 'PASS: Cross-tenant duplicate active device correctly rejected by global unique index';
END $$;

-- ============================================================
-- 9. Device Re-pairing After Revocation
-- ============================================================

DO $$
DECLARE
    org_a UUID := 'f1210001-aaaa-aaaa-aaaa-000000000001';
    org_b UUID := 'f1210001-bbbb-bbbb-bbbb-000000000002';
    branch_a UUID := 'f1210002-aaaa-aaaa-aaaa-000000000001';
    branch_b UUID := 'f1210002-bbbb-bbbb-bbbb-000000000002';
    reg_a_id UUID;
    reg_b_id UUID;
BEGIN
    -- Pair hardware to Org A
    INSERT INTO public.registers (organization_id, branch_id, name, code, device_identifier, device_pairing_status, device_paired_at)
    VALUES (org_a, branch_a, 'Register A', 'REG-REV1', 'hw-transfer-terminal-01', 'paired', now())
    RETURNING id INTO reg_a_id;

    -- Revoke pairing on Register A
    UPDATE public.registers
    SET device_pairing_status = 'revoked'
    WHERE id = reg_a_id;

    -- Now Org B should successfully pair the same hardware because it is no longer actively paired
    INSERT INTO public.registers (organization_id, branch_id, name, code, device_identifier, device_pairing_status, device_paired_at)
    VALUES (org_b, branch_b, 'Register B', 'REG-REV2', 'hw-transfer-terminal-01', 'paired', now())
    RETURNING id INTO reg_b_id;

    IF reg_b_id IS NULL THEN
        RAISE EXCEPTION 'SCHEMA VIOLATION: Re-pairing revoked device failed' USING ERRCODE = 'TF001';
    END IF;

    DELETE FROM public.registers WHERE id IN (reg_a_id, reg_b_id);
    RAISE NOTICE 'PASS: Device re-pairing succeeded after previous binding was revoked';
END $$;

-- ============================================================
-- 10. Cross-Tenant Composite Foreign Key Rejection
-- ============================================================

DO $$
DECLARE
    org_a UUID := 'f1210001-aaaa-aaaa-aaaa-000000000001';
    branch_b UUID := 'f1210002-bbbb-bbbb-bbbb-000000000002';
BEGIN
    INSERT INTO public.registers (organization_id, branch_id, name, code)
    VALUES (org_a, branch_b, 'Cross Tenant Register', 'XREG-99');

    RAISE EXCEPTION 'SCHEMA VIOLATION: Cross-tenant branch register was permitted' USING ERRCODE = 'TF001';
EXCEPTION
    WHEN SQLSTATE 'TF001' THEN RAISE;
    WHEN foreign_key_violation THEN
        RAISE NOTICE 'PASS: Cross-tenant branch register correctly rejected by composite FK';
END $$;

-- ============================================================
-- 11. Cascading Deletion Verification
-- ============================================================

DO $$
DECLARE
    test_org_id UUID;
    test_branch_id UUID;
    test_reg_id UUID;
BEGIN
    INSERT INTO public.organizations (name, default_currency, default_language)
    VALUES ('Cascade Register Org', 'USD', 'en')
    RETURNING id INTO test_org_id;

    INSERT INTO public.branches (organization_id, name, currency)
    VALUES (test_org_id, 'Cascade Branch', 'USD')
    RETURNING id INTO test_branch_id;

    INSERT INTO public.registers (organization_id, branch_id, name, code, device_identifier, device_pairing_status)
    VALUES (test_org_id, test_branch_id, 'Cascade Reg', 'CREG-01', 'cascade-device-01', 'paired')
    RETURNING id INTO test_reg_id;

    -- Deleting organization must cascade to registers
    DELETE FROM public.organizations WHERE id = test_org_id;

    IF EXISTS (SELECT 1 FROM public.registers WHERE id = test_reg_id) THEN
        RAISE EXCEPTION 'SCHEMA VIOLATION: Register was not cascaded upon organization deletion' USING ERRCODE = 'TF001';
    END IF;

    RAISE NOTICE 'PASS: Organization deletion correctly cascades to registers';
END $$;

-- ============================================================
-- 12. Transaction-Stable updated_at Trigger on Registers
-- ============================================================

DO $$
DECLARE
    org_a UUID := 'f1210001-aaaa-aaaa-aaaa-000000000001';
    branch_a UUID := 'f1210002-aaaa-aaaa-aaaa-000000000001';
    reg_id UUID;
    ts_after TIMESTAMPTZ;
    stale_ts TIMESTAMPTZ := now() - INTERVAL '1 day';
BEGIN
    INSERT INTO public.registers (organization_id, branch_id, name, code)
    VALUES (org_a, branch_a, 'Timestamp Test Register', 'TS-REG-01')
    RETURNING id INTO reg_id;

    -- Force stale timestamp
    ALTER TABLE public.registers DISABLE TRIGGER trg_updated_at_registers;
    UPDATE public.registers SET updated_at = stale_ts WHERE id = reg_id;
    ALTER TABLE public.registers ENABLE TRIGGER trg_updated_at_registers;

    -- Update register
    UPDATE public.registers SET name = 'Timestamp Test Register Updated' WHERE id = reg_id;

    SELECT updated_at INTO ts_after FROM public.registers WHERE id = reg_id;

    IF ts_after <= stale_ts THEN
        RAISE EXCEPTION 'SCHEMA VIOLATION: updated_at was not updated by trigger (stale=%, actual=%)',
            stale_ts, ts_after USING ERRCODE = 'TF001';
    END IF;

    DELETE FROM public.registers WHERE id = reg_id;
    RAISE NOTICE 'PASS: updated_at trigger correctly advances on register UPDATE';
END $$;

ROLLBACK;
