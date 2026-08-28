-- Supabase Test Suite: cross_tenant_negative_test.sql
-- F1.25 — Cross-Tenant Negative Tests
-- Dedicated adversarial runtime verification of multi-tenant isolation, composite foreign-key
-- integrity constraints, device identity pairing invariants, privileged RPC authorization gates,
-- role/permission boundaries, and anonymous/unaffiliated penetration resistance.

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
GRANT EXECUTE ON ALL FUNCTIONS IN SCHEMA auth TO authenticated, anon;
GRANT EXECUTE ON ALL FUNCTIONS IN SCHEMA public TO authenticated;

-- ============================================================
-- 2. Test Fixtures Setup (Elevated Initial Context)
-- ============================================================

DO $$
DECLARE
    -- Tenant A Auth Users
    owner_a UUID := '11111111-1111-1111-1111-111111111111';
    admin_a UUID := '12121212-1212-1212-1212-121212121212';
    manager_a UUID := '13131313-1313-1313-1313-131313131313';
    cashier_a UUID := '14141414-1414-1414-1414-141414141414';

    -- Tenant B Auth Users
    owner_b UUID := '22222222-2222-2222-2222-222222222222';
    admin_b UUID := '23232323-2323-2323-2323-232323232323';
    manager_b UUID := '24242424-2424-2424-2424-242424242424';
    cashier_b UUID := '25252525-2525-2525-2525-252525252525';

    -- Unaffiliated Auth User
    unaffiliated UUID := '99999999-9999-9999-9999-999999999999';

    -- Tenant A Resources
    org_a UUID := 'f1250001-aaaa-aaaa-aaaa-000000000001';
    branch_a UUID := 'f1250002-aaaa-aaaa-aaaa-000000000001';
    reg_a_paired UUID := 'f1250003-aaaa-aaaa-aaaa-000000000001';
    reg_a_unpaired UUID := 'f1250003-aaaa-aaaa-aaaa-000000000002';
    user_a_id UUID := 'f1250004-aaaa-aaaa-aaaa-000000000001';

    -- Tenant B Resources
    org_b UUID := 'f1250001-bbbb-bbbb-bbbb-000000000002';
    branch_b UUID := 'f1250002-bbbb-bbbb-bbbb-000000000002';
    reg_b_paired UUID := 'f1250003-bbbb-bbbb-bbbb-000000000001';
    reg_b_unpaired UUID := 'f1250003-bbbb-bbbb-bbbb-000000000002';
    user_b_id UUID := 'f1250004-bbbb-bbbb-bbbb-000000000002';

    perm_id UUID := 'f1250005-aaaa-aaaa-aaaa-000000000001';
BEGIN
    -- Auth Users
    INSERT INTO auth.users (id, email) VALUES
        (owner_a, 'f125_owner_a@tenant-a.com'),
        (admin_a, 'f125_admin_a@tenant-a.com'),
        (manager_a, 'f125_manager_a@tenant-a.com'),
        (cashier_a, 'f125_cashier_a@tenant-a.com'),
        (owner_b, 'f125_owner_b@tenant-b.com'),
        (admin_b, 'f125_admin_b@tenant-b.com'),
        (manager_b, 'f125_manager_b@tenant-b.com'),
        (cashier_b, 'f125_cashier_b@tenant-b.com'),
        (unaffiliated, 'f125_unaffiliated@nowhere.com')
    ON CONFLICT (id) DO NOTHING;

    -- Organizations
    INSERT INTO public.organizations (id, name, default_currency, default_language) VALUES
        (org_a, 'Tenant Alpha Organization', 'USD', 'en'),
        (org_b, 'Tenant Beta Organization', 'EUR', 'de')
    ON CONFLICT (id) DO NOTHING;

    -- Organization Memberships
    INSERT INTO public.organization_members (organization_id, user_id, role) VALUES
        (org_a, owner_a, 'owner'),
        (org_a, admin_a, 'admin'),
        (org_a, manager_a, 'manager'),
        (org_a, cashier_a, 'cashier'),
        (org_b, owner_b, 'owner'),
        (org_b, admin_b, 'admin'),
        (org_b, manager_b, 'manager'),
        (org_b, cashier_b, 'cashier')
    ON CONFLICT (organization_id, user_id) DO NOTHING;

    -- Branches
    INSERT INTO public.branches (id, organization_id, name, currency, is_active) VALUES
        (branch_a, org_a, 'Branch Alpha Main', 'USD', true),
        (branch_b, org_b, 'Branch Beta Main', 'EUR', true)
    ON CONFLICT (id) DO NOTHING;

    -- Registers (Paired and Unpaired for both tenants)
    INSERT INTO public.registers (id, organization_id, branch_id, name, code, is_active, device_identifier, device_pairing_status, device_paired_at, device_last_seen_at) VALUES
        (reg_a_paired, org_a, branch_a, 'Reg A Paired', 'REG-A-01', true, 'dev-alpha-active-01', 'paired', now(), now()),
        (reg_a_unpaired, org_a, branch_a, 'Reg A Unpaired', 'REG-A-02', true, NULL, 'unpaired', NULL, NULL),
        (reg_b_paired, org_b, branch_b, 'Reg B Paired', 'REG-B-01', true, 'dev-beta-active-01', 'paired', now(), now()),
        (reg_b_unpaired, org_b, branch_b, 'Reg B Unpaired', 'REG-B-02', true, NULL, 'unpaired', NULL, NULL)
    ON CONFLICT (id) DO NOTHING;

    -- POS Staff Users
    INSERT INTO public.users (id, organization_id, branch_id, supabase_user_id, full_name, username, role, is_active) VALUES
        (user_a_id, org_a, branch_a, cashier_a, 'Alice Cashier Alpha', 'alice_a', 'cashier', true),
        (user_b_id, org_b, branch_b, cashier_b, 'Bob Cashier Beta', 'bob_b', 'cashier', true)
    ON CONFLICT (id) DO NOTHING;

    -- Permissions Catalog & Role Permissions & User Permissions Override Fixture
    INSERT INTO public.permissions (id, code, description) VALUES
        (perm_id, 'sales.create', 'Permission to create sales transactions')
    ON CONFLICT (code) DO UPDATE SET description = EXCLUDED.description
    RETURNING id INTO perm_id;

    IF perm_id IS NULL THEN
        RAISE EXCEPTION 'FIXTURE ERROR: Failed to establish sales.create permission record';
    END IF;

    INSERT INTO public.role_permissions (role, permission_id) VALUES
        ('cashier', perm_id)
    ON CONFLICT (role, permission_id) DO NOTHING;

    INSERT INTO public.user_permissions (user_id, permission_id, effect) VALUES
        (user_a_id, perm_id, 'allow'),
        (user_b_id, perm_id, 'allow')
    ON CONFLICT (user_id, permission_id) DO NOTHING;
END $$;

-- ============================================================
-- Suite 1: Composite Foreign-Key & Tenant Boundary Invariants (N01–N04)
-- Executed in elevated (table owner) context to specifically verify composite FK rejection (23503)
-- ============================================================

DO $$
DECLARE
    org_a UUID := 'f1250001-aaaa-aaaa-aaaa-000000000001';
    branch_b UUID := 'f1250002-bbbb-bbbb-bbbb-000000000002';
    user_a_id UUID := 'f1250004-aaaa-aaaa-aaaa-000000000001';
    reg_a_paired UUID := 'f1250003-aaaa-aaaa-aaaa-000000000001';
BEGIN
    -- N01: Attempting to insert a user with Org A and Branch B (belonging to Org B)
    BEGIN
        INSERT INTO public.users (id, organization_id, branch_id, full_name, username, role)
        VALUES ('f1250099-aaaa-aaaa-aaaa-000000000001', org_a, branch_b, 'Mismatch User', 'mismatch_u', 'cashier');
        RAISE EXCEPTION 'COMPOSITE FK VIOLATION: User insert with mismatched Org A / Branch B succeeded' USING ERRCODE = 'TF001';
    EXCEPTION
        WHEN SQLSTATE 'TF001' THEN RAISE;
        WHEN foreign_key_violation THEN
            RAISE NOTICE 'PASS N01: User cross-tenant branch-org mismatch blocked by composite FK (fk_users_branch_org)';
    END;

    -- N02: Attempting to insert a register with Org A and Branch B (belonging to Org B)
    BEGIN
        INSERT INTO public.registers (id, organization_id, branch_id, name, code)
        VALUES ('f1250099-aaaa-aaaa-aaaa-000000000002', org_a, branch_b, 'Mismatch Reg', 'REG-MISM-1');
        RAISE EXCEPTION 'COMPOSITE FK VIOLATION: Register insert with mismatched Org A / Branch B succeeded' USING ERRCODE = 'TF001';
    EXCEPTION
        WHEN SQLSTATE 'TF001' THEN RAISE;
        WHEN foreign_key_violation THEN
            RAISE NOTICE 'PASS N02: Register cross-tenant branch-org mismatch blocked by composite FK (fk_registers_branch_org)';
    END;

    -- N03: Attempting to update existing User A branch_id to Branch B (belonging to Org B)
    BEGIN
        UPDATE public.users SET branch_id = branch_b WHERE id = user_a_id;
        RAISE EXCEPTION 'COMPOSITE FK VIOLATION: User branch update to cross-tenant Branch B succeeded' USING ERRCODE = 'TF001';
    EXCEPTION
        WHEN SQLSTATE 'TF001' THEN RAISE;
        WHEN foreign_key_violation THEN
            RAISE NOTICE 'PASS N03: User cross-tenant branch reassignment blocked by composite FK (fk_users_branch_org)';
    END;

    -- N04: Attempting to update existing Register A branch_id to Branch B (belonging to Org B)
    BEGIN
        UPDATE public.registers SET branch_id = branch_b WHERE id = reg_a_paired;
        RAISE EXCEPTION 'COMPOSITE FK VIOLATION: Register branch update to cross-tenant Branch B succeeded' USING ERRCODE = 'TF001';
    EXCEPTION
        WHEN SQLSTATE 'TF001' THEN RAISE;
        WHEN foreign_key_violation THEN
            RAISE NOTICE 'PASS N04: Register cross-tenant branch reassignment blocked by composite FK (fk_registers_branch_org)';
    END;
END $$;

-- ============================================================
-- Suite 2: Cross-Tenant Entity Reassignment & Boundary Jumping (N05–N08)
-- Executed under authenticated tenant identity to verify RLS isolation & WITH CHECK guards
-- ============================================================

DO $$
DECLARE
    admin_a UUID := '12121212-1212-1212-1212-121212121212';
    manager_a UUID := '13131313-1313-1313-1313-131313131313';
    owner_a UUID := '11111111-1111-1111-1111-111111111111';

    org_a UUID := 'f1250001-aaaa-aaaa-aaaa-000000000001';
    org_b UUID := 'f1250001-bbbb-bbbb-bbbb-000000000002';
    branch_a UUID := 'f1250002-aaaa-aaaa-aaaa-000000000001';
    reg_a_paired UUID := 'f1250003-aaaa-aaaa-aaaa-000000000001';
    user_a_id UUID := 'f1250004-aaaa-aaaa-aaaa-000000000001';

    mutated_cnt INTEGER := 0;
    curr_org UUID;
BEGIN
    SET LOCAL ROLE authenticated;

    -- N05: Admin A attempts to transfer branch ownership from Org A to Org B
    PERFORM set_config('request.jwt.claim.sub', admin_a::text, true);
    BEGIN
        UPDATE public.branches SET organization_id = org_b WHERE id = branch_a;
        GET DIAGNOSTICS mutated_cnt = ROW_COUNT;
        IF mutated_cnt > 0 THEN
            RAISE EXCEPTION 'RLS SECURITY VIOLATION: Admin A transferred branch to Org B (% rows)', mutated_cnt USING ERRCODE = 'TF001';
        END IF;
        RAISE NOTICE 'PASS N05: Branch organization hijack blocked (0 rows mutated)';
    EXCEPTION
        WHEN SQLSTATE 'TF001' THEN RAISE;
        WHEN insufficient_privilege THEN
            RAISE NOTICE 'PASS N05: Branch organization hijack rejected by RLS WITH CHECK policy';
    END;

    -- Verify Branch A organization_id remains strictly Org A
    SELECT organization_id INTO curr_org FROM public.branches WHERE id = branch_a;
    IF curr_org <> org_a THEN
        RAISE EXCEPTION 'INVARIANT BROKEN: Branch A organization_id was corrupted to %', curr_org;
    END IF;

    -- N06: Manager A attempts to transfer register ownership from Org A to Org B
    PERFORM set_config('request.jwt.claim.sub', manager_a::text, true);
    BEGIN
        UPDATE public.registers SET organization_id = org_b WHERE id = reg_a_paired;
        GET DIAGNOSTICS mutated_cnt = ROW_COUNT;
        IF mutated_cnt > 0 THEN
            RAISE EXCEPTION 'RLS SECURITY VIOLATION: Manager A transferred register to Org B (% rows)', mutated_cnt USING ERRCODE = 'TF001';
        END IF;
        RAISE NOTICE 'PASS N06: Register organization hijack blocked (0 rows mutated)';
    EXCEPTION
        WHEN SQLSTATE 'TF001' THEN RAISE;
        WHEN insufficient_privilege THEN
            RAISE NOTICE 'PASS N06: Register organization hijack rejected by RLS WITH CHECK policy';
    END;

    -- N07: Admin A attempts to transfer POS user ownership from Org A to Org B
    PERFORM set_config('request.jwt.claim.sub', admin_a::text, true);
    BEGIN
        UPDATE public.users SET organization_id = org_b WHERE id = user_a_id;
        GET DIAGNOSTICS mutated_cnt = ROW_COUNT;
        IF mutated_cnt > 0 THEN
            RAISE EXCEPTION 'RLS SECURITY VIOLATION: Admin A transferred user to Org B (% rows)', mutated_cnt USING ERRCODE = 'TF001';
        END IF;
        RAISE NOTICE 'PASS N07: User organization hijack blocked (0 rows mutated)';
    EXCEPTION
        WHEN SQLSTATE 'TF001' THEN RAISE;
        WHEN insufficient_privilege THEN
            RAISE NOTICE 'PASS N07: User organization hijack rejected by RLS WITH CHECK policy';
    END;

    -- N08: Owner A attempts to reassign organization_members record to Org B
    PERFORM set_config('request.jwt.claim.sub', owner_a::text, true);
    BEGIN
        UPDATE public.organization_members SET organization_id = org_b WHERE organization_id = org_a AND user_id = admin_a;
        GET DIAGNOSTICS mutated_cnt = ROW_COUNT;
        IF mutated_cnt > 0 THEN
            RAISE EXCEPTION 'RLS SECURITY VIOLATION: Owner A reassigned member to Org B (% rows)', mutated_cnt USING ERRCODE = 'TF001';
        END IF;
        RAISE NOTICE 'PASS N08: Organization member reassignment blocked (0 rows mutated)';
    EXCEPTION
        WHEN SQLSTATE 'TF001' THEN RAISE;
        WHEN insufficient_privilege THEN
            RAISE NOTICE 'PASS N08: Organization member reassignment rejected by RLS policy';
    END;
END $$;

-- ============================================================
-- Suite 3: Cross-Tenant Device Identity, Hijacking & Active Reuse (N09–N13)
-- Executed under authenticated tenant identity to verify device constraints and RPC guards
-- ============================================================

DO $$
DECLARE
    manager_a UUID := '13131313-1313-1313-1313-131313131313';
    admin_a UUID := '12121212-1212-1212-1212-121212121212';
    unaffiliated UUID := '99999999-9999-9999-9999-999999999999';
    member_b UUID := '25252525-2525-2525-2525-252525252525';

    reg_a_unpaired UUID := 'f1250003-aaaa-aaaa-aaaa-000000000002';
    reg_b_paired UUID := 'f1250003-bbbb-bbbb-bbbb-000000000001';
    reg_b_unpaired UUID := 'f1250003-bbbb-bbbb-bbbb-000000000002';

    mutated_cnt INTEGER := 0;
    initial_last_seen TIMESTAMPTZ;
    current_last_seen TIMESTAMPTZ;
    curr_device TEXT;
    curr_status TEXT;
BEGIN
    -- Baseline verification in elevated context
    SELECT device_last_seen_at INTO initial_last_seen FROM public.registers WHERE id = reg_b_paired;
    IF initial_last_seen IS NULL THEN
        RAISE EXCEPTION 'FIXTURE ERROR: initial_last_seen for reg_b_paired is NULL';
    END IF;

    SET LOCAL ROLE authenticated;

    -- N09: Cross-tenant device identifier collision on RPC pairing (device is active on Tenant B)
    PERFORM set_config('request.jwt.claim.sub', manager_a::text, true);
    BEGIN
        PERFORM public.pair_device_to_register(reg_a_unpaired, 'dev-beta-active-01');
        RAISE EXCEPTION 'DEVICE SECURITY VIOLATION: Paired device active in Tenant B to Tenant A register' USING ERRCODE = 'TF001';
    EXCEPTION
        WHEN SQLSTATE 'TF001' THEN RAISE;
        WHEN unique_violation THEN
            RAISE NOTICE 'PASS N09: Cross-tenant active device collision rejected by unique index (uq_registers_global_active_device)';
    END;

    -- N10: Manager A calling pair_device_to_register targeting Tenant B's register
    BEGIN
        PERFORM public.pair_device_to_register(reg_b_unpaired, 'dev-alpha-fresh-01');
        RAISE EXCEPTION 'RPC SECURITY VIOLATION: Manager A paired device to Tenant B register' USING ERRCODE = 'TF001';
    EXCEPTION
        WHEN SQLSTATE 'TF001' THEN RAISE;
        WHEN SQLSTATE '42501' THEN
            RAISE NOTICE 'PASS N10: Cross-tenant device pairing RPC rejected with 42501 Insufficient privilege';
    END;

    -- N11: Admin A calling revoke_device_pairing targeting Tenant B's register
    PERFORM set_config('request.jwt.claim.sub', admin_a::text, true);
    BEGIN
        PERFORM public.revoke_device_pairing(reg_b_paired);
        RAISE EXCEPTION 'RPC SECURITY VIOLATION: Admin A revoked device pairing on Tenant B register' USING ERRCODE = 'TF001';
    EXCEPTION
        WHEN SQLSTATE 'TF001' THEN RAISE;
        WHEN SQLSTATE '42501' THEN
            RAISE NOTICE 'PASS N11: Cross-tenant device revocation RPC rejected with 42501 Insufficient privilege';
    END;

    -- N12: Device Heartbeat Spoofing across Tenants
    -- Sub-test 12a: Unaffiliated user calls heartbeat on Tenant B register
    PERFORM set_config('request.jwt.claim.sub', unaffiliated::text, true);
    BEGIN
        PERFORM public.record_device_heartbeat(reg_b_paired, 'dev-beta-active-01');
        RAISE EXCEPTION 'HEARTBEAT VIOLATION: Unaffiliated user recorded heartbeat on Tenant B register' USING ERRCODE = 'TF001';
    EXCEPTION
        WHEN SQLSTATE 'TF001' THEN RAISE;
        WHEN SQLSTATE '42501' THEN
            RAISE NOTICE 'PASS N12a: Unaffiliated heartbeat rejected with 42501 Insufficient privilege';
    END;

    -- Sub-test 12b: Tenant A user calls heartbeat on Tenant B register
    PERFORM set_config('request.jwt.claim.sub', admin_a::text, true);
    BEGIN
        PERFORM public.record_device_heartbeat(reg_b_paired, 'dev-alpha-active-01');
        RAISE EXCEPTION 'HEARTBEAT VIOLATION: Tenant A user recorded heartbeat on Tenant B register' USING ERRCODE = 'TF001';
    EXCEPTION
        WHEN SQLSTATE 'TF001' THEN RAISE;
        WHEN SQLSTATE '42501' THEN
            RAISE NOTICE 'PASS N12b: Cross-tenant member heartbeat rejected with 42501 Insufficient privilege';
    END;

    -- Sub-test 12c: Tenant B member presents mismatched device identifier
    PERFORM set_config('request.jwt.claim.sub', member_b::text, true);
    BEGIN
        PERFORM public.record_device_heartbeat(reg_b_paired, 'dev-spoofed-device-id');
        RAISE EXCEPTION 'HEARTBEAT VIOLATION: Mismatched device identifier accepted for heartbeat' USING ERRCODE = 'TF001';
    EXCEPTION
        WHEN SQLSTATE 'TF001' THEN RAISE;
        WHEN SQLSTATE '42501' THEN
            RAISE NOTICE 'PASS N12c: Mismatched device identifier heartbeat rejected with 42501 Mismatch';
    END;

    -- Reset to elevated role to verify Tenant B timestamp was untouched
    RESET ROLE;
    SELECT device_last_seen_at INTO current_last_seen FROM public.registers WHERE id = reg_b_paired;
    IF current_last_seen IS DISTINCT FROM initial_last_seen THEN
        RAISE EXCEPTION 'HEARTBEAT INVARIANT BROKEN: Target register device_last_seen_at was altered by spoofed heartbeat (initial: %, current: %)', initial_last_seen, current_last_seen;
    END IF;

    -- N13: Direct SQL manipulation of foreign tenant register pairing state
    SET LOCAL ROLE authenticated;
    PERFORM set_config('request.jwt.claim.sub', admin_a::text, true);
    BEGIN
        UPDATE public.registers SET device_pairing_status = 'revoked', device_identifier = NULL WHERE id = reg_b_paired;
        GET DIAGNOSTICS mutated_cnt = ROW_COUNT;
        IF mutated_cnt > 0 THEN
            RAISE EXCEPTION 'RLS SECURITY VIOLATION: Admin A mutated % registers in Tenant B via direct SQL', mutated_cnt USING ERRCODE = 'TF001';
        END IF;
        RAISE NOTICE 'PASS N13: Direct SQL manipulation of foreign tenant register pairing state blocked by RLS (0 rows updated)';
    EXCEPTION
        WHEN SQLSTATE 'TF001' THEN RAISE;
        WHEN insufficient_privilege THEN
            RAISE NOTICE 'PASS N13: Direct SQL manipulation of foreign tenant register rejected by RLS policy';
    END;

    -- Verify Tenant B register state in elevated context
    RESET ROLE;
    SELECT device_pairing_status, device_identifier INTO curr_status, curr_device FROM public.registers WHERE id = reg_b_paired;
    IF curr_status <> 'paired' OR curr_device <> 'dev-beta-active-01' THEN
        RAISE EXCEPTION 'INVARIANT BROKEN: Tenant B register was mutated by unauthorized caller';
    END IF;
END $$;

-- ============================================================
-- Suite 4: Cross-Tenant Privileged RPC Attacks & Parameter Injection (N14–N18)
-- Executed under authenticated tenant identity verifying function authorization & parameter isolation
-- ============================================================

DO $$
DECLARE
    owner_a UUID := '11111111-1111-1111-1111-111111111111';
    manager_a UUID := '13131313-1313-1313-1313-131313131313';
    owner_b UUID := '22222222-2222-2222-2222-222222222222';
    admin_b UUID := '23232323-2323-2323-2323-232323232323';

    org_a UUID := 'f1250001-aaaa-aaaa-aaaa-000000000001';
    org_b UUID := 'f1250001-bbbb-bbbb-bbbb-000000000002';
BEGIN
    SET LOCAL ROLE authenticated;

    -- N14: Owner A calls set_organization_member_role targeting Tenant B
    PERFORM set_config('request.jwt.claim.sub', owner_a::text, true);
    BEGIN
        PERFORM public.set_organization_member_role(org_b, admin_b, 'cashier');
        RAISE EXCEPTION 'RPC SECURITY VIOLATION: Owner A modified member role in Tenant B' USING ERRCODE = 'TF001';
    EXCEPTION
        WHEN SQLSTATE 'TF001' THEN RAISE;
        WHEN SQLSTATE '42501' THEN
            RAISE NOTICE 'PASS N14: Cross-tenant member role modification rejected with 42501 Insufficient privilege';
    END;

    -- N15: Owner A calls set_organization_member_role targeting Org A but with non-member user (owner_b)
    BEGIN
        PERFORM public.set_organization_member_role(org_a, owner_b, 'admin');
        RAISE EXCEPTION 'RPC SECURITY VIOLATION: Owner A assigned role to non-member in Org A' USING ERRCODE = 'TF001';
    EXCEPTION
        WHEN SQLSTATE 'TF001' THEN RAISE;
        WHEN SQLSTATE 'P0002' THEN
            RAISE NOTICE 'PASS N15: Non-member role assignment rejected with P0002 Target member not found';
    END;

    -- N16: Bootstrap validation - caller with invalid/empty organization name rejected fail-closed
    BEGIN
        PERFORM public.create_organization_with_initial_setup('', 'USD', 'en', 'Main Branch', 'POS-01', 'REG-01');
        RAISE EXCEPTION 'RPC SECURITY VIOLATION: create_organization_with_initial_setup accepted empty name' USING ERRCODE = 'TF001';
    EXCEPTION
        WHEN SQLSTATE 'TF001' THEN RAISE;
        WHEN SQLSTATE '22023' THEN
            RAISE NOTICE 'PASS N16: Invalid organization bootstrap parameters rejected with 22023 Invalid parameter';
    END;

    -- N17: Manager A attempting privilege escalation / self-promotion via set_organization_member_role
    PERFORM set_config('request.jwt.claim.sub', manager_a::text, true);
    BEGIN
        PERFORM public.set_organization_member_role(org_a, manager_a, 'owner');
        RAISE EXCEPTION 'ROLE ESCALATION VIOLATION: Manager A promoted self to owner' USING ERRCODE = 'TF001';
    EXCEPTION
        WHEN SQLSTATE 'TF001' THEN RAISE;
        WHEN SQLSTATE '42501' THEN
            RAISE NOTICE 'PASS N17: Manager privilege escalation rejected with 42501 Insufficient privilege';
    END;

    -- N18: Sole Owner demotion attempt via set_organization_member_role
    PERFORM set_config('request.jwt.claim.sub', owner_a::text, true);
    BEGIN
        PERFORM public.set_organization_member_role(org_a, owner_a, 'admin');
        RAISE EXCEPTION 'SOLE OWNER VIOLATION: Sole owner was demoted to admin' USING ERRCODE = 'TF001';
    EXCEPTION
        WHEN SQLSTATE 'TF001' THEN RAISE;
        WHEN SQLSTATE '23514' THEN
            RAISE NOTICE 'PASS N18: Sole owner demotion rejected with 23514 check constraint violation';
    END;
END $$;

-- ============================================================
-- Suite 5: Cross-Tenant Membership, Permissions & Catalog Tampering (N19–N22)
-- Executed under authenticated tenant identity verifying membership and catalog protection
-- ============================================================

DO $$
DECLARE
    admin_a UUID := '12121212-1212-1212-1212-121212121212';
    owner_a UUID := '11111111-1111-1111-1111-111111111111';
    user_b_id UUID := 'f1250004-bbbb-bbbb-bbbb-000000000002';
    org_b UUID := 'f1250001-bbbb-bbbb-bbbb-000000000002';
    perm_id UUID := 'f1250005-aaaa-aaaa-aaaa-000000000001';

    mutated_cnt INTEGER := 0;
    perm_count INTEGER := 0;
BEGIN
    -- Elevated pre-check: verify fixtures exist
    SELECT COUNT(*) INTO perm_count FROM public.user_permissions WHERE user_id = user_b_id AND permission_id = perm_id;
    IF perm_count < 1 THEN
        RAISE EXCEPTION 'FIXTURE ERROR: user_permissions row missing for user_b_id';
    END IF;

    SELECT COUNT(*) INTO perm_count FROM public.role_permissions WHERE role = 'cashier' AND permission_id = perm_id;
    IF perm_count < 1 THEN
        RAISE EXCEPTION 'FIXTURE ERROR: role_permissions row missing for cashier role';
    END IF;

    SET LOCAL ROLE authenticated;

    -- N19: Admin A attempts direct SQL INSERT into organization_members for Org B
    PERFORM set_config('request.jwt.claim.sub', admin_a::text, true);
    BEGIN
        INSERT INTO public.organization_members (organization_id, user_id, role)
        VALUES (org_b, admin_a, 'admin');
        RAISE EXCEPTION 'MEMBERSHIP VIOLATION: Admin A inserted membership into Org B' USING ERRCODE = 'TF001';
    EXCEPTION
        WHEN SQLSTATE 'TF001' THEN RAISE;
        WHEN insufficient_privilege THEN
            RAISE NOTICE 'PASS N19: Cross-tenant membership insertion rejected by RLS policy';
    END;

    -- N20: Admin A attempts direct SQL INSERT of user_permission override for User B (in Org B)
    BEGIN
        INSERT INTO public.user_permissions (user_id, permission_id, effect)
        VALUES (user_b_id, perm_id, 'deny');
        RAISE EXCEPTION 'PERMISSION VIOLATION: Admin A inserted user_permission override for Tenant B user' USING ERRCODE = 'TF001';
    EXCEPTION
        WHEN SQLSTATE 'TF001' THEN RAISE;
        WHEN insufficient_privilege THEN
            RAISE NOTICE 'PASS N20: Cross-tenant user permission override injection rejected by RLS policy';
    END;

    -- N21: Admin A attempts direct SQL DELETE of user_permissions for User B (in Org B)
    BEGIN
        DELETE FROM public.user_permissions WHERE user_id = user_b_id;
        GET DIAGNOSTICS mutated_cnt = ROW_COUNT;
        IF mutated_cnt > 0 THEN
            RAISE EXCEPTION 'RLS SECURITY VIOLATION: Admin A deleted % user_permission rows in Org B', mutated_cnt USING ERRCODE = 'TF001';
        END IF;
        RAISE NOTICE 'PASS N21: Cross-tenant user permission override deletion blocked by RLS (0 rows deleted)';
    EXCEPTION
        WHEN SQLSTATE 'TF001' THEN RAISE;
        WHEN insufficient_privilege THEN
            RAISE NOTICE 'PASS N21: Cross-tenant user permission override deletion rejected by RLS policy';
    END;

    -- Elevated verification: confirm User B user_permissions row was NOT deleted
    RESET ROLE;
    SELECT COUNT(*) INTO perm_count FROM public.user_permissions WHERE user_id = user_b_id AND permission_id = perm_id;
    IF perm_count < 1 THEN
        RAISE EXCEPTION 'INVARIANT BROKEN: User B permission override was deleted by unauthorized caller';
    END IF;

    -- N22: Tenant Owner attempts mutation of global catalog (permissions & role_permissions)
    SET LOCAL ROLE authenticated;
    PERFORM set_config('request.jwt.claim.sub', owner_a::text, true);
    BEGIN
        INSERT INTO public.permissions (id, code, description)
        VALUES ('f1250099-cccc-cccc-cccc-000000000001', 'rogue.permission', 'Rogue Permission');
        RAISE EXCEPTION 'CATALOG VIOLATION: Owner A inserted into global permissions catalog' USING ERRCODE = 'TF001';
    EXCEPTION
        WHEN SQLSTATE 'TF001' THEN RAISE;
        WHEN insufficient_privilege THEN
            RAISE NOTICE 'PASS N22a: Global permissions catalog insertion rejected by RLS policy';
    END;

    BEGIN
        DELETE FROM public.role_permissions WHERE role = 'cashier' AND permission_id = perm_id;
        GET DIAGNOSTICS mutated_cnt = ROW_COUNT;
        IF mutated_cnt > 0 THEN
            RAISE EXCEPTION 'CATALOG VIOLATION: Owner A deleted % role_permission catalog rows', mutated_cnt USING ERRCODE = 'TF001';
        END IF;
        RAISE NOTICE 'PASS N22b: Global role_permissions catalog deletion blocked (0 rows deleted)';
    EXCEPTION
        WHEN SQLSTATE 'TF001' THEN RAISE;
        WHEN insufficient_privilege THEN
            RAISE NOTICE 'PASS N22b: Global role_permissions catalog deletion rejected by RLS policy';
    END;

    -- Elevated verification: confirm role_permissions row for cashier was NOT deleted
    RESET ROLE;
    SELECT COUNT(*) INTO perm_count FROM public.role_permissions WHERE role = 'cashier' AND permission_id = perm_id;
    IF perm_count < 1 THEN
        RAISE EXCEPTION 'INVARIANT BROKEN: Global role_permissions catalog row was deleted by tenant owner';
    END IF;
END $$;

-- ============================================================
-- Suite 6: Anonymous & Unaffiliated Adversarial Penetration (N23–N25)
-- Executed under anon and unaffiliated authenticated roles
-- ============================================================

DO $$
DECLARE
    unaffiliated UUID := '99999999-9999-9999-9999-999999999999';
    org_a UUID := 'f1250001-aaaa-aaaa-aaaa-000000000001';
    reg_a_paired UUID := 'f1250003-aaaa-aaaa-aaaa-000000000001';
    reg_a_unpaired UUID := 'f1250003-aaaa-aaaa-aaaa-000000000002';
    cashier_a UUID := '14141414-1414-1414-1414-141414141414';

    mutated_cnt INTEGER := 0;
BEGIN
    -- N23: Unaffiliated authenticated user calling privileged RPCs
    SET LOCAL ROLE authenticated;
    PERFORM set_config('request.jwt.claim.sub', unaffiliated::text, true);

    BEGIN
        PERFORM public.pair_device_to_register(reg_a_unpaired, 'dev-unaffil-01');
        RAISE EXCEPTION 'RPC VIOLATION: Unaffiliated user paired device' USING ERRCODE = 'TF001';
    EXCEPTION
        WHEN SQLSTATE 'TF001' THEN RAISE;
        WHEN SQLSTATE '42501' THEN
            RAISE NOTICE 'PASS N23a: Unaffiliated pair_device_to_register rejected with 42501 Insufficient privilege';
    END;

    BEGIN
        PERFORM public.revoke_device_pairing(reg_a_paired);
        RAISE EXCEPTION 'RPC VIOLATION: Unaffiliated user revoked device' USING ERRCODE = 'TF001';
    EXCEPTION
        WHEN SQLSTATE 'TF001' THEN RAISE;
        WHEN SQLSTATE '42501' THEN
            RAISE NOTICE 'PASS N23b: Unaffiliated revoke_device_pairing rejected with 42501 Insufficient privilege';
    END;

    BEGIN
        PERFORM public.set_organization_member_role(org_a, cashier_a, 'admin');
        RAISE EXCEPTION 'RPC VIOLATION: Unaffiliated user modified member role' USING ERRCODE = 'TF001';
    EXCEPTION
        WHEN SQLSTATE 'TF001' THEN RAISE;
        WHEN SQLSTATE '42501' THEN
            RAISE NOTICE 'PASS N23c: Unaffiliated set_organization_member_role rejected with 42501 Insufficient privilege';
    END;

    -- N24: Anonymous direct mutation penetration across all cloud tables
    SET LOCAL ROLE anon;
    PERFORM set_config('request.jwt.claim.sub', '', true);

    BEGIN
        INSERT INTO public.organizations (id, name) VALUES ('f1250099-0000-0000-0000-000000000001', 'Anon Org');
        RAISE EXCEPTION 'ANON PENETRATION: Anon inserted organization' USING ERRCODE = 'TF001';
    EXCEPTION
        WHEN SQLSTATE 'TF001' THEN RAISE;
        WHEN insufficient_privilege THEN
            RAISE NOTICE 'PASS N24a: Anonymous organization insert rejected by PostgreSQL privilege check';
    END;

    BEGIN
        INSERT INTO public.branches (organization_id, name) VALUES (org_a, 'Anon Branch');
        RAISE EXCEPTION 'ANON PENETRATION: Anon inserted branch' USING ERRCODE = 'TF001';
    EXCEPTION
        WHEN SQLSTATE 'TF001' THEN RAISE;
        WHEN insufficient_privilege THEN
            RAISE NOTICE 'PASS N24b: Anonymous branch insert rejected by PostgreSQL privilege check';
    END;

    BEGIN
        INSERT INTO public.registers (organization_id, branch_id, name, code) VALUES (org_a, 'f1250002-aaaa-aaaa-aaaa-000000000001', 'Anon Reg', 'ANON-1');
        RAISE EXCEPTION 'ANON PENETRATION: Anon inserted register' USING ERRCODE = 'TF001';
    EXCEPTION
        WHEN SQLSTATE 'TF001' THEN RAISE;
        WHEN insufficient_privilege THEN
            RAISE NOTICE 'PASS N24c: Anonymous register insert rejected by PostgreSQL privilege check';
    END;

    BEGIN
        UPDATE public.branches SET name = 'Anon Hacked Branch' WHERE id = 'f1250002-aaaa-aaaa-aaaa-000000000001';
        GET DIAGNOSTICS mutated_cnt = ROW_COUNT;
        IF mutated_cnt > 0 THEN
            RAISE EXCEPTION 'ANON PENETRATION: Anon updated % branch rows', mutated_cnt USING ERRCODE = 'TF001';
        END IF;
        RAISE NOTICE 'PASS N24d: Anonymous branch update blocked (0 rows mutated)';
    EXCEPTION
        WHEN SQLSTATE 'TF001' THEN RAISE;
        WHEN insufficient_privilege THEN
            RAISE NOTICE 'PASS N24d: Anonymous branch update rejected by PostgreSQL privilege check';
    END;

    BEGIN
        DELETE FROM public.users WHERE id = 'f1250004-aaaa-aaaa-aaaa-000000000001';
        GET DIAGNOSTICS mutated_cnt = ROW_COUNT;
        IF mutated_cnt > 0 THEN
            RAISE EXCEPTION 'ANON PENETRATION: Anon deleted % user rows', mutated_cnt USING ERRCODE = 'TF001';
        END IF;
        RAISE NOTICE 'PASS N24e: Anonymous user delete blocked (0 rows mutated)';
    EXCEPTION
        WHEN SQLSTATE 'TF001' THEN RAISE;
        WHEN insufficient_privilege THEN
            RAISE NOTICE 'PASS N24e: Anonymous user delete rejected by PostgreSQL privilege check';
    END;

    -- N25: Anonymous privileged RPC execution denial
    BEGIN
        PERFORM public.create_organization_with_initial_setup('Anon Org Bootstrap');
        RAISE EXCEPTION 'ANON RPC PENETRATION: Anon called create_organization_with_initial_setup' USING ERRCODE = 'TF001';
    EXCEPTION
        WHEN SQLSTATE 'TF001' THEN RAISE;
        WHEN SQLSTATE '42501' THEN
            RAISE NOTICE 'PASS N25a: Anonymous create_organization_with_initial_setup rejected with 42501';
        WHEN insufficient_privilege THEN
            RAISE NOTICE 'PASS N25a: Anonymous create_organization_with_initial_setup rejected by privilege check';
    END;

    BEGIN
        PERFORM public.pair_device_to_register(reg_a_unpaired, 'dev-anon-01');
        RAISE EXCEPTION 'ANON RPC PENETRATION: Anon called pair_device_to_register' USING ERRCODE = 'TF001';
    EXCEPTION
        WHEN SQLSTATE 'TF001' THEN RAISE;
        WHEN SQLSTATE '42501' THEN
            RAISE NOTICE 'PASS N25b: Anonymous pair_device_to_register rejected with 42501';
        WHEN insufficient_privilege THEN
            RAISE NOTICE 'PASS N25b: Anonymous pair_device_to_register rejected by privilege check';
    END;

    BEGIN
        PERFORM public.revoke_device_pairing(reg_a_paired);
        RAISE EXCEPTION 'ANON RPC PENETRATION: Anon called revoke_device_pairing' USING ERRCODE = 'TF001';
    EXCEPTION
        WHEN SQLSTATE 'TF001' THEN RAISE;
        WHEN SQLSTATE '42501' THEN
            RAISE NOTICE 'PASS N25c: Anonymous revoke_device_pairing rejected with 42501';
        WHEN insufficient_privilege THEN
            RAISE NOTICE 'PASS N25c: Anonymous revoke_device_pairing rejected by privilege check';
    END;

    BEGIN
        PERFORM public.record_device_heartbeat(reg_a_paired, 'dev-alpha-active-01');
        RAISE EXCEPTION 'ANON RPC PENETRATION: Anon called record_device_heartbeat' USING ERRCODE = 'TF001';
    EXCEPTION
        WHEN SQLSTATE 'TF001' THEN RAISE;
        WHEN SQLSTATE '42501' THEN
            RAISE NOTICE 'PASS N25d: Anonymous record_device_heartbeat rejected with 42501';
        WHEN insufficient_privilege THEN
            RAISE NOTICE 'PASS N25d: Anonymous record_device_heartbeat rejected by privilege check';
    END;

    BEGIN
        PERFORM public.set_organization_member_role(org_a, cashier_a, 'admin');
        RAISE EXCEPTION 'ANON RPC PENETRATION: Anon called set_organization_member_role' USING ERRCODE = 'TF001';
    EXCEPTION
        WHEN SQLSTATE 'TF001' THEN RAISE;
        WHEN SQLSTATE '42501' THEN
            RAISE NOTICE 'PASS N25e: Anonymous set_organization_member_role rejected with 42501';
        WHEN insufficient_privilege THEN
            RAISE NOTICE 'PASS N25e: Anonymous set_organization_member_role rejected by privilege check';
    END;
END $$;

ROLLBACK;
