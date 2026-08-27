-- Supabase Migration: 002_organization_branch_member_schema.sql
-- F1.20 — Organization / Branch / Member Cloud Schema Hardening
-- Append-only migration: adds composite tenant integrity constraints,
-- domain check constraints, automatic timestamps, sole-owner mutation guard,
-- child-side supporting indexes, and performance indexes.
--
-- INVARIANT: 001_phase1_identity_and_rls.sql is immutable and untouched.

-- ============================================================
-- 1. Composite Foreign Key — Tenant Boundary Integrity
-- ============================================================

-- Enable composite FK targets: branches must be uniquely identifiable
-- by (organization_id, id) to serve as a composite FK reference.
ALTER TABLE public.branches
    DROP CONSTRAINT IF EXISTS uq_branches_org_id;

ALTER TABLE public.branches
    ADD CONSTRAINT uq_branches_org_id UNIQUE (organization_id, id);

-- users: organization_id + branch_id must reference the same org in branches.
ALTER TABLE public.users
    DROP CONSTRAINT IF EXISTS users_branch_id_fkey;

ALTER TABLE public.users
    DROP CONSTRAINT IF EXISTS fk_users_branch_org;

ALTER TABLE public.users
    ADD CONSTRAINT fk_users_branch_org
    FOREIGN KEY (organization_id, branch_id)
    REFERENCES public.branches(organization_id, id)
    ON DELETE CASCADE;

-- registers: organization_id + branch_id must reference the same org in branches.
ALTER TABLE public.registers
    DROP CONSTRAINT IF EXISTS registers_branch_id_fkey;

ALTER TABLE public.registers
    DROP CONSTRAINT IF EXISTS fk_registers_branch_org;

ALTER TABLE public.registers
    ADD CONSTRAINT fk_registers_branch_org
    FOREIGN KEY (organization_id, branch_id)
    REFERENCES public.branches(organization_id, id)
    ON DELETE CASCADE;

-- ============================================================
-- 2. Domain Check Constraints
-- ============================================================

-- Organizations
ALTER TABLE public.organizations
    DROP CONSTRAINT IF EXISTS chk_organizations_name;

ALTER TABLE public.organizations
    ADD CONSTRAINT chk_organizations_name
    CHECK (length(trim(name)) > 0 AND length(name) <= 255);

ALTER TABLE public.organizations
    DROP CONSTRAINT IF EXISTS chk_organizations_currency;

ALTER TABLE public.organizations
    ADD CONSTRAINT chk_organizations_currency
    CHECK (length(default_currency) = 3 AND default_currency ~ '^[A-Z]{3}$');

ALTER TABLE public.organizations
    DROP CONSTRAINT IF EXISTS chk_organizations_language;

ALTER TABLE public.organizations
    ADD CONSTRAINT chk_organizations_language
    CHECK (length(trim(default_language)) >= 2 AND length(default_language) <= 10
           AND default_language ~ '^[a-zA-Z0-9_-]+$');

-- Branches
ALTER TABLE public.branches
    DROP CONSTRAINT IF EXISTS chk_branches_name;

ALTER TABLE public.branches
    ADD CONSTRAINT chk_branches_name
    CHECK (length(trim(name)) > 0 AND length(name) <= 255);

ALTER TABLE public.branches
    DROP CONSTRAINT IF EXISTS chk_branches_currency;

ALTER TABLE public.branches
    ADD CONSTRAINT chk_branches_currency
    CHECK (length(currency) = 3 AND currency ~ '^[A-Z]{3}$');

-- Users (POS Staff)
ALTER TABLE public.users
    DROP CONSTRAINT IF EXISTS chk_users_full_name;

ALTER TABLE public.users
    ADD CONSTRAINT chk_users_full_name
    CHECK (length(trim(full_name)) > 0 AND length(full_name) <= 255);

ALTER TABLE public.users
    DROP CONSTRAINT IF EXISTS chk_users_username;

ALTER TABLE public.users
    ADD CONSTRAINT chk_users_username
    CHECK (username IS NULL
           OR (length(trim(username)) >= 3 AND length(username) <= 50
               AND username ~ '^[a-zA-Z0-9_.-]+$'));

-- ============================================================
-- 3. Unique Supabase Identity Mapping per Organization
-- ============================================================

-- A given Supabase auth identity can map to at most one POS user per org.
CREATE UNIQUE INDEX IF NOT EXISTS uq_users_org_supabase_user
    ON public.users (organization_id, supabase_user_id)
    WHERE supabase_user_id IS NOT NULL;

-- ============================================================
-- 4. Automatic updated_at Timestamp Trigger (Least Privilege)
-- ============================================================

-- Trigger function runs under invoking role context (no SECURITY DEFINER needed).
CREATE OR REPLACE FUNCTION public.handle_updated_at()
RETURNS TRIGGER
LANGUAGE plpgsql
SET search_path = public
AS $$
BEGIN
    NEW.updated_at = now();
    RETURN NEW;
END;
$$;

-- Attach to all tenant entities with updated_at columns
DROP TRIGGER IF EXISTS trg_updated_at_organizations ON public.organizations;
CREATE TRIGGER trg_updated_at_organizations
    BEFORE UPDATE ON public.organizations
    FOR EACH ROW
    EXECUTE FUNCTION public.handle_updated_at();

DROP TRIGGER IF EXISTS trg_updated_at_branches ON public.branches;
CREATE TRIGGER trg_updated_at_branches
    BEFORE UPDATE ON public.branches
    FOR EACH ROW
    EXECUTE FUNCTION public.handle_updated_at();

DROP TRIGGER IF EXISTS trg_updated_at_organization_members ON public.organization_members;
CREATE TRIGGER trg_updated_at_organization_members
    BEFORE UPDATE ON public.organization_members
    FOR EACH ROW
    EXECUTE FUNCTION public.handle_updated_at();

DROP TRIGGER IF EXISTS trg_updated_at_users ON public.users;
CREATE TRIGGER trg_updated_at_users
    BEFORE UPDATE ON public.users
    FOR EACH ROW
    EXECUTE FUNCTION public.handle_updated_at();

DROP TRIGGER IF EXISTS trg_updated_at_registers ON public.registers;
CREATE TRIGGER trg_updated_at_registers
    BEFORE UPDATE ON public.registers
    FOR EACH ROW
    EXECUTE FUNCTION public.handle_updated_at();

-- ============================================================
-- 5. Sole-Owner Mutation Guard (UPDATE + DELETE + Cascade Safety)
-- ============================================================

-- Upgrades prevent_orphaned_organization to handle:
-- 1. DELETE: prevents deleting the sole owner, while allowing ON DELETE CASCADE
--    when the parent organization itself is being deleted.
-- 2. UPDATE: validates demotion (OLD.role = 'owner' AND NEW.role <> 'owner')
--    and returns NEW so updates/role changes are preserved.
CREATE OR REPLACE FUNCTION public.prevent_orphaned_organization()
RETURNS TRIGGER
LANGUAGE plpgsql
SECURITY DEFINER
SET search_path = public
AS $$
DECLARE
    remaining_owners INTEGER;
BEGIN
    IF TG_OP = 'DELETE' THEN
        -- Allow cascade deletion when the parent organization itself is being deleted
        IF NOT EXISTS (SELECT 1 FROM public.organizations WHERE id = OLD.organization_id) THEN
            RETURN OLD;
        END IF;

        IF OLD.role = 'owner' THEN
            SELECT COUNT(*)
            INTO remaining_owners
            FROM public.organization_members om
            WHERE om.organization_id = OLD.organization_id
              AND om.role = 'owner'
              AND om.id <> OLD.id;

            IF remaining_owners < 1 THEN
                RAISE EXCEPTION 'Cannot delete the sole remaining owner of an organization';
            END IF;
        END IF;
        RETURN OLD;
    ELSIF TG_OP = 'UPDATE' THEN
        -- Guard against demoting the sole remaining owner
        IF OLD.role = 'owner' AND NEW.role <> 'owner' THEN
            SELECT COUNT(*)
            INTO remaining_owners
            FROM public.organization_members om
            WHERE om.organization_id = OLD.organization_id
              AND om.role = 'owner'
              AND om.id <> OLD.id;

            IF remaining_owners < 1 THEN
                RAISE EXCEPTION 'Cannot demote the sole remaining owner of an organization';
            END IF;
        END IF;
        RETURN NEW;
    END IF;
    RETURN NEW;
END;
$$;

DROP TRIGGER IF EXISTS trg_prevent_orphaned_organization ON public.organization_members;
CREATE TRIGGER trg_prevent_orphaned_organization
    BEFORE UPDATE OR DELETE ON public.organization_members
    FOR EACH ROW
    EXECUTE FUNCTION public.prevent_orphaned_organization();

-- ============================================================
-- 6. Child-Side Composite FK Supporting Indexes
-- ============================================================

CREATE INDEX IF NOT EXISTS idx_users_org_branch
    ON public.users (organization_id, branch_id);

CREATE INDEX IF NOT EXISTS idx_registers_org_branch
    ON public.registers (organization_id, branch_id);

-- ============================================================
-- 7. Performance Indexes
-- ============================================================

CREATE INDEX IF NOT EXISTS idx_branches_org_active
    ON public.branches (organization_id, is_active);

CREATE INDEX IF NOT EXISTS idx_branches_org_name
    ON public.branches (organization_id, name);

CREATE INDEX IF NOT EXISTS idx_users_org_active
    ON public.users (organization_id, is_active);

CREATE INDEX IF NOT EXISTS idx_members_org_role
    ON public.organization_members (organization_id, role);
