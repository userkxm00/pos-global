-- Supabase Test Suite: rls_policies_test.sql
-- F1.08 — Supabase RLS policies verification
-- Deterministic test assertions for Phase 1 RLS Tenant Isolation and Role Policies

BEGIN;

-- 1. Create Mock auth schema if not exists for standalone testing
CREATE SCHEMA IF NOT EXISTS auth;
CREATE TABLE IF NOT EXISTS auth.users (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    email TEXT UNIQUE,
    created_at TIMESTAMPTZ DEFAULT now()
);

-- 2. Test Fixtures Setup
DO $$
DECLARE
    user_a_owner UUID := '11111111-1111-1111-1111-111111111111';
    user_a_cashier UUID := '22222222-2222-2222-2222-222222222222';
    user_b_owner UUID := '33333333-3333-3333-3333-333333333333';
    user_intruder UUID := '44444444-4444-4444-4444-444444444444';

    org_a UUID := 'aaaaaaaa-aaaa-aaaa-aaaa-aaaaaaaaaaaa';
    org_b UUID := 'bbbbbbbb-bbbb-bbbb-bbbb-bbbbbbbbbbbb';

    branch_a UUID := 'a1a1a1a1-a1a1-a1a1-a1a1-a1a1a1a1a1a1';
    branch_b UUID := 'b1b1b1b1-b1b1-b1b1-b1b1-b1b1b1b1b1b1';

    reg_a UUID := 'a2a2a2a2-a2a2-a2a2-a2a2-a2a2a2a2a2a2';
    reg_b UUID := 'b2b2b2b2-b2b2-b2b2-b2b2-b2b2b2b2b2b2';
BEGIN
    -- Insert test auth users
    INSERT INTO auth.users (id, email) VALUES
        (user_a_owner, 'owner_a@tenant-a.com'),
        (user_a_cashier, 'cashier_a@tenant-a.com'),
        (user_b_owner, 'owner_b@tenant-b.com'),
        (user_intruder, 'intruder@external.com')
    ON CONFLICT (id) DO NOTHING;

    -- Insert Organizations
    INSERT INTO public.organizations (id, name, default_currency, default_language) VALUES
        (org_a, 'Tenant Organization A', 'USD', 'en'),
        (org_b, 'Tenant Organization B', 'EUR', 'de')
    ON CONFLICT (id) DO NOTHING;

    -- Insert Organization Memberships
    INSERT INTO public.organization_members (organization_id, user_id, role) VALUES
        (org_a, user_a_owner, 'owner'),
        (org_a, user_a_cashier, 'cashier'),
        (org_b, user_b_owner, 'owner')
    ON CONFLICT (organization_id, user_id) DO NOTHING;

    -- Insert Branches
    INSERT INTO public.branches (id, organization_id, name, currency, is_active) VALUES
        (branch_a, org_a, 'Branch A Main', 'USD', true),
        (branch_b, org_b, 'Branch B Main', 'EUR', true)
    ON CONFLICT (id) DO NOTHING;

    -- Insert Registers
    INSERT INTO public.registers (id, organization_id, branch_id, name, code, is_active) VALUES
        (reg_a, org_a, branch_a, 'POS-01', 'REG-A1', true),
        (reg_b, org_b, branch_b, 'POS-01', 'REG-B1', true)
    ON CONFLICT (id) DO NOTHING;

    -- Insert POS Users
    INSERT INTO public.users (id, organization_id, branch_id, supabase_user_id, full_name, username, role, is_active) VALUES
        ('a3a3a3a3-a3a3-a3a3-a3a3-a3a3a3a3a3a3', org_a, branch_a, user_a_owner, 'Alice Owner', 'alice_owner', 'admin', true),
        ('a4a4a4a4-a4a4-a4a4-a4a4-a4a4a4a4a4a4', org_a, branch_a, user_a_cashier, 'Bob Cashier', 'bob_cashier', 'cashier', true),
        ('b3b3b3b3-b3b3-b3b3-b3b3-b3b3b3b3b3b3', org_b, branch_b, user_b_owner, 'Charlie Owner', 'charlie_owner', 'admin', true)
    ON CONFLICT (id) DO NOTHING;
END $$;

-- 3. Deterministic Invariant Verifications

-- Test Case 1: Tenant A owner can see Org A, but cannot see Org B
DO $$
DECLARE
    cnt INTEGER;
BEGIN
    -- Set auth context to User A Owner
    PERFORM set_config('request.jwt.claim.sub', '11111111-1111-1111-1111-111111111111', true);

    SELECT COUNT(*) INTO cnt FROM public.organizations WHERE id = 'aaaaaaaa-aaaa-aaaa-aaaa-aaaaaaaaaaaa';
    IF cnt <> 1 THEN
        RAISE EXCEPTION 'RLS FAIL: User A Owner should see Organization A, got % rows', cnt;
    END IF;

    SELECT COUNT(*) INTO cnt FROM public.organizations WHERE id = 'bbbbbbbb-bbbb-bbbb-bbbb-bbbbbbbbbbbb';
    IF cnt <> 0 THEN
        RAISE EXCEPTION 'RLS SECURITY VIOLATION: User A Owner must NOT see Organization B, got % rows', cnt;
    END IF;
END $$;

-- Test Case 2: Tenant A owner can see Branch A and Register A, but not Branch B or Register B
DO $$
DECLARE
    cnt INTEGER;
BEGIN
    PERFORM set_config('request.jwt.claim.sub', '11111111-1111-1111-1111-111111111111', true);

    SELECT COUNT(*) INTO cnt FROM public.branches WHERE organization_id = 'aaaaaaaa-aaaa-aaaa-aaaa-aaaaaaaaaaaa';
    IF cnt < 1 THEN
        RAISE EXCEPTION 'RLS FAIL: User A Owner should see branches in Org A';
    END IF;

    SELECT COUNT(*) INTO cnt FROM public.branches WHERE organization_id = 'bbbbbbbb-bbbb-bbbb-bbbb-bbbbbbbbbbbb';
    IF cnt <> 0 THEN
        RAISE EXCEPTION 'RLS SECURITY VIOLATION: User A Owner must NOT see branches in Org B, got % rows', cnt;
    END IF;

    SELECT COUNT(*) INTO cnt FROM public.registers WHERE organization_id = 'bbbbbbbb-bbbb-bbbb-bbbb-bbbbbbbbbbbb';
    IF cnt <> 0 THEN
        RAISE EXCEPTION 'RLS SECURITY VIOLATION: User A Owner must NOT see registers in Org B, got % rows', cnt;
    END IF;
END $$;

-- Test Case 3: Intruder / Non-member sees ZERO rows across all tenant tables
DO $$
DECLARE
    cnt INTEGER;
BEGIN
    -- Set auth context to Intruder
    PERFORM set_config('request.jwt.claim.sub', '44444444-4444-4444-4444-444444444444', true);

    SELECT COUNT(*) INTO cnt FROM public.organizations;
    IF cnt <> 0 THEN
        RAISE EXCEPTION 'RLS SECURITY VIOLATION: Non-member saw % organizations', cnt;
    END IF;

    SELECT COUNT(*) INTO cnt FROM public.branches;
    IF cnt <> 0 THEN
        RAISE EXCEPTION 'RLS SECURITY VIOLATION: Non-member saw % branches', cnt;
    END IF;

    SELECT COUNT(*) INTO cnt FROM public.registers;
    IF cnt <> 0 THEN
        RAISE EXCEPTION 'RLS SECURITY VIOLATION: Non-member saw % registers', cnt;
    END IF;

    SELECT COUNT(*) INTO cnt FROM public.users;
    IF cnt <> 0 THEN
        RAISE EXCEPTION 'RLS SECURITY VIOLATION: Non-member saw % users', cnt;
    END IF;
END $$;

-- Test Case 4: Helper functions behave deterministically
DO $$
BEGIN
    PERFORM set_config('request.jwt.claim.sub', '11111111-1111-1111-1111-111111111111', true);

    IF NOT public.is_org_member('aaaaaaaa-aaaa-aaaa-aaaa-aaaaaaaaaaaa') THEN
        RAISE EXCEPTION 'Helper is_org_member failed for valid member';
    END IF;

    IF public.is_org_member('bbbbbbbb-bbbb-bbbb-bbbb-bbbbbbbbbbbb') THEN
        RAISE EXCEPTION 'Helper is_org_member granted membership for foreign org';
    END IF;

    IF NOT public.is_org_admin_or_owner('aaaaaaaa-aaaa-aaaa-aaaa-aaaaaaaaaaaa') THEN
        RAISE EXCEPTION 'Helper is_org_admin_or_owner failed for owner';
    END IF;
END $$;

ROLLBACK;
