-- Supabase Test Suite: rls_policies_test.sql
-- F1.08 — Supabase RLS policies verification
-- Deterministic test assertions for Phase 1 RLS Tenant Isolation, Role Boundaries, and Sole-Owner Protection

BEGIN;

-- 1. Setup Standalone Roles and Grants if not running in preconfigured Supabase
DO $$
BEGIN
    IF NOT EXISTS (SELECT 1 FROM pg_roles WHERE rolname = 'authenticated') THEN
        CREATE ROLE authenticated NOLOGIN NOINHERIT;
    END IF;
    IF NOT EXISTS (SELECT 1 FROM pg_roles WHERE rolname = 'anon') THEN
        CREATE ROLE anon NOLOGIN NOINHERIT;
    END IF;
END $$;

-- 2. Standalone auth schema & auth.uid() function
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

-- Grant permissions to test roles
GRANT USAGE ON SCHEMA public, auth TO authenticated, anon;
GRANT ALL ON ALL TABLES IN SCHEMA public TO authenticated;
GRANT SELECT ON ALL TABLES IN SCHEMA auth TO authenticated, anon;
GRANT EXECUTE ON ALL FUNCTIONS IN SCHEMA public, auth TO authenticated, anon;

-- 3. Test Fixtures Setup (Run with table owner privileges)
DO $$
DECLARE
    user_a_owner UUID := '11111111-1111-1111-1111-111111111111';
    user_a_owner2 UUID := '12121212-1212-1212-1212-121212121212';
    user_a_manager UUID := '21212121-2121-2121-2121-212121212121';
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
        (user_a_owner2, 'owner_a2@tenant-a.com'),
        (user_a_manager, 'manager_a@tenant-a.com'),
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
        (org_a, user_a_manager, 'manager'),
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

-- 4. Deterministic Invariant Verifications Under REAL Authenticated Context

-- Test 1: standalone_auth_uid_context_is_valid
DO $$
BEGIN
    PERFORM set_config('request.jwt.claim.sub', '11111111-1111-1111-1111-111111111111', true);
    IF auth.uid() <> '11111111-1111-1111-1111-111111111111'::uuid THEN
        RAISE EXCEPTION 'auth.uid() did not resolve configured claim correctly';
    END IF;

    PERFORM set_config('request.jwt.claim.sub', '', true);
    IF auth.uid() IS NOT NULL THEN
        RAISE EXCEPTION 'auth.uid() must be NULL when claim is empty';
    END IF;
END $$;

-- Test 2: rls_executes_under_authenticated_role & rls_cross_tenant_isolation_is_enforced_under_authenticated_role
DO $$
DECLARE
    cnt INTEGER;
BEGIN
    -- Switch to authenticated role and User A Owner context
    SET LOCAL ROLE authenticated;
    PERFORM set_config('request.jwt.claim.sub', '11111111-1111-1111-1111-111111111111', true);

    -- Should see Org A
    SELECT COUNT(*) INTO cnt FROM public.organizations WHERE id = 'aaaaaaaa-aaaa-aaaa-aaaa-aaaaaaaaaaaa';
    IF cnt <> 1 THEN
        RAISE EXCEPTION 'RLS FAIL: User A Owner should see Organization A under authenticated role, got %', cnt;
    END IF;

    -- Must NOT see Org B
    SELECT COUNT(*) INTO cnt FROM public.organizations WHERE id = 'bbbbbbbb-bbbb-bbbb-bbbb-bbbbbbbbbbbb';
    IF cnt <> 0 THEN
        RAISE EXCEPTION 'RLS SECURITY VIOLATION: User A Owner saw Org B (% rows)', cnt;
    END IF;

    -- Must NOT see Branches, Registers, or Users in Org B
    SELECT COUNT(*) INTO cnt FROM public.branches WHERE organization_id = 'bbbbbbbb-bbbb-bbbb-bbbb-bbbbbbbbbbbb';
    IF cnt <> 0 THEN
        RAISE EXCEPTION 'RLS SECURITY VIOLATION: User A Owner saw branches in Org B (% rows)', cnt;
    END IF;

    SELECT COUNT(*) INTO cnt FROM public.registers WHERE organization_id = 'bbbbbbbb-bbbb-bbbb-bbbb-bbbbbbbbbbbb';
    IF cnt <> 0 THEN
        RAISE EXCEPTION 'RLS SECURITY VIOLATION: User A Owner saw registers in Org B (% rows)', cnt;
    END IF;

    SELECT COUNT(*) INTO cnt FROM public.users WHERE organization_id = 'bbbbbbbb-bbbb-bbbb-bbbb-bbbbbbbbbbbb';
    IF cnt <> 0 THEN
        RAISE EXCEPTION 'RLS SECURITY VIOLATION: User A Owner saw users in Org B (% rows)', cnt;
    END IF;
END $$;

-- Test 3: rls_non_owner_cannot_perform_owner_only_mutation
DO $$
DECLARE
    blocked BOOLEAN := false;
BEGIN
    SET LOCAL ROLE authenticated;
    PERFORM set_config('request.jwt.claim.sub', '22222222-2222-2222-2222-222222222222', true); -- Cashier A

    -- Cashier cannot create branches in Org A
    BEGIN
        INSERT INTO public.branches (id, organization_id, name, currency)
        VALUES ('a9a9a9a9-a9a9-a9a9-a9a9-a9a9a9a9a9a9', 'aaaaaaaa-aaaa-aaaa-aaaa-aaaaaaaaaaaa', 'Unauthorized Branch', 'USD');
    EXCEPTION WHEN OTHERS THEN
        blocked := true;
    END;

    -- If no exception was raised by RLS check, verify row was not inserted
    IF NOT blocked THEN
        IF EXISTS (SELECT 1 FROM public.branches WHERE id = 'a9a9a9a9-a9a9-a9a9-a9a9-a9a9a9a9a9a9') THEN
            RAISE EXCEPTION 'RLS SECURITY VIOLATION: Cashier successfully inserted branch into Org A';
        END IF;
    END IF;
END $$;

-- Test 4: cross_tenant_branch_mismatch_is_rejected
DO $$
DECLARE
    blocked_insert BOOLEAN := false;
    blocked_user_insert BOOLEAN := false;
BEGIN
    SET LOCAL ROLE authenticated;
    PERFORM set_config('request.jwt.claim.sub', '21212121-2121-2121-2121-212121212121', true); -- Manager A

    -- Manager A attempts to create a register in Org A linked to Branch B (which belongs to Org B)
    BEGIN
        INSERT INTO public.registers (id, organization_id, branch_id, name, code)
        VALUES ('a8a8a8a8-a8a8-a8a8-a8a8-a8a8a8a8a8a8', 'aaaaaaaa-aaaa-aaaa-aaaa-aaaaaaaaaaaa', 'b1b1b1b1-b1b1-b1b1-b1b1-b1b1b1b1b1b1', 'Mismatched Reg', 'REG-BAD');
    EXCEPTION WHEN OTHERS THEN
        blocked_insert := true;
    END;

    IF NOT blocked_insert THEN
        IF EXISTS (SELECT 1 FROM public.registers WHERE id = 'a8a8a8a8-a8a8-a8a8-a8a8-a8a8a8a8a8a8') THEN
            RAISE EXCEPTION 'RLS SECURITY VIOLATION: Manager A attached a register to Branch B across tenants!';
        END IF;
    END IF;

    -- Switch to Admin A context
    PERFORM set_config('request.jwt.claim.sub', '11111111-1111-1111-1111-111111111111', true);

    -- Admin A attempts to create a user in Org A linked to Branch B (which belongs to Org B)
    BEGIN
        INSERT INTO public.users (id, organization_id, branch_id, full_name, username, role)
        VALUES ('a7a7a7a7-a7a7-a7a7-a7a7-a7a7a7a7a7a7', 'aaaaaaaa-aaaa-aaaa-aaaa-aaaaaaaaaaaa', 'b1b1b1b1-b1b1-b1b1-b1b1-b1b1b1b1b1b1', 'Bad Branch User', 'bad_user', 'cashier');
    EXCEPTION WHEN OTHERS THEN
        blocked_user_insert := true;
    END;

    IF NOT blocked_user_insert THEN
        IF EXISTS (SELECT 1 FROM public.users WHERE id = 'a7a7a7a7-a7a7-a7a7-a7a7-a7a7a7a7a7a7') THEN
            RAISE EXCEPTION 'RLS SECURITY VIOLATION: Admin A attached a user to Branch B across tenants!';
        END IF;
    END IF;
END $$;

-- Test 5: sole_owner_cannot_delete_self
DO $$
DECLARE
    deleted_cnt INTEGER := 0;
    exception_caught BOOLEAN := false;
BEGIN
    SET LOCAL ROLE authenticated;
    PERFORM set_config('request.jwt.claim.sub', '11111111-1111-1111-1111-111111111111', true); -- Sole Owner of Org A

    BEGIN
        DELETE FROM public.organization_members
        WHERE organization_id = 'aaaaaaaa-aaaa-aaaa-aaaa-aaaaaaaaaaaa'
          AND user_id = '11111111-1111-1111-1111-111111111111';
        GET DIAGNOSTICS deleted_cnt = ROW_COUNT;
    EXCEPTION WHEN OTHERS THEN
        exception_caught := true;
    END;

    -- Sole owner deletion must have deleted 0 rows or thrown exception
    IF deleted_cnt > 0 THEN
        RAISE EXCEPTION 'RLS SECURITY VIOLATION: Sole owner was able to delete their own membership, leaving org orphaned!';
    END IF;

    -- Confirm owner still exists
    IF NOT EXISTS (
        SELECT 1 FROM public.organization_members
        WHERE organization_id = 'aaaaaaaa-aaaa-aaaa-aaaa-aaaaaaaaaaaa'
          AND user_id = '11111111-1111-1111-1111-111111111111'
          AND role = 'owner'
    ) THEN
        RAISE EXCEPTION 'RLS INVARIANT BROKEN: Sole owner membership was deleted';
    END IF;
END $$;

-- Test 6: owner_can_leave_when_another_owner_remains
DO $$
DECLARE
    deleted_cnt INTEGER := 0;
BEGIN
    -- First, add second owner (elevated fixture setup)
    RESET ROLE;
    INSERT INTO public.organization_members (organization_id, user_id, role)
    VALUES ('aaaaaaaa-aaaa-aaaa-aaaa-aaaaaaaaaaaa', '12121212-1212-1212-1212-121212121212', 'owner')
    ON CONFLICT (organization_id, user_id) DO UPDATE SET role = 'owner';

    -- Now run as second owner under authenticated role
    SET LOCAL ROLE authenticated;
    PERFORM set_config('request.jwt.claim.sub', '12121212-1212-1212-1212-121212121212', true);

    DELETE FROM public.organization_members
    WHERE organization_id = 'aaaaaaaa-aaaa-aaaa-aaaa-aaaaaaaaaaaa'
      AND user_id = '12121212-1212-1212-1212-121212121212';
    GET DIAGNOSTICS deleted_cnt = ROW_COUNT;

    IF deleted_cnt <> 1 THEN
        RAISE EXCEPTION 'RLS FAIL: Co-owner should be permitted to leave when another owner remains (deleted % rows)', deleted_cnt;
    END IF;

    -- Primary owner A must still be present
    IF NOT EXISTS (
        SELECT 1 FROM public.organization_members
        WHERE organization_id = 'aaaaaaaa-aaaa-aaaa-aaaa-aaaaaaaaaaaa'
          AND user_id = '11111111-1111-1111-1111-111111111111'
          AND role = 'owner'
    ) THEN
        RAISE EXCEPTION 'RLS INVARIANT BROKEN: Primary owner was unexpectedly deleted';
    END IF;
END $$;

-- Test 7: manager_and_cashier_policies_are_exercised_under_their_real_context
DO $$
DECLARE
    cnt INTEGER;
BEGIN
    -- Manager context
    SET LOCAL ROLE authenticated;
    PERFORM set_config('request.jwt.claim.sub', '21212121-2121-2121-2121-212121212121', true);

    -- Manager can insert registers in own org
    INSERT INTO public.registers (id, organization_id, branch_id, name, code)
    VALUES ('a5a5a5a5-a5a5-a5a5-a5a5-a5a5a5a5a5a5', 'aaaaaaaa-aaaa-aaaa-aaaa-aaaaaaaaaaaa', 'a1a1a1a1-a1a1-a1a1-a1a1-a1a1a1a1a1a1', 'Manager Added Reg', 'REG-MGR1')
    ON CONFLICT (branch_id, code) DO NOTHING;

    -- Verify manager's register exists
    SELECT COUNT(*) INTO cnt FROM public.registers WHERE id = 'a5a5a5a5-a5a5-a5a5-a5a5-a5a5a5a5a5a5';
    IF cnt <> 1 THEN
        RAISE EXCEPTION 'RLS FAIL: Manager was unable to insert register in own organization';
    END IF;

    -- Switch to anonymous role: must see 0 rows
    SET LOCAL ROLE anon;
    PERFORM set_config('request.jwt.claim.sub', '', true);

    SELECT COUNT(*) INTO cnt FROM public.organizations;
    IF cnt <> 0 THEN
        RAISE EXCEPTION 'RLS SECURITY VIOLATION: Anonymous role saw % organizations', cnt;
    END IF;
END $$;

ROLLBACK;
