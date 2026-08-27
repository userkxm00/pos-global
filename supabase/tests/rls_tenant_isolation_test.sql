-- Supabase Test Suite: rls_tenant_isolation_test.sql
-- F1.22 — RLS Tenant Isolation Verification Suite
-- Deterministic runtime verification of Row Level Security (RLS) policies across all 8 cloud tables,
-- all tenant roles (owner, admin, manager, cashier), unaffiliated authenticated users, and anonymous callers.

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
-- 2. Test Fixtures Setup (Elevated Initial Context)
-- ============================================================

DO $$
DECLARE
    owner_a UUID := '11111111-1111-1111-1111-111111111111';
    admin_a UUID := '12121212-1212-1212-1212-121212121212';
    manager_a UUID := '13131313-1313-1313-1313-131313131313';
    cashier_a UUID := '14141414-1414-1414-1414-141414141414';
    owner_b UUID := '22222222-2222-2222-2222-222222222222';
    unaffiliated UUID := '99999999-9999-9999-9999-999999999999';

    org_a UUID := 'f1220001-aaaa-aaaa-aaaa-000000000001';
    org_b UUID := 'f1220001-bbbb-bbbb-bbbb-000000000002';

    branch_a UUID := 'f1220002-aaaa-aaaa-aaaa-000000000001';
    branch_b UUID := 'f1220002-bbbb-bbbb-bbbb-000000000002';

    reg_a UUID := 'f1220003-aaaa-aaaa-aaaa-000000000001';
    reg_b UUID := 'f1220003-bbbb-bbbb-bbbb-000000000002';

    user_a_id UUID := 'f1220004-aaaa-aaaa-aaaa-000000000001';
    user_b_id UUID := 'f1220004-bbbb-bbbb-bbbb-000000000002';

    perm_id UUID;
BEGIN
    -- Auth users
    INSERT INTO auth.users (id, email) VALUES
        (owner_a, 'f122_owner_a@tenant-a.com'),
        (admin_a, 'f122_admin_a@tenant-a.com'),
        (manager_a, 'f122_manager_a@tenant-a.com'),
        (cashier_a, 'f122_cashier_a@tenant-a.com'),
        (owner_b, 'f122_owner_b@tenant-b.com'),
        (unaffiliated, 'f122_unaffiliated@nowhere.com')
    ON CONFLICT (id) DO NOTHING;

    -- Organizations
    INSERT INTO public.organizations (id, name, default_currency, default_language) VALUES
        (org_a, 'RLS Isolation Org A', 'USD', 'en'),
        (org_b, 'RLS Isolation Org B', 'EUR', 'de')
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
    INSERT INTO public.registers (id, organization_id, branch_id, name, code, is_active, device_identifier, device_pairing_status, device_paired_at) VALUES
        (reg_a, org_a, branch_a, 'Register A1', 'REG-A1', true, 'dev-a-terminal-01', 'paired', now()),
        (reg_b, org_b, branch_b, 'Register B1', 'REG-B1', true, 'dev-b-terminal-01', 'paired', now())
    ON CONFLICT (id) DO NOTHING;

    -- POS Users
    INSERT INTO public.users (id, organization_id, branch_id, supabase_user_id, full_name, username, role, is_active) VALUES
        (user_a_id, org_a, branch_a, cashier_a, 'Alice Cashier A', 'alice_c_a', 'cashier', true),
        (user_b_id, org_b, branch_b, owner_b, 'Bob Owner B', 'bob_o_b', 'admin', true)
    ON CONFLICT (id) DO NOTHING;

    -- Permissions & User Permissions Override
    SELECT id INTO perm_id FROM public.permissions WHERE code = 'sales.create' LIMIT 1;
    IF perm_id IS NOT NULL THEN
        INSERT INTO public.user_permissions (user_id, permission_id, effect) VALUES
            (user_a_id, perm_id, 'allow'),
            (user_b_id, perm_id, 'allow')
        ON CONFLICT (user_id, permission_id) DO NOTHING;
    END IF;
END $$;

-- ============================================================
-- 3. Cross-Tenant Read Isolation (SELECT across all tables)
-- ============================================================

DO $$
DECLARE
    owner_a UUID := '11111111-1111-1111-1111-111111111111';
    cashier_a UUID := '14141414-1414-1414-1414-141414141414';
    unaffiliated UUID := '99999999-9999-9999-9999-999999999999';

    org_a UUID := 'f1220001-aaaa-aaaa-aaaa-000000000001';
    org_b UUID := 'f1220001-bbbb-bbbb-bbbb-000000000002';
    cnt INTEGER;
BEGIN
    -- Case 1: Owner A Context
    SET LOCAL ROLE authenticated;
    PERFORM set_config('request.jwt.claim.sub', owner_a::text, true);

    -- Must see Org A, 0 rows of Org B
    SELECT COUNT(*) INTO cnt FROM public.organizations WHERE id = org_a;
    IF cnt <> 1 THEN RAISE EXCEPTION 'RLS FAIL: Owner A cannot see Org A'; END IF;

    SELECT COUNT(*) INTO cnt FROM public.organizations WHERE id = org_b;
    IF cnt <> 0 THEN RAISE EXCEPTION 'RLS SECURITY VIOLATION: Owner A saw Org B (% rows)', cnt; END IF;

    SELECT COUNT(*) INTO cnt FROM public.organization_members WHERE organization_id = org_b;
    IF cnt <> 0 THEN RAISE EXCEPTION 'RLS SECURITY VIOLATION: Owner A saw Org B members (% rows)', cnt; END IF;

    SELECT COUNT(*) INTO cnt FROM public.branches WHERE organization_id = org_b;
    IF cnt <> 0 THEN RAISE EXCEPTION 'RLS SECURITY VIOLATION: Owner A saw Org B branches (% rows)', cnt; END IF;

    SELECT COUNT(*) INTO cnt FROM public.registers WHERE organization_id = org_b;
    IF cnt <> 0 THEN RAISE EXCEPTION 'RLS SECURITY VIOLATION: Owner A saw Org B registers (% rows)', cnt; END IF;

    SELECT COUNT(*) INTO cnt FROM public.users WHERE organization_id = org_b;
    IF cnt <> 0 THEN RAISE EXCEPTION 'RLS SECURITY VIOLATION: Owner A saw Org B users (% rows)', cnt; END IF;

    -- User permissions: Owner A sees Org A user permissions, but 0 Org B user permissions
    SELECT COUNT(*) INTO cnt FROM public.user_permissions up
    JOIN public.users u ON u.id = up.user_id
    WHERE u.organization_id = org_b;
    IF cnt <> 0 THEN RAISE EXCEPTION 'RLS SECURITY VIOLATION: Owner A saw Org B user_permissions (% rows)', cnt; END IF;

    -- Case 2: Cashier A Context (Least Privileged Tenant Role)
    PERFORM set_config('request.jwt.claim.sub', cashier_a::text, true);

    SELECT COUNT(*) INTO cnt FROM public.organizations WHERE id = org_b;
    IF cnt <> 0 THEN RAISE EXCEPTION 'RLS SECURITY VIOLATION: Cashier A saw Org B (% rows)', cnt; END IF;

    SELECT COUNT(*) INTO cnt FROM public.branches WHERE organization_id = org_b;
    IF cnt <> 0 THEN RAISE EXCEPTION 'RLS SECURITY VIOLATION: Cashier A saw Org B branches (% rows)', cnt; END IF;

    SELECT COUNT(*) INTO cnt FROM public.registers WHERE organization_id = org_b;
    IF cnt <> 0 THEN RAISE EXCEPTION 'RLS SECURITY VIOLATION: Cashier A saw Org B registers (% rows)', cnt; END IF;

    SELECT COUNT(*) INTO cnt FROM public.users WHERE organization_id = org_b;
    IF cnt <> 0 THEN RAISE EXCEPTION 'RLS SECURITY VIOLATION: Cashier A saw Org B users (% rows)', cnt; END IF;

    -- Case 3: Unaffiliated Authenticated User (Zero Memberships)
    PERFORM set_config('request.jwt.claim.sub', unaffiliated::text, true);

    SELECT COUNT(*) INTO cnt FROM public.organizations WHERE id IN (org_a, org_b);
    IF cnt <> 0 THEN RAISE EXCEPTION 'RLS SECURITY VIOLATION: Unaffiliated user saw organizations (% rows)', cnt; END IF;

    SELECT COUNT(*) INTO cnt FROM public.branches WHERE organization_id IN (org_a, org_b);
    IF cnt <> 0 THEN RAISE EXCEPTION 'RLS SECURITY VIOLATION: Unaffiliated user saw branches (% rows)', cnt; END IF;

    SELECT COUNT(*) INTO cnt FROM public.registers WHERE organization_id IN (org_a, org_b);
    IF cnt <> 0 THEN RAISE EXCEPTION 'RLS SECURITY VIOLATION: Unaffiliated user saw registers (% rows)', cnt; END IF;

    SELECT COUNT(*) INTO cnt FROM public.users WHERE organization_id IN (org_a, org_b);
    IF cnt <> 0 THEN RAISE EXCEPTION 'RLS SECURITY VIOLATION: Unaffiliated user saw users (% rows)', cnt; END IF;

    -- Case 4: Anonymous Caller (anon role)
    SET LOCAL ROLE anon;
    PERFORM set_config('request.jwt.claim.sub', '', true);

    SELECT COUNT(*) INTO cnt FROM public.organizations;
    IF cnt <> 0 THEN RAISE EXCEPTION 'RLS SECURITY VIOLATION: Anon role saw organizations (% rows)', cnt; END IF;

    SELECT COUNT(*) INTO cnt FROM public.branches;
    IF cnt <> 0 THEN RAISE EXCEPTION 'RLS SECURITY VIOLATION: Anon role saw branches (% rows)', cnt; END IF;

    SELECT COUNT(*) INTO cnt FROM public.registers;
    IF cnt <> 0 THEN RAISE EXCEPTION 'RLS SECURITY VIOLATION: Anon role saw registers (% rows)', cnt; END IF;

    SELECT COUNT(*) INTO cnt FROM public.users;
    IF cnt <> 0 THEN RAISE EXCEPTION 'RLS SECURITY VIOLATION: Anon role saw users (% rows)', cnt; END IF;

    RAISE NOTICE 'PASS: Cross-tenant and unauthenticated read isolation verified across all tables';
END $$;

-- ============================================================
-- 4. Cross-Tenant Write & Mutation Isolation (INSERT, UPDATE, DELETE)
-- ============================================================

DO $$
DECLARE
    admin_a UUID := '12121212-1212-1212-1212-121212121212';
    manager_a UUID := '13131313-1313-1313-1313-131313131313';
    owner_b UUID := '22222222-2222-2222-2222-222222222222';

    org_a UUID := 'f1220001-aaaa-aaaa-aaaa-000000000001';
    org_b UUID := 'f1220001-bbbb-bbbb-bbbb-000000000002';
    branch_b UUID := 'f1220002-bbbb-bbbb-bbbb-000000000002';
    reg_b UUID := 'f1220003-bbbb-bbbb-bbbb-000000000002';
    user_b_id UUID := 'f1220004-bbbb-bbbb-bbbb-000000000002';

    perm_id UUID;
    mutated_cnt INTEGER := 0;
BEGIN
    SET LOCAL ROLE authenticated;
    PERFORM set_config('request.jwt.claim.sub', admin_a::text, true);

    -- 1. Cross-Tenant Branch INSERT: Admin A attempting to insert branch in Org B
    BEGIN
        INSERT INTO public.branches (id, organization_id, name, currency)
        VALUES ('f1220099-bbbb-bbbb-bbbb-000000000001', org_b, 'Rogue Branch in B', 'EUR');
        RAISE EXCEPTION 'RLS SECURITY VIOLATION: Admin A was permitted to insert branch into Org B' USING ERRCODE = 'TF001';
    EXCEPTION
        WHEN SQLSTATE 'TF001' THEN RAISE;
        WHEN insufficient_privilege THEN
            RAISE NOTICE 'PASS: Cross-tenant branch insert correctly denied by RLS policy';
    END;

    -- 2. Cross-Tenant Branch UPDATE: Admin A attempting to update branch in Org B
    UPDATE public.branches SET name = 'Attacked Branch B' WHERE id = branch_b;
    GET DIAGNOSTICS mutated_cnt = ROW_COUNT;
    IF mutated_cnt > 0 THEN
        RAISE EXCEPTION 'RLS SECURITY VIOLATION: Admin A updated % branch rows in Org B', mutated_cnt;
    END IF;

    -- 3. Cross-Tenant Branch DELETE: Admin A attempting to delete branch in Org B
    DELETE FROM public.branches WHERE id = branch_b;
    GET DIAGNOSTICS mutated_cnt = ROW_COUNT;
    IF mutated_cnt > 0 THEN
        RAISE EXCEPTION 'RLS SECURITY VIOLATION: Admin A deleted % branch rows in Org B', mutated_cnt;
    END IF;

    -- 4. Cross-Tenant Register INSERT: Manager A attempting to insert register in Org B
    PERFORM set_config('request.jwt.claim.sub', manager_a::text, true);
    BEGIN
        INSERT INTO public.registers (id, organization_id, branch_id, name, code)
        VALUES ('f1220099-bbbb-bbbb-bbbb-000000000002', org_b, branch_b, 'Rogue Reg in B', 'RREG-B');
        RAISE EXCEPTION 'RLS SECURITY VIOLATION: Manager A was permitted to insert register into Org B' USING ERRCODE = 'TF001';
    EXCEPTION
        WHEN SQLSTATE 'TF001' THEN RAISE;
        WHEN insufficient_privilege THEN
            RAISE NOTICE 'PASS: Cross-tenant register insert correctly denied by RLS policy';
    END;

    -- 5. Cross-Tenant Register UPDATE / Device Mutation: Manager A attempting to mutate register in Org B
    UPDATE public.registers SET name = 'Attacked Reg B', device_pairing_status = 'revoked' WHERE id = reg_b;
    GET DIAGNOSTICS mutated_cnt = ROW_COUNT;
    IF mutated_cnt > 0 THEN
        RAISE EXCEPTION 'RLS SECURITY VIOLATION: Manager A updated % register rows in Org B', mutated_cnt;
    END IF;

    -- 6. Cross-Tenant Register DELETE: Admin A attempting to delete register in Org B
    PERFORM set_config('request.jwt.claim.sub', admin_a::text, true);
    DELETE FROM public.registers WHERE id = reg_b;
    GET DIAGNOSTICS mutated_cnt = ROW_COUNT;
    IF mutated_cnt > 0 THEN
        RAISE EXCEPTION 'RLS SECURITY VIOLATION: Admin A deleted % register rows in Org B', mutated_cnt;
    END IF;

    -- 7. Cross-Tenant User INSERT: Admin A attempting to insert user in Org B (with valid supabase_user_id)
    BEGIN
        INSERT INTO public.users (id, organization_id, branch_id, supabase_user_id, full_name, username, role)
        VALUES ('f1220099-bbbb-bbbb-bbbb-000000000003', org_b, branch_b, admin_a, 'Rogue User B', 'rogue_b', 'cashier');
        RAISE EXCEPTION 'RLS SECURITY VIOLATION: Admin A was permitted to insert user into Org B' USING ERRCODE = 'TF001';
    EXCEPTION
        WHEN SQLSTATE 'TF001' THEN RAISE;
        WHEN insufficient_privilege THEN
            RAISE NOTICE 'PASS: Cross-tenant user insert correctly denied by RLS policy';
    END;

    -- 8. Cross-Tenant User UPDATE: Admin A attempting to update user in Org B
    UPDATE public.users SET full_name = 'Attacked User B' WHERE id = user_b_id;
    GET DIAGNOSTICS mutated_cnt = ROW_COUNT;
    IF mutated_cnt > 0 THEN
        RAISE EXCEPTION 'RLS SECURITY VIOLATION: Admin A updated % user rows in Org B', mutated_cnt;
    END IF;

    -- 9. Cross-Tenant User DELETE: Admin A attempting to delete user in Org B
    DELETE FROM public.users WHERE id = user_b_id;
    GET DIAGNOSTICS mutated_cnt = ROW_COUNT;
    IF mutated_cnt > 0 THEN
        RAISE EXCEPTION 'RLS SECURITY VIOLATION: Admin A deleted % user rows in Org B', mutated_cnt;
    END IF;

    -- 10. Cross-Tenant User Permission Mutation: Admin A attempting to mutate user_permissions in Org B
    SELECT id INTO perm_id FROM public.permissions WHERE code = 'sales.create' LIMIT 1;
    IF perm_id IS NOT NULL THEN
        UPDATE public.user_permissions SET effect = 'deny' WHERE user_id = user_b_id AND permission_id = perm_id;
        GET DIAGNOSTICS mutated_cnt = ROW_COUNT;
        IF mutated_cnt > 0 THEN
            RAISE EXCEPTION 'RLS SECURITY VIOLATION: Admin A mutated % user_permission rows in Org B', mutated_cnt;
        END IF;
    END IF;

    RAISE NOTICE 'PASS: Cross-tenant write and mutation isolation verified across all resources';
END $$;

-- ============================================================
-- 5. Intra-Tenant Role Hierarchy & Privilege Boundaries
-- ============================================================

DO $$
DECLARE
    cashier_a UUID := '14141414-1414-1414-1414-141414141414';
    manager_a UUID := '13131313-1313-1313-1313-131313131313';
    admin_a UUID := '12121212-1212-1212-1212-121212121212';
    owner_a UUID := '11111111-1111-1111-1111-111111111111';

    org_a UUID := 'f1220001-aaaa-aaaa-aaaa-000000000001';
    branch_a UUID := 'f1220002-aaaa-aaaa-aaaa-000000000001';
    reg_a UUID := 'f1220003-aaaa-aaaa-aaaa-000000000001';
    user_a_id UUID := 'f1220004-aaaa-aaaa-aaaa-000000000001';

    perm_id UUID;
    mutated_cnt INTEGER := 0;
BEGIN
    -- 1. Cashier Role Boundaries:
    SET LOCAL ROLE authenticated;
    PERFORM set_config('request.jwt.claim.sub', cashier_a::text, true);

    -- Cashier cannot insert branches
    BEGIN
        INSERT INTO public.branches (id, organization_id, name, currency)
        VALUES ('f1220099-aaaa-aaaa-aaaa-000000000001', org_a, 'Cashier Branch', 'USD');
        RAISE EXCEPTION 'RLS SECURITY VIOLATION: Cashier was permitted to create branch' USING ERRCODE = 'TF001';
    EXCEPTION
        WHEN SQLSTATE 'TF001' THEN RAISE;
        WHEN insufficient_privilege THEN
            RAISE NOTICE 'PASS: Cashier branch creation correctly denied by RLS policy';
    END;

    -- Cashier cannot insert registers
    BEGIN
        INSERT INTO public.registers (id, organization_id, branch_id, name, code)
        VALUES ('f1220099-aaaa-aaaa-aaaa-000000000002', org_a, branch_a, 'Cashier Reg', 'CREG-1');
        RAISE EXCEPTION 'RLS SECURITY VIOLATION: Cashier was permitted to create register' USING ERRCODE = 'TF001';
    EXCEPTION
        WHEN SQLSTATE 'TF001' THEN RAISE;
        WHEN insufficient_privilege THEN
            RAISE NOTICE 'PASS: Cashier register creation correctly denied by RLS policy';
    END;

    -- Cashier cannot update register / device pairing
    UPDATE public.registers SET device_pairing_status = 'revoked' WHERE id = reg_a;
    GET DIAGNOSTICS mutated_cnt = ROW_COUNT;
    IF mutated_cnt > 0 THEN RAISE EXCEPTION 'RLS SECURITY VIOLATION: Cashier updated register'; END IF;

    -- Cashier cannot insert users (with valid supabase_user_id)
    BEGIN
        INSERT INTO public.users (id, organization_id, branch_id, supabase_user_id, full_name, username, role)
        VALUES ('f1220099-aaaa-aaaa-aaaa-000000000003', org_a, branch_a, cashier_a, 'Cashier User', 'c_user', 'cashier');
        RAISE EXCEPTION 'RLS SECURITY VIOLATION: Cashier was permitted to create user' USING ERRCODE = 'TF001';
    EXCEPTION
        WHEN SQLSTATE 'TF001' THEN RAISE;
        WHEN insufficient_privilege THEN
            RAISE NOTICE 'PASS: Cashier user creation correctly denied by RLS policy';
    END;

    -- Cashier cannot mutate user_permissions
    SELECT id INTO perm_id FROM public.permissions WHERE code = 'sales.create' LIMIT 1;
    IF perm_id IS NOT NULL THEN
        UPDATE public.user_permissions SET effect = 'deny' WHERE user_id = user_a_id AND permission_id = perm_id;
        GET DIAGNOSTICS mutated_cnt = ROW_COUNT;
        IF mutated_cnt > 0 THEN RAISE EXCEPTION 'RLS SECURITY VIOLATION: Cashier mutated user_permissions'; END IF;
    END IF;

    -- 2. Manager Role Boundaries:
    PERFORM set_config('request.jwt.claim.sub', manager_a::text, true);

    -- Manager cannot insert branches
    BEGIN
        INSERT INTO public.branches (id, organization_id, name, currency)
        VALUES ('f1220099-aaaa-aaaa-aaaa-000000000004', org_a, 'Manager Branch', 'USD');
        RAISE EXCEPTION 'RLS SECURITY VIOLATION: Manager was permitted to create branch' USING ERRCODE = 'TF001';
    EXCEPTION
        WHEN SQLSTATE 'TF001' THEN RAISE;
        WHEN insufficient_privilege THEN
            RAISE NOTICE 'PASS: Manager branch creation correctly denied by RLS policy';
    END;

    -- Manager CAN insert registers in own branch (explicitly verified)
    INSERT INTO public.registers (id, organization_id, branch_id, name, code)
    VALUES ('f1220099-aaaa-aaaa-aaaa-000000000005', org_a, branch_a, 'Manager Reg', 'MREG-1');
    GET DIAGNOSTICS mutated_cnt = ROW_COUNT;
    IF mutated_cnt <> 1 THEN
        RAISE EXCEPTION 'RLS FAIL: Manager was unable to insert register in own branch (inserted % rows)', mutated_cnt;
    END IF;

    -- Manager CANNOT delete registers
    DELETE FROM public.registers WHERE id = 'f1220099-aaaa-aaaa-aaaa-000000000005';
    GET DIAGNOSTICS mutated_cnt = ROW_COUNT;
    IF mutated_cnt > 0 THEN RAISE EXCEPTION 'RLS SECURITY VIOLATION: Manager deleted register'; END IF;

    -- 3. Admin Role Boundaries:
    PERFORM set_config('request.jwt.claim.sub', admin_a::text, true);

    -- Admin CAN delete register created by Manager
    DELETE FROM public.registers WHERE id = 'f1220099-aaaa-aaaa-aaaa-000000000005';
    GET DIAGNOSTICS mutated_cnt = ROW_COUNT;
    IF mutated_cnt <> 1 THEN RAISE EXCEPTION 'RLS FAIL: Admin was unable to delete register'; END IF;

    -- Admin CANNOT delete organization (Owner-only privilege)
    DELETE FROM public.organizations WHERE id = org_a;
    GET DIAGNOSTICS mutated_cnt = ROW_COUNT;
    IF mutated_cnt > 0 THEN RAISE EXCEPTION 'RLS SECURITY VIOLATION: Admin deleted organization'; END IF;

    -- 4. Owner Role Verification:
    PERFORM set_config('request.jwt.claim.sub', owner_a::text, true);
    -- Confirm Org A still exists and owner has full authority
    SELECT COUNT(*) INTO mutated_cnt FROM public.organizations WHERE id = org_a;
    IF mutated_cnt <> 1 THEN RAISE EXCEPTION 'RLS INVARIANT BROKEN: Org A missing'; END IF;

    RAISE NOTICE 'PASS: Intra-tenant role hierarchy and privilege boundaries verified';
END $$;

-- ============================================================
-- 6. Global Catalog Access Policies (Permissions & Role Permissions)
-- ============================================================

DO $$
DECLARE
    owner_a UUID := '11111111-1111-1111-1111-111111111111';
    cnt INTEGER;
BEGIN
    -- Authenticated user can read permissions catalog
    SET LOCAL ROLE authenticated;
    PERFORM set_config('request.jwt.claim.sub', owner_a::text, true);

    SELECT COUNT(*) INTO cnt FROM public.permissions;
    IF cnt = 0 THEN RAISE EXCEPTION 'RLS FAIL: Authenticated user cannot read permissions catalog'; END IF;

    SELECT COUNT(*) INTO cnt FROM public.role_permissions;
    IF cnt = 0 THEN RAISE EXCEPTION 'RLS FAIL: Authenticated user cannot read role_permissions catalog'; END IF;

    -- Anonymous caller cannot read catalog without authentication
    SET LOCAL ROLE anon;
    PERFORM set_config('request.jwt.claim.sub', '', true);

    SELECT COUNT(*) INTO cnt FROM public.permissions;
    IF cnt <> 0 THEN RAISE EXCEPTION 'RLS SECURITY VIOLATION: Anon role saw % permissions', cnt; END IF;

    SELECT COUNT(*) INTO cnt FROM public.role_permissions;
    IF cnt <> 0 THEN RAISE EXCEPTION 'RLS SECURITY VIOLATION: Anon role saw % role_permissions', cnt; END IF;

    RAISE NOTICE 'PASS: Global catalog access policies verified';
END $$;

ROLLBACK;
