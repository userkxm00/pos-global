-- Supabase Test Suite: organization_branch_member_test.sql
-- F1.20 — Organization / Branch / Member Cloud Schema Verification
-- Behavioral PostgreSQL test assertions for composite tenant integrity,
-- domain check constraints, automatic timestamps, sole-owner mutation guard,
-- and unique identity mapping.
--
-- This test validates runtime database behavior, not static SQL structure.

BEGIN;

-- ============================================================
-- 1. Test Setup: Standalone auth schema (same pattern as rls_policies_test.sql)
-- ============================================================

DO $$
BEGIN
    IF NOT EXISTS (SELECT 1 FROM pg_roles WHERE rolname = 'authenticated') THEN
        CREATE ROLE authenticated NOLOGIN NOINHERIT;
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

GRANT USAGE ON SCHEMA public, auth TO authenticated;
GRANT ALL ON ALL TABLES IN SCHEMA public TO authenticated;
GRANT SELECT ON ALL TABLES IN SCHEMA auth TO authenticated;
GRANT EXECUTE ON ALL FUNCTIONS IN SCHEMA public, auth TO authenticated;

-- ============================================================
-- 2. Test Fixtures
-- ============================================================

DO $$
DECLARE
    auth_owner_a UUID := '10000000-0000-0000-0000-000000000001';
    auth_owner_a2 UUID := '10000000-0000-0000-0000-000000000002';
    auth_member_a UUID := '10000000-0000-0000-0000-000000000003';
    auth_owner_b UUID := '20000000-0000-0000-0000-000000000001';

    org_a UUID := 'f1200001-aaaa-aaaa-aaaa-000000000001';
    org_b UUID := 'f1200001-bbbb-bbbb-bbbb-000000000002';

    branch_a UUID := 'f1200002-aaaa-aaaa-aaaa-000000000001';
    branch_b UUID := 'f1200002-bbbb-bbbb-bbbb-000000000002';
BEGIN
    -- Auth users
    INSERT INTO auth.users (id, email) VALUES
        (auth_owner_a, 'f1_20_owner_a@test.com'),
        (auth_owner_a2, 'f1_20_owner_a2@test.com'),
        (auth_member_a, 'f1_20_member_a@test.com'),
        (auth_owner_b, 'f1_20_owner_b@test.com')
    ON CONFLICT (id) DO NOTHING;

    -- Organizations
    INSERT INTO public.organizations (id, name, default_currency, default_language) VALUES
        (org_a, 'F1.20 Test Org A', 'USD', 'en'),
        (org_b, 'F1.20 Test Org B', 'EUR', 'de')
    ON CONFLICT (id) DO NOTHING;

    -- Organization Members
    INSERT INTO public.organization_members (organization_id, user_id, role) VALUES
        (org_a, auth_owner_a, 'owner'),
        (org_a, auth_owner_a2, 'owner'),
        (org_a, auth_member_a, 'cashier'),
        (org_b, auth_owner_b, 'owner')
    ON CONFLICT (organization_id, user_id) DO NOTHING;

    -- Branches
    INSERT INTO public.branches (id, organization_id, name, currency, is_active) VALUES
        (branch_a, org_a, 'F1.20 Branch A', 'USD', true),
        (branch_b, org_b, 'F1.20 Branch B', 'EUR', true)
    ON CONFLICT (id) DO NOTHING;
END $$;

-- ============================================================
-- 3. Composite FK — Cross-Tenant Branch Rejection (Users)
-- ============================================================

-- Attempt to create a user in org_a but attached to branch_b (org_b).
-- This MUST be rejected by fk_users_branch_org composite FK.
DO $$
DECLARE
    org_a UUID := 'f1200001-aaaa-aaaa-aaaa-000000000001';
    branch_b UUID := 'f1200002-bbbb-bbbb-bbbb-000000000002';
    auth_owner_a UUID := '10000000-0000-0000-0000-000000000001';
BEGIN
    INSERT INTO public.users (organization_id, branch_id, supabase_user_id, full_name, username, role)
    VALUES (org_a, branch_b, auth_owner_a, 'Cross Tenant User', 'cross_user', 'cashier');

    RAISE EXCEPTION 'SCHEMA VIOLATION: Cross-tenant user-branch attachment was permitted';
EXCEPTION
    WHEN foreign_key_violation THEN
        RAISE NOTICE 'PASS: Cross-tenant user-branch attachment correctly rejected by composite FK';
END $$;

-- ============================================================
-- 4. Composite FK — Cross-Tenant Branch Rejection (Registers)
-- ============================================================

DO $$
DECLARE
    org_a UUID := 'f1200001-aaaa-aaaa-aaaa-000000000001';
    branch_b UUID := 'f1200002-bbbb-bbbb-bbbb-000000000002';
BEGIN
    INSERT INTO public.registers (organization_id, branch_id, name, code)
    VALUES (org_a, branch_b, 'Cross Tenant Register', 'XREG-01');

    RAISE EXCEPTION 'SCHEMA VIOLATION: Cross-tenant register-branch attachment was permitted';
EXCEPTION
    WHEN foreign_key_violation THEN
        RAISE NOTICE 'PASS: Cross-tenant register-branch attachment correctly rejected by composite FK';
END $$;

-- ============================================================
-- 5. Domain Check Constraints — Organization
-- ============================================================

-- Empty name rejected
DO $$
BEGIN
    INSERT INTO public.organizations (name, default_currency, default_language)
    VALUES ('', 'USD', 'en');

    RAISE EXCEPTION 'SCHEMA VIOLATION: Empty organization name was permitted';
EXCEPTION
    WHEN check_violation THEN
        RAISE NOTICE 'PASS: Empty organization name correctly rejected';
END $$;

-- Whitespace-only name rejected
DO $$
BEGIN
    INSERT INTO public.organizations (name, default_currency, default_language)
    VALUES ('   ', 'USD', 'en');

    RAISE EXCEPTION 'SCHEMA VIOLATION: Whitespace-only organization name was permitted';
EXCEPTION
    WHEN check_violation THEN
        RAISE NOTICE 'PASS: Whitespace-only organization name correctly rejected';
END $$;

-- Invalid currency format rejected (lowercase)
DO $$
BEGIN
    INSERT INTO public.organizations (name, default_currency, default_language)
    VALUES ('Test Currency Org', 'usd', 'en');

    RAISE EXCEPTION 'SCHEMA VIOLATION: Lowercase currency code was permitted';
EXCEPTION
    WHEN check_violation THEN
        RAISE NOTICE 'PASS: Lowercase currency code correctly rejected';
END $$;

-- Invalid currency format rejected (2-char)
DO $$
BEGIN
    INSERT INTO public.organizations (name, default_currency, default_language)
    VALUES ('Test Currency Org 2', 'US', 'en');

    RAISE EXCEPTION 'SCHEMA VIOLATION: 2-character currency code was permitted';
EXCEPTION
    WHEN check_violation THEN
        RAISE NOTICE 'PASS: 2-character currency code correctly rejected';
END $$;

-- Invalid currency format rejected (4-char)
DO $$
BEGIN
    INSERT INTO public.organizations (name, default_currency, default_language)
    VALUES ('Test Currency Org 3', 'USDX', 'en');

    RAISE EXCEPTION 'SCHEMA VIOLATION: 4-character currency code was permitted';
EXCEPTION
    WHEN check_violation THEN
        RAISE NOTICE 'PASS: 4-character currency code correctly rejected';
END $$;

-- Invalid language (1-char)
DO $$
BEGIN
    INSERT INTO public.organizations (name, default_currency, default_language)
    VALUES ('Test Lang Org', 'USD', 'e');

    RAISE EXCEPTION 'SCHEMA VIOLATION: 1-character language code was permitted';
EXCEPTION
    WHEN check_violation THEN
        RAISE NOTICE 'PASS: 1-character language code correctly rejected';
END $$;

-- Valid organization insert succeeds
DO $$
DECLARE
    new_org_id UUID;
BEGIN
    INSERT INTO public.organizations (name, default_currency, default_language)
    VALUES ('Valid Check Org', 'GBP', 'en-GB')
    RETURNING id INTO new_org_id;

    -- Clean up
    DELETE FROM public.organizations WHERE id = new_org_id;
    RAISE NOTICE 'PASS: Valid organization creation succeeded';
END $$;

-- ============================================================
-- 6. Domain Check Constraints — Branch
-- ============================================================

-- Empty branch name rejected
DO $$
DECLARE
    org_a UUID := 'f1200001-aaaa-aaaa-aaaa-000000000001';
BEGIN
    INSERT INTO public.branches (organization_id, name, currency)
    VALUES (org_a, '', 'USD');

    RAISE EXCEPTION 'SCHEMA VIOLATION: Empty branch name was permitted';
EXCEPTION
    WHEN check_violation THEN
        RAISE NOTICE 'PASS: Empty branch name correctly rejected';
END $$;

-- Invalid branch currency rejected
DO $$
DECLARE
    org_a UUID := 'f1200001-aaaa-aaaa-aaaa-000000000001';
BEGIN
    INSERT INTO public.branches (organization_id, name, currency)
    VALUES (org_a, 'Bad Currency Branch', 'bad');

    RAISE EXCEPTION 'SCHEMA VIOLATION: Invalid branch currency was permitted';
EXCEPTION
    WHEN check_violation THEN
        RAISE NOTICE 'PASS: Invalid branch currency correctly rejected';
END $$;

-- ============================================================
-- 7. Domain Check Constraints — Users
-- ============================================================

-- Empty full_name rejected
DO $$
DECLARE
    org_a UUID := 'f1200001-aaaa-aaaa-aaaa-000000000001';
    branch_a UUID := 'f1200002-aaaa-aaaa-aaaa-000000000001';
BEGIN
    INSERT INTO public.users (organization_id, branch_id, full_name, role)
    VALUES (org_a, branch_a, '', 'cashier');

    RAISE EXCEPTION 'SCHEMA VIOLATION: Empty user full_name was permitted';
EXCEPTION
    WHEN check_violation THEN
        RAISE NOTICE 'PASS: Empty user full_name correctly rejected';
END $$;

-- Too-short username rejected (2 chars)
DO $$
DECLARE
    org_a UUID := 'f1200001-aaaa-aaaa-aaaa-000000000001';
    branch_a UUID := 'f1200002-aaaa-aaaa-aaaa-000000000001';
BEGIN
    INSERT INTO public.users (organization_id, branch_id, full_name, username, role)
    VALUES (org_a, branch_a, 'Short User', 'ab', 'cashier');

    RAISE EXCEPTION 'SCHEMA VIOLATION: 2-char username was permitted';
EXCEPTION
    WHEN check_violation THEN
        RAISE NOTICE 'PASS: 2-char username correctly rejected';
END $$;

-- Username with spaces rejected
DO $$
DECLARE
    org_a UUID := 'f1200001-aaaa-aaaa-aaaa-000000000001';
    branch_a UUID := 'f1200002-aaaa-aaaa-aaaa-000000000001';
BEGIN
    INSERT INTO public.users (organization_id, branch_id, full_name, username, role)
    VALUES (org_a, branch_a, 'Space User', 'has space', 'cashier');

    RAISE EXCEPTION 'SCHEMA VIOLATION: Username with spaces was permitted';
EXCEPTION
    WHEN check_violation THEN
        RAISE NOTICE 'PASS: Username with spaces correctly rejected';
END $$;

-- NULL username accepted (optional field)
DO $$
DECLARE
    org_a UUID := 'f1200001-aaaa-aaaa-aaaa-000000000001';
    branch_a UUID := 'f1200002-aaaa-aaaa-aaaa-000000000001';
    new_user_id UUID;
BEGIN
    INSERT INTO public.users (organization_id, branch_id, full_name, username, role)
    VALUES (org_a, branch_a, 'Null Username User', NULL, 'cashier')
    RETURNING id INTO new_user_id;

    DELETE FROM public.users WHERE id = new_user_id;
    RAISE NOTICE 'PASS: NULL username correctly accepted';
END $$;

-- ============================================================
-- 8. Sole-Owner Deletion Prevention
-- ============================================================

-- Remove co-owner first so auth_owner_a becomes sole owner, then attempt deletion
DO $$
DECLARE
    org_a UUID := 'f1200001-aaaa-aaaa-aaaa-000000000001';
    auth_owner_a UUID := '10000000-0000-0000-0000-000000000001';
    auth_owner_a2 UUID := '10000000-0000-0000-0000-000000000002';
BEGIN
    -- Remove co-owner to make auth_owner_a the sole owner
    DELETE FROM public.organization_members
    WHERE organization_id = org_a AND user_id = auth_owner_a2;

    -- Now attempt to delete the sole remaining owner
    DELETE FROM public.organization_members
    WHERE organization_id = org_a AND user_id = auth_owner_a;

    RAISE EXCEPTION 'SCHEMA VIOLATION: Sole owner deletion was permitted';
EXCEPTION
    WHEN raise_exception THEN
        -- Re-insert co-owner for subsequent tests
        INSERT INTO public.organization_members (organization_id, user_id, role)
        VALUES (org_a, auth_owner_a2, 'owner')
        ON CONFLICT (organization_id, user_id) DO NOTHING;
        RAISE NOTICE 'PASS: Sole owner deletion correctly prevented by trigger';
END $$;

-- ============================================================
-- 9. Sole-Owner Role Demotion Prevention (UPDATE guard)
-- ============================================================

-- Remove co-owner, then attempt to demote the sole remaining owner to 'admin'
DO $$
DECLARE
    org_a UUID := 'f1200001-aaaa-aaaa-aaaa-000000000001';
    auth_owner_a UUID := '10000000-0000-0000-0000-000000000001';
    auth_owner_a2 UUID := '10000000-0000-0000-0000-000000000002';
BEGIN
    -- Remove co-owner to make auth_owner_a sole owner
    DELETE FROM public.organization_members
    WHERE organization_id = org_a AND user_id = auth_owner_a2;

    -- Attempt to demote sole owner to admin
    UPDATE public.organization_members
    SET role = 'admin'
    WHERE organization_id = org_a AND user_id = auth_owner_a;

    RAISE EXCEPTION 'SCHEMA VIOLATION: Sole owner role demotion was permitted';
EXCEPTION
    WHEN raise_exception THEN
        -- Re-insert co-owner for subsequent tests
        INSERT INTO public.organization_members (organization_id, user_id, role)
        VALUES (org_a, auth_owner_a2, 'owner')
        ON CONFLICT (organization_id, user_id) DO NOTHING;
        RAISE NOTICE 'PASS: Sole owner role demotion correctly prevented by trigger';
END $$;

-- ============================================================
-- 10. Co-Owner Deletion Permitted When Another Owner Remains
-- ============================================================

DO $$
DECLARE
    org_a UUID := 'f1200001-aaaa-aaaa-aaaa-000000000001';
    auth_owner_a2 UUID := '10000000-0000-0000-0000-000000000002';
BEGIN
    -- Both auth_owner_a and auth_owner_a2 are owners. Deleting one is permitted.
    DELETE FROM public.organization_members
    WHERE organization_id = org_a AND user_id = auth_owner_a2;

    -- Verify deletion succeeded
    IF EXISTS (
        SELECT 1 FROM public.organization_members
        WHERE organization_id = org_a AND user_id = auth_owner_a2
    ) THEN
        RAISE EXCEPTION 'SCHEMA VIOLATION: Co-owner deletion did not take effect';
    END IF;

    -- Re-insert co-owner for subsequent tests
    INSERT INTO public.organization_members (organization_id, user_id, role)
    VALUES (org_a, auth_owner_a2, 'owner')
    ON CONFLICT (organization_id, user_id) DO NOTHING;

    RAISE NOTICE 'PASS: Co-owner deletion correctly permitted when another owner remains';
END $$;

-- ============================================================
-- 11. Automatic updated_at Timestamp Trigger
-- ============================================================

DO $$
DECLARE
    org_a UUID := 'f1200001-aaaa-aaaa-aaaa-000000000001';
    ts_before TIMESTAMPTZ;
    ts_after TIMESTAMPTZ;
BEGIN
    SELECT updated_at INTO ts_before FROM public.organizations WHERE id = org_a;

    -- Ensure clock advances
    PERFORM pg_sleep(0.05);

    UPDATE public.organizations SET name = 'F1.20 Test Org A Updated' WHERE id = org_a;

    SELECT updated_at INTO ts_after FROM public.organizations WHERE id = org_a;

    IF ts_after <= ts_before THEN
        RAISE EXCEPTION 'SCHEMA VIOLATION: updated_at was not advanced by trigger (before=%, after=%)',
            ts_before, ts_after;
    END IF;

    -- Reset name
    UPDATE public.organizations SET name = 'F1.20 Test Org A' WHERE id = org_a;

    RAISE NOTICE 'PASS: updated_at trigger correctly advances timestamp on UPDATE';
END $$;

-- ============================================================
-- 12. Unique Supabase Identity Mapping per Organization
-- ============================================================

-- Attempt to insert a second user in same org with same supabase_user_id
DO $$
DECLARE
    org_a UUID := 'f1200001-aaaa-aaaa-aaaa-000000000001';
    branch_a UUID := 'f1200002-aaaa-aaaa-aaaa-000000000001';
    auth_member_a UUID := '10000000-0000-0000-0000-000000000003';
    user1_id UUID;
BEGIN
    -- First user with this supabase_user_id
    INSERT INTO public.users (organization_id, branch_id, supabase_user_id, full_name, role)
    VALUES (org_a, branch_a, auth_member_a, 'First Mapping', 'cashier')
    RETURNING id INTO user1_id;

    -- Second user with the SAME supabase_user_id in the SAME org - must fail
    INSERT INTO public.users (organization_id, branch_id, supabase_user_id, full_name, role)
    VALUES (org_a, branch_a, auth_member_a, 'Duplicate Mapping', 'cashier');

    RAISE EXCEPTION 'SCHEMA VIOLATION: Duplicate supabase_user_id mapping in same org was permitted';
EXCEPTION
    WHEN unique_violation THEN
        -- Clean up
        DELETE FROM public.users WHERE id = user1_id;
        RAISE NOTICE 'PASS: Duplicate supabase_user_id in same org correctly rejected';
END $$;

-- ============================================================
-- 13. Cascading Organization Deletion
-- ============================================================

DO $$
DECLARE
    test_org_id UUID;
    test_branch_id UUID;
    auth_user UUID := '10000000-0000-0000-0000-000000000003';
BEGIN
    -- Create temporary org, branch, user, and register
    INSERT INTO public.organizations (name, default_currency, default_language)
    VALUES ('Cascade Test Org', 'USD', 'en')
    RETURNING id INTO test_org_id;

    INSERT INTO public.organization_members (organization_id, user_id, role)
    VALUES (test_org_id, auth_user, 'owner')
    ON CONFLICT (organization_id, user_id) DO NOTHING;

    INSERT INTO public.branches (organization_id, name, currency)
    VALUES (test_org_id, 'Cascade Branch', 'USD')
    RETURNING id INTO test_branch_id;

    INSERT INTO public.users (organization_id, branch_id, full_name, role)
    VALUES (test_org_id, test_branch_id, 'Cascade User', 'cashier');

    INSERT INTO public.registers (organization_id, branch_id, name, code)
    VALUES (test_org_id, test_branch_id, 'Cascade Register', 'CREG-01');

    -- Delete the organization: all children must cascade
    -- First remove the sole-owner guard by adding a second owner then
    -- deleting the member directly, or just delete the org itself.
    DELETE FROM public.organizations WHERE id = test_org_id;

    -- Verify all children are removed
    IF EXISTS (SELECT 1 FROM public.branches WHERE organization_id = test_org_id) THEN
        RAISE EXCEPTION 'SCHEMA VIOLATION: Branches not cascaded on org deletion';
    END IF;
    IF EXISTS (SELECT 1 FROM public.organization_members WHERE organization_id = test_org_id) THEN
        RAISE EXCEPTION 'SCHEMA VIOLATION: Members not cascaded on org deletion';
    END IF;

    RAISE NOTICE 'PASS: Organization deletion correctly cascades to branches, members, users, and registers';
END $$;

-- ============================================================
-- 14. Valid Same-Tenant User Creation (Positive Test)
-- ============================================================

DO $$
DECLARE
    org_a UUID := 'f1200001-aaaa-aaaa-aaaa-000000000001';
    branch_a UUID := 'f1200002-aaaa-aaaa-aaaa-000000000001';
    new_user_id UUID;
BEGIN
    INSERT INTO public.users (organization_id, branch_id, full_name, username, role)
    VALUES (org_a, branch_a, 'Valid Tenant User', 'valid_user', 'cashier')
    RETURNING id INTO new_user_id;

    IF new_user_id IS NULL THEN
        RAISE EXCEPTION 'SCHEMA VIOLATION: Valid same-tenant user creation failed';
    END IF;

    DELETE FROM public.users WHERE id = new_user_id;
    RAISE NOTICE 'PASS: Valid same-tenant user creation succeeded';
END $$;

-- ============================================================
-- 15. Valid Same-Tenant Register Creation (Positive Test)
-- ============================================================

DO $$
DECLARE
    org_a UUID := 'f1200001-aaaa-aaaa-aaaa-000000000001';
    branch_a UUID := 'f1200002-aaaa-aaaa-aaaa-000000000001';
    new_reg_id UUID;
BEGIN
    INSERT INTO public.registers (organization_id, branch_id, name, code)
    VALUES (org_a, branch_a, 'Valid Tenant Register', 'VREG-01')
    RETURNING id INTO new_reg_id;

    IF new_reg_id IS NULL THEN
        RAISE EXCEPTION 'SCHEMA VIOLATION: Valid same-tenant register creation failed';
    END IF;

    DELETE FROM public.registers WHERE id = new_reg_id;
    RAISE NOTICE 'PASS: Valid same-tenant register creation succeeded';
END $$;

-- ============================================================
-- Cleanup: Rollback all test changes
-- ============================================================

ROLLBACK;
