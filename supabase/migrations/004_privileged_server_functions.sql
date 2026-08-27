-- Supabase Migration: 004_privileged_server_functions.sql
-- F1.23 — Privileged Server Functions / Secure Server-Side Operations
-- Append-only migration: implements security-hardened PostgreSQL RPC functions
-- for device pairing, device revocation, device heartbeat, atomic organization onboarding,
-- and member role management.
--
-- INVARIANTS:
-- - Migrations 001, 002, and 003 are immutable and untouched.
-- - All functions use SECURITY DEFINER with fixed search_path = public.
-- - All functions perform explicit fail-closed authorization checks prior to mutation.
-- - Execution is revoked from PUBLIC and granted only to authenticated role.

-- ============================================================
-- 1. Secure Device Pairing RPC
-- ============================================================

CREATE OR REPLACE FUNCTION public.pair_device_to_register(
    p_register_id UUID,
    p_device_identifier TEXT
)
RETURNS public.registers
LANGUAGE plpgsql
SECURITY DEFINER
SET search_path = public
AS $$
DECLARE
    v_uid UUID;
    v_reg public.registers;
    v_trimmed_device TEXT;
BEGIN
    v_uid := auth.uid();
    IF v_uid IS NULL THEN
        RAISE EXCEPTION 'Authentication required to pair device' USING ERRCODE = '42501';
    END IF;

    -- Fetch target register with row lock to prevent TOCTOU race conditions
    SELECT * INTO v_reg
    FROM public.registers
    WHERE id = p_register_id
    FOR UPDATE;

    IF NOT FOUND THEN
        RAISE EXCEPTION 'Register not found' USING ERRCODE = 'P0002';
    END IF;

    -- Verify caller is manager, admin, or owner of the register's organization
    IF NOT public.is_org_manager_or_above(v_reg.organization_id) THEN
        RAISE EXCEPTION 'Insufficient privilege to pair device to register in organization %', v_reg.organization_id USING ERRCODE = '42501';
    END IF;

    -- Validate device identifier format (same domain rules as F1.21: 3-128 chars, alphanumeric with .:-_)
    v_trimmed_device := trim(p_device_identifier);
    IF v_trimmed_device IS NULL
       OR length(v_trimmed_device) < 3
       OR length(v_trimmed_device) > 128
       OR v_trimmed_device !~ '^[a-zA-Z0-9_.:-]+$' THEN
        RAISE EXCEPTION 'Invalid device identifier format' USING ERRCODE = '22023';
    END IF;

    -- Verify register is currently unpaired or revoked (cannot overwrite already active paired device without explicit revocation)
    IF v_reg.device_pairing_status = 'paired' THEN
        RAISE EXCEPTION 'Register is already paired to an active device' USING ERRCODE = '23505';
    END IF;

    -- Atomically update register
    UPDATE public.registers
    SET device_identifier = v_trimmed_device,
        device_pairing_status = 'paired',
        device_paired_at = now(),
        device_last_seen_at = now(),
        updated_at = now()
    WHERE id = p_register_id
    RETURNING * INTO v_reg;

    RETURN v_reg;
END;
$$;

-- ============================================================
-- 2. Secure Device Revocation RPC
-- ============================================================

CREATE OR REPLACE FUNCTION public.revoke_device_pairing(
    p_register_id UUID
)
RETURNS public.registers
LANGUAGE plpgsql
SECURITY DEFINER
SET search_path = public
AS $$
DECLARE
    v_uid UUID;
    v_reg public.registers;
BEGIN
    v_uid := auth.uid();
    IF v_uid IS NULL THEN
        RAISE EXCEPTION 'Authentication required to revoke device pairing' USING ERRCODE = '42501';
    END IF;

    -- Fetch target register with row lock to prevent TOCTOU race conditions
    SELECT * INTO v_reg
    FROM public.registers
    WHERE id = p_register_id
    FOR UPDATE;

    IF NOT FOUND THEN
        RAISE EXCEPTION 'Register not found' USING ERRCODE = 'P0002';
    END IF;

    -- Verify caller is manager, admin, or owner of the register's organization
    IF NOT public.is_org_manager_or_above(v_reg.organization_id) THEN
        RAISE EXCEPTION 'Insufficient privilege to revoke device pairing in organization %', v_reg.organization_id USING ERRCODE = '42501';
    END IF;

    -- Verify register is currently paired
    IF v_reg.device_pairing_status <> 'paired' THEN
        RAISE EXCEPTION 'Register is not in paired status' USING ERRCODE = '22000';
    END IF;

    -- Atomically update to revoked
    UPDATE public.registers
    SET device_pairing_status = 'revoked',
        device_last_seen_at = now(),
        updated_at = now()
    WHERE id = p_register_id
    RETURNING * INTO v_reg;

    RETURN v_reg;
END;
$$;

-- ============================================================
-- 3. Secure Device Heartbeat RPC
-- ============================================================

CREATE OR REPLACE FUNCTION public.record_device_heartbeat(
    p_register_id UUID,
    p_device_identifier TEXT
)
RETURNS VOID
LANGUAGE plpgsql
SECURITY DEFINER
SET search_path = public
AS $$
DECLARE
    v_uid UUID;
    v_reg public.registers;
    v_trimmed_device TEXT;
BEGIN
    v_uid := auth.uid();
    IF v_uid IS NULL THEN
        RAISE EXCEPTION 'Authentication required for heartbeat' USING ERRCODE = '42501';
    END IF;

    -- Reject NULL or empty device identifier immediately
    IF p_device_identifier IS NULL THEN
        RAISE EXCEPTION 'Device identifier cannot be null' USING ERRCODE = '42501';
    END IF;

    v_trimmed_device := trim(p_device_identifier);
    IF length(v_trimmed_device) = 0 THEN
        RAISE EXCEPTION 'Device identifier cannot be empty' USING ERRCODE = '42501';
    END IF;

    SELECT * INTO v_reg
    FROM public.registers
    WHERE id = p_register_id;

    IF NOT FOUND THEN
        RAISE EXCEPTION 'Register not found' USING ERRCODE = 'P0002';
    END IF;

    -- Verify caller is an active member of the register's organization
    IF NOT public.is_org_member(v_reg.organization_id) THEN
        RAISE EXCEPTION 'Insufficient privilege: caller is not a member of register organization %', v_reg.organization_id USING ERRCODE = '42501';
    END IF;

    -- Verify register is paired
    IF v_reg.device_pairing_status <> 'paired' THEN
        RAISE EXCEPTION 'Register is not actively paired' USING ERRCODE = '22000';
    END IF;

    -- Verify exact device identifier matches
    IF v_reg.device_identifier IS NULL OR v_reg.device_identifier <> v_trimmed_device THEN
        RAISE EXCEPTION 'Mismatched device identifier for register' USING ERRCODE = '42501';
    END IF;

    -- Update device_last_seen_at
    UPDATE public.registers
    SET device_last_seen_at = now()
    WHERE id = p_register_id;
END;
$$;

-- ============================================================
-- 4. Atomic Organization Bootstrap RPC
-- ============================================================

CREATE OR REPLACE FUNCTION public.create_organization_with_initial_setup(
    p_org_name TEXT,
    p_default_currency TEXT DEFAULT 'USD',
    p_default_language TEXT DEFAULT 'en',
    p_branch_name TEXT DEFAULT 'Main Branch',
    p_register_name TEXT DEFAULT 'POS-01',
    p_register_code TEXT DEFAULT 'REG-01'
)
RETURNS JSONB
LANGUAGE plpgsql
SECURITY DEFINER
SET search_path = public
AS $$
DECLARE
    v_uid UUID;
    v_org public.organizations;
    v_branch public.branches;
    v_register public.registers;
    v_user public.users;
    v_trimmed_org TEXT;
    v_trimmed_branch TEXT;
    v_trimmed_reg TEXT;
    v_trimmed_code TEXT;
    v_currency TEXT;
    v_language TEXT;
BEGIN
    v_uid := auth.uid();
    IF v_uid IS NULL THEN
        RAISE EXCEPTION 'Authentication required to create organization' USING ERRCODE = '42501';
    END IF;

    v_trimmed_org := trim(p_org_name);
    IF v_trimmed_org IS NULL OR length(v_trimmed_org) = 0 OR length(v_trimmed_org) > 255 THEN
        RAISE EXCEPTION 'Invalid organization name' USING ERRCODE = '22023';
    END IF;

    v_currency := upper(trim(coalesce(p_default_currency, 'USD')));
    IF length(v_currency) <> 3 THEN
        RAISE EXCEPTION 'Invalid currency code (must be 3 characters)' USING ERRCODE = '22023';
    END IF;

    v_language := lower(trim(coalesce(p_default_language, 'en')));
    IF length(v_language) < 2 OR length(v_language) > 10 THEN
        RAISE EXCEPTION 'Invalid language code' USING ERRCODE = '22023';
    END IF;

    v_trimmed_branch := trim(coalesce(p_branch_name, 'Main Branch'));
    IF length(v_trimmed_branch) = 0 OR length(v_trimmed_branch) > 255 THEN
        RAISE EXCEPTION 'Invalid branch name' USING ERRCODE = '22023';
    END IF;

    v_trimmed_reg := trim(coalesce(p_register_name, 'POS-01'));
    IF length(v_trimmed_reg) = 0 OR length(v_trimmed_reg) > 255 THEN
        RAISE EXCEPTION 'Invalid register name' USING ERRCODE = '22023';
    END IF;

    v_trimmed_code := trim(coalesce(p_register_code, 'REG-01'));
    IF length(v_trimmed_code) = 0 OR length(v_trimmed_code) > 50 OR v_trimmed_code !~ '^[a-zA-Z0-9_.-]+$' THEN
        RAISE EXCEPTION 'Invalid register code' USING ERRCODE = '22023';
    END IF;

    -- 1. Insert organization (the handle_new_organization_owner trigger auto-creates owner membership in organization_members)
    INSERT INTO public.organizations (name, default_currency, default_language)
    VALUES (v_trimmed_org, v_currency, v_language)
    RETURNING * INTO v_org;

    -- 2. Insert initial branch
    INSERT INTO public.branches (organization_id, name, currency, is_active)
    VALUES (v_org.id, v_trimmed_branch, v_currency, true)
    RETURNING * INTO v_branch;

    -- 3. Insert initial register
    INSERT INTO public.registers (organization_id, branch_id, name, code, is_active, device_pairing_status)
    VALUES (v_org.id, v_branch.id, v_trimmed_reg, v_trimmed_code, true, 'unpaired')
    RETURNING * INTO v_register;

    -- 4. Insert initial staff user profile for the owner
    INSERT INTO public.users (organization_id, branch_id, supabase_user_id, full_name, username, role, is_active)
    VALUES (v_org.id, v_branch.id, v_uid, 'Organization Owner', 'owner', 'admin', true)
    RETURNING * INTO v_user;

    RETURN jsonb_build_object(
        'organization', to_jsonb(v_org),
        'branch', to_jsonb(v_branch),
        'register', to_jsonb(v_register),
        'user', to_jsonb(v_user)
    );
END;
$$;

-- ============================================================
-- 5. Privileged Member Role Management RPC
-- ============================================================

CREATE OR REPLACE FUNCTION public.set_organization_member_role(
    p_organization_id UUID,
    p_target_user_id UUID,
    p_new_role TEXT
)
RETURNS public.organization_members
LANGUAGE plpgsql
SECURITY DEFINER
SET search_path = public
AS $$
DECLARE
    v_uid UUID;
    v_caller_role TEXT;
    v_target_member public.organization_members;
    v_new_role TEXT;
    v_other_owners_count INTEGER;
BEGIN
    v_uid := auth.uid();
    IF v_uid IS NULL THEN
        RAISE EXCEPTION 'Authentication required to set member role' USING ERRCODE = '42501';
    END IF;

    v_new_role := lower(trim(p_new_role));
    IF v_new_role NOT IN ('owner', 'admin', 'manager', 'cashier') THEN
        RAISE EXCEPTION 'Invalid role % (must be owner, admin, manager, or cashier)', p_new_role USING ERRCODE = '22023';
    END IF;

    -- Get caller's role in this organization
    SELECT role INTO v_caller_role
    FROM public.organization_members
    WHERE organization_id = p_organization_id
      AND user_id = v_uid;

    IF NOT FOUND OR v_caller_role NOT IN ('owner', 'admin') THEN
        RAISE EXCEPTION 'Insufficient privilege: caller is not an admin or owner of organization %', p_organization_id USING ERRCODE = '42501';
    END IF;

    -- Find target member in the organization
    SELECT * INTO v_target_member
    FROM public.organization_members
    WHERE organization_id = p_organization_id
      AND user_id = p_target_user_id;

    IF NOT FOUND THEN
        RAISE EXCEPTION 'Target member not found in organization %', p_organization_id USING ERRCODE = 'P0002';
    END IF;

    -- Privilege hierarchy rules:
    -- 1. Promoting to 'owner' or modifying an existing 'owner' requires caller to be 'owner'
    IF (v_new_role = 'owner' OR v_target_member.role = 'owner') AND v_caller_role <> 'owner' THEN
        RAISE EXCEPTION 'Only an owner can promote to or modify an owner role' USING ERRCODE = '42501';
    END IF;

    -- 2. If demoting an owner, ensure at least one other owner remains
    IF v_target_member.role = 'owner' AND v_new_role <> 'owner' THEN
        SELECT COUNT(*) INTO v_other_owners_count
        FROM public.organization_members
        WHERE organization_id = p_organization_id
          AND role = 'owner'
          AND user_id <> p_target_user_id;

        IF v_other_owners_count < 1 THEN
            RAISE EXCEPTION 'Cannot demote the sole remaining owner of an organization' USING ERRCODE = '23514';
        END IF;
    END IF;

    -- Update member role
    UPDATE public.organization_members
    SET role = v_new_role,
        updated_at = now()
    WHERE id = v_target_member.id
    RETURNING * INTO v_target_member;

    RETURN v_target_member;
END;
$$;

-- ============================================================
-- 6. Function Execution Permissions Hardening
-- ============================================================

-- Revoke execution from PUBLIC on all 5 privileged functions
REVOKE ALL ON FUNCTION public.pair_device_to_register(UUID, TEXT) FROM PUBLIC;
REVOKE ALL ON FUNCTION public.revoke_device_pairing(UUID) FROM PUBLIC;
REVOKE ALL ON FUNCTION public.record_device_heartbeat(UUID, TEXT) FROM PUBLIC;
REVOKE ALL ON FUNCTION public.create_organization_with_initial_setup(TEXT, TEXT, TEXT, TEXT, TEXT, TEXT) FROM PUBLIC;
REVOKE ALL ON FUNCTION public.set_organization_member_role(UUID, UUID, TEXT) FROM PUBLIC;

-- Grant execution explicitly to authenticated role
GRANT EXECUTE ON FUNCTION public.pair_device_to_register(UUID, TEXT) TO authenticated;
GRANT EXECUTE ON FUNCTION public.revoke_device_pairing(UUID) TO authenticated;
GRANT EXECUTE ON FUNCTION public.record_device_heartbeat(UUID, TEXT) TO authenticated;
GRANT EXECUTE ON FUNCTION public.create_organization_with_initial_setup(TEXT, TEXT, TEXT, TEXT, TEXT, TEXT) TO authenticated;
GRANT EXECUTE ON FUNCTION public.set_organization_member_role(UUID, UUID, TEXT) TO authenticated;
