-- Supabase Test Suite: privileged_server_functions_test.sql
-- F1.23 — Privileged Server Functions Behavioral Test Suite
-- Real PostgreSQL 15 runtime tests for all 17 approved test cases.

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
    owner_a UUID := '11111111-1111-1111-1111-111111111111';
    admin_a UUID := '12121212-1212-1212-1212-121212121212';
    manager_a UUID := '13131313-1313-1313-1313-131313131313';
    cashier_a UUID := '14141414-1414-1414-1414-141414141414';
    owner_b UUID := '22222222-2222-2222-2222-222222222222';
    unaffiliated UUID := '99999999-9999-9999-9999-999999999999';

    org_a UUID := 'f1230001-aaaa-aaaa-aaaa-000000000001';
    org_b UUID := 'f1230001-bbbb-bbbb-bbbb-000000000002';

    branch_a UUID := 'f1230002-aaaa-aaaa-aaaa-000000000001';
    branch_b UUID := 'f1230002-bbbb-bbbb-bbbb-000000000002';

    reg_a_unpaired UUID := 'f1230003-aaaa-aaaa-aaaa-000000000001';
    reg_a_paired UUID := 'f1230003-aaaa-aaaa-aaaa-000000000002';
    reg_b UUID := 'f1230003-bbbb-bbbb-bbbb-000000000001';

    user_a_cashier UUID := 'f1230004-aaaa-aaaa-aaaa-000000000001';
BEGIN
    -- Auth users
    INSERT INTO auth.users (id, email) VALUES
        (owner_a, 'f123_owner_a@tenant-a.com'),
        (admin_a, 'f123_admin_a@tenant-a.com'),
        (manager_a, 'f123_manager_a@tenant-a.com'),
        (cashier_a, 'f123_cashier_a@tenant-a.com'),
        (owner_b, 'f123_owner_b@tenant-b.com'),
        (unaffiliated, 'f123_unaffiliated@nowhere.com')
    ON CONFLICT (id) DO NOTHING;

    -- Organizations
    INSERT INTO public.organizations (id, name, default_currency, default_language) VALUES
        (org_a, 'F1.23 RPC Org A', 'USD', 'en'),
        (org_b, 'F1.23 RPC Org B', 'EUR', 'de')
    ON CONFLICT (id) DO NOTHING;

    -- Organization Memberships
    INSERT INTO public.organization_members (organization_id, user_id, role) VALUES
        (org_a, owner_a, 'owner'),
        (org_a, admin_a, 'admin'),
        (org_a, manager_a, 'manager'),
        (org_a, cashier_a, 'cashier'),
        (org_b, owner_b, 'owner')
    ON CONFLICT (organization_id, user_id) DO NOTHING;

    -- Branches
    INSERT INTO public.branches (id, organization_id, name, currency, is_active) VALUES
        (branch_a, org_a, 'Branch A', 'USD', true),
        (branch_b, org_b, 'Branch B', 'EUR', true)
    ON CONFLICT (id) DO NOTHING;

    -- Registers
    INSERT INTO public.registers (id, organization_id, branch_id, name, code, is_active, device_pairing_status) VALUES
        (reg_a_unpaired, org_a, branch_a, 'Unpaired Register A', 'REG-UNPAIRED', true, 'unpaired')
    ON CONFLICT (id) DO NOTHING;

    INSERT INTO public.registers (id, organization_id, branch_id, name, code, is_active, device_identifier, device_pairing_status, device_paired_at, device_last_seen_at) VALUES
        (reg_a_paired, org_a, branch_a, 'Paired Register A', 'REG-PAIRED', true, 'hw-terminal-a-01', 'paired', now() - INTERVAL '1 hour', now() - INTERVAL '1 hour'),
        (reg_b, org_b, branch_b, 'Register B1', 'REG-B1', true, 'hw-terminal-b-01', 'paired', now() - INTERVAL '1 hour', now() - INTERVAL '1 hour')
    ON CONFLICT (id) DO NOTHING;

    -- POS Users
    INSERT INTO public.users (id, organization_id, branch_id, supabase_user_id, full_name, username, role, is_active) VALUES
        (user_a_cashier, org_a, branch_a, cashier_a, 'Cashier User A', 'cashier_a', 'cashier', true)
    ON CONFLICT (id) DO NOTHING;
END $$;

-- ============================================================
-- 3. Tests 1–5: pair_device_to_register Verification
-- ============================================================

-- Case 1: pair_device_to_register — manager success
DO $$
DECLARE
    manager_a UUID := '13131313-1313-1313-1313-131313131313';
    reg_id UUID := 'f1230003-aaaa-aaaa-aaaa-000000000001';
    res public.registers;
BEGIN
    SET LOCAL ROLE authenticated;
    PERFORM set_config('request.jwt.claim.sub', manager_a::text, true);

    res := public.pair_device_to_register(reg_id, 'hw-new-terminal-01');

    IF res.device_pairing_status <> 'paired' OR res.device_identifier <> 'hw-new-terminal-01' THEN
        RAISE EXCEPTION 'RPC FAIL: pair_device_to_register did not set status to paired';
    END IF;

    RAISE NOTICE 'PASS Case 1: pair_device_to_register manager success';
END $$;

-- Case 2: pair_device_to_register — cashier denied
DO $$
DECLARE
    cashier_a UUID := '14141414-1414-1414-1414-141414141414';
    reg_id UUID := 'f1230003-aaaa-aaaa-aaaa-000000000001';
    res public.registers;
BEGIN
    SET LOCAL ROLE authenticated;
    PERFORM set_config('request.jwt.claim.sub', cashier_a::text, true);

    BEGIN
        res := public.pair_device_to_register(reg_id, 'hw-rogue-device-01');
        RAISE EXCEPTION 'SECURITY VIOLATION: Cashier was able to pair device' USING ERRCODE = 'TF001';
    EXCEPTION
        WHEN SQLSTATE 'TF001' THEN RAISE;
        WHEN insufficient_privilege THEN
            RAISE NOTICE 'PASS Case 2: pair_device_to_register cashier denied (SQLSTATE 42501)';
    END;
END $$;

-- Case 3: pair_device_to_register — cross-tenant denied
DO $$
DECLARE
    manager_a UUID := '13131313-1313-1313-1313-131313131313';
    reg_b UUID := 'f1230003-bbbb-bbbb-bbbb-000000000001';
    res public.registers;
BEGIN
    SET LOCAL ROLE authenticated;
    PERFORM set_config('request.jwt.claim.sub', manager_a::text, true);

    BEGIN
        res := public.pair_device_to_register(reg_b, 'hw-rogue-device-02');
        RAISE EXCEPTION 'SECURITY VIOLATION: Manager A was able to pair device in Org B' USING ERRCODE = 'TF001';
    EXCEPTION
        WHEN SQLSTATE 'TF001' THEN RAISE;
        WHEN insufficient_privilege THEN
            RAISE NOTICE 'PASS Case 3: pair_device_to_register cross-tenant denied (SQLSTATE 42501)';
    END;
END $$;

-- Case 4: pair_device_to_register — unaffiliated user denied
DO $$
DECLARE
    unaffiliated UUID := '99999999-9999-9999-9999-999999999999';
    reg_id UUID := 'f1230003-aaaa-aaaa-aaaa-000000000001';
    res public.registers;
BEGIN
    SET LOCAL ROLE authenticated;
    PERFORM set_config('request.jwt.claim.sub', unaffiliated::text, true);

    BEGIN
        res := public.pair_device_to_register(reg_id, 'hw-rogue-device-03');
        RAISE EXCEPTION 'SECURITY VIOLATION: Unaffiliated user was able to pair device' USING ERRCODE = 'TF001';
    EXCEPTION
        WHEN SQLSTATE 'TF001' THEN RAISE;
        WHEN insufficient_privilege THEN
            RAISE NOTICE 'PASS Case 4: pair_device_to_register unaffiliated user denied (SQLSTATE 42501)';
    END;
END $$;

-- Case 5: pair_device_to_register — anonymous caller denied
DO $$
DECLARE
    reg_id UUID := 'f1230003-aaaa-aaaa-aaaa-000000000001';
    res public.registers;
BEGIN
    SET LOCAL ROLE anon;
    PERFORM set_config('request.jwt.claim.sub', '', true);

    BEGIN
        res := public.pair_device_to_register(reg_id, 'hw-rogue-device-04');
        RAISE EXCEPTION 'SECURITY VIOLATION: Anonymous caller was able to pair device' USING ERRCODE = 'TF001';
    EXCEPTION
        WHEN SQLSTATE 'TF001' THEN RAISE;
        WHEN insufficient_privilege THEN
            RAISE NOTICE 'PASS Case 5: pair_device_to_register anonymous caller denied (SQLSTATE 42501)';
    END;
END $$;

-- ============================================================
-- 4. Tests 6–8: revoke_device_pairing Verification
-- ============================================================

-- Case 6: revoke_device_pairing — admin success
DO $$
DECLARE
    admin_a UUID := '12121212-1212-1212-1212-121212121212';
    reg_id UUID := 'f1230003-aaaa-aaaa-aaaa-000000000002';
    res public.registers;
BEGIN
    SET LOCAL ROLE authenticated;
    PERFORM set_config('request.jwt.claim.sub', admin_a::text, true);

    res := public.revoke_device_pairing(reg_id);

    IF res.device_pairing_status <> 'revoked' THEN
        RAISE EXCEPTION 'RPC FAIL: revoke_device_pairing did not set status to revoked';
    END IF;

    RAISE NOTICE 'PASS Case 6: revoke_device_pairing admin success';
END $$;

-- Case 7: revoke_device_pairing — cashier denied
DO $$
DECLARE
    cashier_a UUID := '14141414-1414-1414-1414-141414141414';
    reg_id UUID := 'f1230003-aaaa-aaaa-aaaa-000000000001';
    res public.registers;
BEGIN
    SET LOCAL ROLE authenticated;
    PERFORM set_config('request.jwt.claim.sub', cashier_a::text, true);

    BEGIN
        res := public.revoke_device_pairing(reg_id);
        RAISE EXCEPTION 'SECURITY VIOLATION: Cashier was able to revoke device pairing' USING ERRCODE = 'TF001';
    EXCEPTION
        WHEN SQLSTATE 'TF001' THEN RAISE;
        WHEN insufficient_privilege THEN
            RAISE NOTICE 'PASS Case 7: revoke_device_pairing cashier denied (SQLSTATE 42501)';
    END;
END $$;

-- Case 8: revoke_device_pairing — cross-tenant denied
DO $$
DECLARE
    admin_a UUID := '12121212-1212-1212-1212-121212121212';
    reg_b UUID := 'f1230003-bbbb-bbbb-bbbb-000000000001';
    res public.registers;
BEGIN
    SET LOCAL ROLE authenticated;
    PERFORM set_config('request.jwt.claim.sub', admin_a::text, true);

    BEGIN
        res := public.revoke_device_pairing(reg_b);
        RAISE EXCEPTION 'SECURITY VIOLATION: Admin A was able to revoke device pairing in Org B' USING ERRCODE = 'TF001';
    EXCEPTION
        WHEN SQLSTATE 'TF001' THEN RAISE;
        WHEN insufficient_privilege THEN
            RAISE NOTICE 'PASS Case 8: revoke_device_pairing cross-tenant denied (SQLSTATE 42501)';
    END;
END $$;

-- ============================================================
-- 5. Tests 9–11: record_device_heartbeat Verification
-- ============================================================

-- Case 9: record_device_heartbeat — valid matching device succeeds
DO $$
DECLARE
    cashier_a UUID := '14141414-1414-1414-1414-141414141414';
    reg_id UUID := 'f1230003-aaaa-aaaa-aaaa-000000000001';
    t_before TIMESTAMPTZ;
    t_after TIMESTAMPTZ;
BEGIN
    -- now() is the transaction timestamp and does not advance in this test
    -- transaction, so age the row as table owner before the heartbeat call.
    RESET ROLE;
    UPDATE public.registers
    SET device_last_seen_at = now() - INTERVAL '1 hour'
    WHERE id = reg_id;

    SET LOCAL ROLE authenticated;
    PERFORM set_config('request.jwt.claim.sub', cashier_a::text, true);

    SELECT device_last_seen_at INTO t_before FROM public.registers WHERE id = reg_id;

    PERFORM public.record_device_heartbeat(reg_id, 'hw-new-terminal-01');

    SELECT device_last_seen_at INTO t_after FROM public.registers WHERE id = reg_id;

    IF t_after <= t_before OR t_after IS NULL THEN
        RAISE EXCEPTION 'RPC FAIL: record_device_heartbeat did not update timestamp';
    END IF;

    RAISE NOTICE 'PASS Case 9: record_device_heartbeat valid matching device succeeds';
END $$;

-- Case 10: record_device_heartbeat — mismatched device denied
DO $$
DECLARE
    cashier_a UUID := '14141414-1414-1414-1414-141414141414';
    reg_id UUID := 'f1230003-aaaa-aaaa-aaaa-000000000001';
BEGIN
    SET LOCAL ROLE authenticated;
    PERFORM set_config('request.jwt.claim.sub', cashier_a::text, true);

    -- 1. Test mismatched device identifier
    BEGIN
        PERFORM public.record_device_heartbeat(reg_id, 'hw-wrong-device-99');
        RAISE EXCEPTION 'SECURITY VIOLATION: Heartbeat accepted mismatched device identifier' USING ERRCODE = 'TF001';
    EXCEPTION
        WHEN SQLSTATE 'TF001' THEN RAISE;
        WHEN insufficient_privilege THEN
            NULL;
    END;

    -- 2. Test NULL device identifier fail-closed
    BEGIN
        PERFORM public.record_device_heartbeat(reg_id, NULL);
        RAISE EXCEPTION 'SECURITY VIOLATION: Heartbeat accepted NULL device identifier' USING ERRCODE = 'TF001';
    EXCEPTION
        WHEN SQLSTATE 'TF001' THEN RAISE;
        WHEN insufficient_privilege THEN
            NULL;
    END;

    RAISE NOTICE 'PASS Case 10: record_device_heartbeat mismatched device denied (SQLSTATE 42501)';
END $$;

-- Case 11: record_device_heartbeat — cross-tenant denied
DO $$
DECLARE
    cashier_a UUID := '14141414-1414-1414-1414-141414141414';
    reg_b UUID := 'f1230003-bbbb-bbbb-bbbb-000000000001';
BEGIN
    SET LOCAL ROLE authenticated;
    PERFORM set_config('request.jwt.claim.sub', cashier_a::text, true);

    BEGIN
        PERFORM public.record_device_heartbeat(reg_b, 'hw-terminal-b-01');
        RAISE EXCEPTION 'SECURITY VIOLATION: Cashier A was able to record heartbeat in Org B' USING ERRCODE = 'TF001';
    EXCEPTION
        WHEN SQLSTATE 'TF001' THEN RAISE;
        WHEN insufficient_privilege THEN
            RAISE NOTICE 'PASS Case 11: record_device_heartbeat cross-tenant denied (SQLSTATE 42501)';
    END;
END $$;

-- ============================================================
-- 6. Tests 12–13: create_organization_with_initial_setup Verification
-- ============================================================

-- Case 12: create_organization_with_initial_setup — authenticated success
DO $$
DECLARE
    unaffiliated UUID := '99999999-9999-9999-9999-999999999999';
    res JSONB;
    new_org_id UUID;
    is_owner BOOLEAN;
BEGIN
    SET LOCAL ROLE authenticated;
    PERFORM set_config('request.jwt.claim.sub', unaffiliated::text, true);

    res := public.create_organization_with_initial_setup(
        'New Global Retailer',
        'USD',
        'en',
        'Downtown Branch',
        'Main Register',
        'REG-MAIN-01'
    );

    new_org_id := (res->'organization'->>'id')::UUID;
    IF new_org_id IS NULL THEN
        RAISE EXCEPTION 'RPC FAIL: create_organization_with_initial_setup did not return valid organization id';
    END IF;

    -- Verify caller is owner
    SELECT EXISTS (
        SELECT 1 FROM public.organization_members
        WHERE organization_id = new_org_id AND user_id = unaffiliated AND role = 'owner'
    ) INTO is_owner;

    IF NOT is_owner THEN
        RAISE EXCEPTION 'RPC FAIL: Creator was not assigned owner role in new organization';
    END IF;

    -- Verify branch, register, and user were created
    IF (res->'branch'->>'id') IS NULL OR (res->'register'->>'id') IS NULL OR (res->'user'->>'id') IS NULL THEN
        RAISE EXCEPTION 'RPC FAIL: Initial setup entities missing in response';
    END IF;

    RAISE NOTICE 'PASS Case 12: create_organization_with_initial_setup authenticated success';
END $$;

-- Case 13: create_organization_with_initial_setup — anonymous denied
DO $$
DECLARE
    res JSONB;
BEGIN
    SET LOCAL ROLE anon;
    PERFORM set_config('request.jwt.claim.sub', '', true);

    BEGIN
        res := public.create_organization_with_initial_setup(
            'Anon Retailer',
            'USD',
            'en'
        );
        RAISE EXCEPTION 'SECURITY VIOLATION: Anonymous caller was able to create organization' USING ERRCODE = 'TF001';
    EXCEPTION
        WHEN SQLSTATE 'TF001' THEN RAISE;
        WHEN insufficient_privilege THEN
            RAISE NOTICE 'PASS Case 13: create_organization_with_initial_setup anonymous denied (SQLSTATE 42501)';
    END;
END $$;

-- ============================================================
-- 7. Tests 14–17: set_organization_member_role Verification
-- ============================================================

-- Case 14: set_organization_member_role — owner promotes cashier to admin
DO $$
DECLARE
    owner_a UUID := '11111111-1111-1111-1111-111111111111';
    cashier_a UUID := '14141414-1414-1414-1414-141414141414';
    org_a UUID := 'f1230001-aaaa-aaaa-aaaa-000000000001';
    res public.organization_members;
BEGIN
    SET LOCAL ROLE authenticated;
    PERFORM set_config('request.jwt.claim.sub', owner_a::text, true);

    res := public.set_organization_member_role(org_a, cashier_a, 'admin');

    IF res.role <> 'admin' THEN
        RAISE EXCEPTION 'RPC FAIL: set_organization_member_role did not update role to admin';
    END IF;

    RAISE NOTICE 'PASS Case 14: set_organization_member_role owner promotes cashier to admin';
END $$;

-- Case 15: set_organization_member_role — admin cannot promote to owner
DO $$
DECLARE
    admin_a UUID := '12121212-1212-1212-1212-121212121212';
    manager_a UUID := '13131313-1313-1313-1313-131313131313';
    org_a UUID := 'f1230001-aaaa-aaaa-aaaa-000000000001';
    res public.organization_members;
BEGIN
    SET LOCAL ROLE authenticated;
    PERFORM set_config('request.jwt.claim.sub', admin_a::text, true);

    BEGIN
        res := public.set_organization_member_role(org_a, manager_a, 'owner');
        RAISE EXCEPTION 'SECURITY VIOLATION: Admin was able to promote member to owner' USING ERRCODE = 'TF001';
    EXCEPTION
        WHEN SQLSTATE 'TF001' THEN RAISE;
        WHEN insufficient_privilege THEN
            RAISE NOTICE 'PASS Case 15: set_organization_member_role admin cannot promote to owner (SQLSTATE 42501)';
    END;
END $$;

-- Case 16: set_organization_member_role — cross-tenant target denied
DO $$
DECLARE
    admin_a UUID := '12121212-1212-1212-1212-121212121212';
    owner_b UUID := '22222222-2222-2222-2222-222222222222';
    org_a UUID := 'f1230001-aaaa-aaaa-aaaa-000000000001';
    res public.organization_members;
BEGIN
    SET LOCAL ROLE authenticated;
    PERFORM set_config('request.jwt.claim.sub', admin_a::text, true);

    BEGIN
        res := public.set_organization_member_role(org_a, owner_b, 'cashier');
        RAISE EXCEPTION 'SECURITY VIOLATION: Admin A was able to set role for cross-tenant user' USING ERRCODE = 'TF001';
    EXCEPTION
        WHEN SQLSTATE 'TF001' THEN RAISE;
        WHEN no_data_found OR SQLSTATE 'P0002' THEN
            RAISE NOTICE 'PASS Case 16: set_organization_member_role cross-tenant target denied (P0002)';
    END;
END $$;

-- Case 17: set_organization_member_role — sole owner demotion denied
DO $$
DECLARE
    owner_a UUID := '11111111-1111-1111-1111-111111111111';
    org_a UUID := 'f1230001-aaaa-aaaa-aaaa-000000000001';
    res public.organization_members;
BEGIN
    SET LOCAL ROLE authenticated;
    PERFORM set_config('request.jwt.claim.sub', owner_a::text, true);

    BEGIN
        res := public.set_organization_member_role(org_a, owner_a, 'admin');
        RAISE EXCEPTION 'SECURITY VIOLATION: Sole owner was able to demote themselves' USING ERRCODE = 'TF001';
    EXCEPTION
        WHEN SQLSTATE 'TF001' THEN RAISE;
        WHEN check_violation OR SQLSTATE '23514' THEN
            RAISE NOTICE 'PASS Case 17: set_organization_member_role sole owner demotion denied (SQLSTATE 23514)';
    END;
END $$;

ROLLBACK;
