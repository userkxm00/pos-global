-- Supabase Migration: 001_phase1_identity_and_rls.sql
-- F1.08 — Supabase RLS policies
-- Phase 1 Cloud Schema, Row Level Security (RLS), and Tenant Isolation Policies

-- 1. Extensions
CREATE EXTENSION IF NOT EXISTS "uuid-ossp";
CREATE EXTENSION IF NOT EXISTS "pgcrypto";

-- 2. Cloud Tables

-- Organizations (Tenancy Root)
CREATE TABLE IF NOT EXISTS public.organizations (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    name TEXT NOT NULL,
    default_currency TEXT NOT NULL DEFAULT 'USD',
    default_language TEXT NOT NULL DEFAULT 'en',
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

-- Organization Members (Identity Mapping & Cloud Roles)
CREATE TABLE IF NOT EXISTS public.organization_members (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    organization_id UUID NOT NULL REFERENCES public.organizations(id) ON DELETE CASCADE,
    user_id UUID NOT NULL REFERENCES auth.users(id) ON DELETE CASCADE,
    role TEXT NOT NULL CHECK (role IN ('owner', 'admin', 'manager', 'cashier')),
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT uq_organization_members_org_user UNIQUE (organization_id, user_id)
);

-- Branches
CREATE TABLE IF NOT EXISTS public.branches (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    organization_id UUID NOT NULL REFERENCES public.organizations(id) ON DELETE CASCADE,
    name TEXT NOT NULL,
    address TEXT,
    currency TEXT NOT NULL DEFAULT 'USD',
    is_active BOOLEAN NOT NULL DEFAULT true,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

-- Registers / POS Terminals
CREATE TABLE IF NOT EXISTS public.registers (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    organization_id UUID NOT NULL REFERENCES public.organizations(id) ON DELETE CASCADE,
    branch_id UUID NOT NULL REFERENCES public.branches(id) ON DELETE CASCADE,
    name TEXT NOT NULL,
    code TEXT NOT NULL,
    is_active BOOLEAN NOT NULL DEFAULT true,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT uq_registers_branch_code UNIQUE (branch_id, code)
);

-- POS Users / Staff Profiles
CREATE TABLE IF NOT EXISTS public.users (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    organization_id UUID NOT NULL REFERENCES public.organizations(id) ON DELETE CASCADE,
    branch_id UUID NOT NULL REFERENCES public.branches(id) ON DELETE CASCADE,
    supabase_user_id UUID REFERENCES auth.users(id) ON DELETE SET NULL,
    full_name TEXT NOT NULL,
    username TEXT,
    role TEXT NOT NULL CHECK (role IN ('admin', 'manager', 'cashier')),
    is_active BOOLEAN NOT NULL DEFAULT true,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT uq_users_org_username UNIQUE (organization_id, username)
);

-- Permissions Catalog
CREATE TABLE IF NOT EXISTS public.permissions (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    code TEXT NOT NULL UNIQUE,
    description TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

-- Role Permissions Mapping
CREATE TABLE IF NOT EXISTS public.role_permissions (
    role TEXT NOT NULL,
    permission_id UUID NOT NULL REFERENCES public.permissions(id) ON DELETE CASCADE,
    PRIMARY KEY (role, permission_id)
);

-- User Permission Overrides
CREATE TABLE IF NOT EXISTS public.user_permissions (
    user_id UUID NOT NULL REFERENCES public.users(id) ON DELETE CASCADE,
    permission_id UUID NOT NULL REFERENCES public.permissions(id) ON DELETE CASCADE,
    effect TEXT NOT NULL CHECK (effect IN ('allow', 'deny')),
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (user_id, permission_id)
);

-- 3. Indexes for RLS Performance
CREATE INDEX IF NOT EXISTS idx_organization_members_user_id ON public.organization_members(user_id);
CREATE INDEX IF NOT EXISTS idx_organization_members_org_id ON public.organization_members(organization_id);
CREATE INDEX IF NOT EXISTS idx_branches_organization_id ON public.branches(organization_id);
CREATE INDEX IF NOT EXISTS idx_registers_organization_id ON public.registers(organization_id);
CREATE INDEX IF NOT EXISTS idx_registers_branch_id ON public.registers(branch_id);
CREATE INDEX IF NOT EXISTS idx_users_organization_id ON public.users(organization_id);
CREATE INDEX IF NOT EXISTS idx_users_branch_id ON public.users(branch_id);
CREATE INDEX IF NOT EXISTS idx_users_supabase_user_id ON public.users(supabase_user_id);

-- 4. Security Helper Functions (SECURITY DEFINER with safe search_path)

CREATE OR REPLACE FUNCTION public.get_user_organization_ids()
RETURNS SETOF UUID
LANGUAGE sql
STABLE
SECURITY DEFINER
SET search_path = public
AS $$
    SELECT organization_id
    FROM public.organization_members
    WHERE user_id = auth.uid();
$$;

CREATE OR REPLACE FUNCTION public.is_org_member(target_org_id UUID)
RETURNS BOOLEAN
LANGUAGE sql
STABLE
SECURITY DEFINER
SET search_path = public
AS $$
    SELECT EXISTS (
        SELECT 1
        FROM public.organization_members
        WHERE organization_id = target_org_id
          AND user_id = auth.uid()
    );
$$;

CREATE OR REPLACE FUNCTION public.is_org_admin_or_owner(target_org_id UUID)
RETURNS BOOLEAN
LANGUAGE sql
STABLE
SECURITY DEFINER
SET search_path = public
AS $$
    SELECT EXISTS (
        SELECT 1
        FROM public.organization_members
        WHERE organization_id = target_org_id
          AND user_id = auth.uid()
          AND role IN ('owner', 'admin')
    );
$$;

CREATE OR REPLACE FUNCTION public.is_org_manager_or_above(target_org_id UUID)
RETURNS BOOLEAN
LANGUAGE sql
STABLE
SECURITY DEFINER
SET search_path = public
AS $$
    SELECT EXISTS (
        SELECT 1
        FROM public.organization_members
        WHERE organization_id = target_org_id
          AND user_id = auth.uid()
          AND role IN ('owner', 'admin', 'manager')
    );
$$;

-- 5. Enable Row Level Security (RLS) on All Cloud Tables
ALTER TABLE public.organizations ENABLE ROW LEVEL SECURITY;
ALTER TABLE public.organization_members ENABLE ROW LEVEL SECURITY;
ALTER TABLE public.branches ENABLE ROW LEVEL SECURITY;
ALTER TABLE public.registers ENABLE ROW LEVEL SECURITY;
ALTER TABLE public.users ENABLE ROW LEVEL SECURITY;
ALTER TABLE public.permissions ENABLE ROW LEVEL SECURITY;
ALTER TABLE public.role_permissions ENABLE ROW LEVEL SECURITY;
ALTER TABLE public.user_permissions ENABLE ROW LEVEL SECURITY;

-- 6. Row Level Security Policies

-- --- Organizations Policies ---
DROP POLICY IF EXISTS organizations_select_policy ON public.organizations;
CREATE POLICY organizations_select_policy ON public.organizations
    FOR SELECT
    USING (id IN (SELECT public.get_user_organization_ids()));

DROP POLICY IF EXISTS organizations_insert_policy ON public.organizations;
CREATE POLICY organizations_insert_policy ON public.organizations
    FOR INSERT
    WITH CHECK (auth.uid() IS NOT NULL);

DROP POLICY IF EXISTS organizations_update_policy ON public.organizations;
CREATE POLICY organizations_update_policy ON public.organizations
    FOR UPDATE
    USING (public.is_org_admin_or_owner(id))
    WITH CHECK (public.is_org_admin_or_owner(id));

DROP POLICY IF EXISTS organizations_delete_policy ON public.organizations;
CREATE POLICY organizations_delete_policy ON public.organizations
    FOR DELETE
    USING (EXISTS (
        SELECT 1 FROM public.organization_members
        WHERE organization_id = id AND user_id = auth.uid() AND role = 'owner'
    ));

-- --- Organization Members Policies ---
DROP POLICY IF EXISTS members_select_policy ON public.organization_members;
CREATE POLICY members_select_policy ON public.organization_members
    FOR SELECT
    USING (organization_id IN (SELECT public.get_user_organization_ids()));

DROP POLICY IF EXISTS members_insert_policy ON public.organization_members;
CREATE POLICY members_insert_policy ON public.organization_members
    FOR INSERT
    WITH CHECK (public.is_org_admin_or_owner(organization_id));

DROP POLICY IF EXISTS members_update_policy ON public.organization_members;
CREATE POLICY members_update_policy ON public.organization_members
    FOR UPDATE
    USING (public.is_org_admin_or_owner(organization_id))
    WITH CHECK (public.is_org_admin_or_owner(organization_id));

DROP POLICY IF EXISTS members_delete_policy ON public.organization_members;
CREATE POLICY members_delete_policy ON public.organization_members
    FOR DELETE
    USING (public.is_org_admin_or_owner(organization_id) OR user_id = auth.uid());

-- --- Branches Policies ---
DROP POLICY IF EXISTS branches_select_policy ON public.branches;
CREATE POLICY branches_select_policy ON public.branches
    FOR SELECT
    USING (organization_id IN (SELECT public.get_user_organization_ids()));

DROP POLICY IF EXISTS branches_insert_policy ON public.branches;
CREATE POLICY branches_insert_policy ON public.branches
    FOR INSERT
    WITH CHECK (public.is_org_admin_or_owner(organization_id));

DROP POLICY IF EXISTS branches_update_policy ON public.branches;
CREATE POLICY branches_update_policy ON public.branches
    FOR UPDATE
    USING (public.is_org_admin_or_owner(organization_id))
    WITH CHECK (public.is_org_admin_or_owner(organization_id));

DROP POLICY IF EXISTS branches_delete_policy ON public.branches;
CREATE POLICY branches_delete_policy ON public.branches
    FOR DELETE
    USING (public.is_org_admin_or_owner(organization_id));

-- --- Registers Policies ---
DROP POLICY IF EXISTS registers_select_policy ON public.registers;
CREATE POLICY registers_select_policy ON public.registers
    FOR SELECT
    USING (organization_id IN (SELECT public.get_user_organization_ids()));

DROP POLICY IF EXISTS registers_insert_policy ON public.registers;
CREATE POLICY registers_insert_policy ON public.registers
    FOR INSERT
    WITH CHECK (public.is_org_manager_or_above(organization_id));

DROP POLICY IF EXISTS registers_update_policy ON public.registers;
CREATE POLICY registers_update_policy ON public.registers
    FOR UPDATE
    USING (public.is_org_manager_or_above(organization_id))
    WITH CHECK (public.is_org_manager_or_above(organization_id));

DROP POLICY IF EXISTS registers_delete_policy ON public.registers;
CREATE POLICY registers_delete_policy ON public.registers
    FOR DELETE
    USING (public.is_org_admin_or_owner(organization_id));

-- --- Users Policies ---
DROP POLICY IF EXISTS users_select_policy ON public.users;
CREATE POLICY users_select_policy ON public.users
    FOR SELECT
    USING (organization_id IN (SELECT public.get_user_organization_ids()));

DROP POLICY IF EXISTS users_insert_policy ON public.users;
CREATE POLICY users_insert_policy ON public.users
    FOR INSERT
    WITH CHECK (public.is_org_admin_or_owner(organization_id));

DROP POLICY IF EXISTS users_update_policy ON public.users;
CREATE POLICY users_update_policy ON public.users
    FOR UPDATE
    USING (public.is_org_admin_or_owner(organization_id))
    WITH CHECK (public.is_org_admin_or_owner(organization_id));

DROP POLICY IF EXISTS users_delete_policy ON public.users;
CREATE POLICY users_delete_policy ON public.users
    FOR DELETE
    USING (public.is_org_admin_or_owner(organization_id));

-- --- Permissions Catalog & Mapping Policies ---
DROP POLICY IF EXISTS permissions_select_policy ON public.permissions;
CREATE POLICY permissions_select_policy ON public.permissions
    FOR SELECT
    USING (auth.uid() IS NOT NULL);

DROP POLICY IF EXISTS role_permissions_select_policy ON public.role_permissions;
CREATE POLICY role_permissions_select_policy ON public.role_permissions
    FOR SELECT
    USING (auth.uid() IS NOT NULL);

DROP POLICY IF EXISTS user_permissions_select_policy ON public.user_permissions;
CREATE POLICY user_permissions_select_policy ON public.user_permissions
    FOR SELECT
    USING (EXISTS (
        SELECT 1 FROM public.users u
        WHERE u.id = user_id AND u.organization_id IN (SELECT public.get_user_organization_ids())
    ));

DROP POLICY IF EXISTS user_permissions_mutate_policy ON public.user_permissions;
CREATE POLICY user_permissions_mutate_policy ON public.user_permissions
    FOR ALL
    USING (EXISTS (
        SELECT 1 FROM public.users u
        WHERE u.id = user_id AND public.is_org_admin_or_owner(u.organization_id)
    ))
    WITH CHECK (EXISTS (
        SELECT 1 FROM public.users u
        WHERE u.id = user_id AND public.is_org_admin_or_owner(u.organization_id)
    ));

-- 7. Seed Authoritative Permission Catalog (matching local migration 004)
INSERT INTO public.permissions (code, description) VALUES
    ('sales.create', 'Create sales'),
    ('sales.refund', 'Refund sales'),
    ('sales.void', 'Void sales'),
    ('inventory.adjust', 'Adjust inventory'),
    ('inventory.transfer', 'Transfer inventory'),
    ('products.manage', 'Manage products'),
    ('purchases.manage', 'Manage purchases'),
    ('customers.manage', 'Manage customers'),
    ('debts.manage', 'Manage customer debts'),
    ('cash.open', 'Open cash shift'),
    ('cash.close', 'Close cash shift'),
    ('cash.adjust', 'Adjust cash'),
    ('reports.view', 'View reports'),
    ('reports.export', 'Export reports'),
    ('users.manage', 'Manage users'),
    ('settings.manage', 'Manage settings'),
    ('license.manage', 'Manage license')
ON CONFLICT (code) DO NOTHING;

-- Seed Authoritative Role Permissions
INSERT INTO public.role_permissions (role, permission_id)
SELECT 'admin', id FROM public.permissions
ON CONFLICT DO NOTHING;

INSERT INTO public.role_permissions (role, permission_id)
SELECT 'manager', id FROM public.permissions
WHERE code NOT IN ('users.manage', 'license.manage')
ON CONFLICT DO NOTHING;

INSERT INTO public.role_permissions (role, permission_id)
SELECT 'cashier', id FROM public.permissions
WHERE code IN ('sales.create', 'customers.manage', 'reports.view', 'cash.open', 'cash.close')
ON CONFLICT DO NOTHING;
