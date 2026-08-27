// Static structural validation tests for F1.20 — Organization / Branch / Member Cloud Schema.
// These tests validate that the SQL migration and test files contain the required
// structural definitions (constraints, triggers, indexes, functions).
//
// IMPORTANT: These are STATIC text analysis tests only. They do NOT execute SQL
// against a live PostgreSQL instance. Runtime behavioral correctness is verified
// by supabase/tests/organization_branch_member_test.sql.

const SCHEMA_MIGRATION_SQL: &str =
    include_str!("../../../supabase/migrations/002_organization_branch_member_schema.sql");
const SCHEMA_TEST_SQL: &str =
    include_str!("../../../supabase/tests/organization_branch_member_test.sql");
const DEVICE_REGISTER_MIGRATION_SQL: &str =
    include_str!("../../../supabase/migrations/003_device_register_cloud_identity.sql");
const DEVICE_REGISTER_TEST_SQL: &str =
    include_str!("../../../supabase/tests/device_register_identity_test.sql");

fn normalize_whitespace(input: &str) -> String {
    input.split_whitespace().collect::<Vec<_>>().join(" ")
}

// ============================================================
// Migration Structure — Composite Foreign Keys (Table-Specific)
// ============================================================

#[test]
fn f1_20_migration_defines_composite_unique_constraint_on_branches() {
    let sql = normalize_whitespace(SCHEMA_MIGRATION_SQL);
    assert!(
        sql.contains("uq_branches_org_id UNIQUE (organization_id, id)"),
        "Expected UNIQUE (organization_id, id) constraint on branches for composite FK target"
    );
}

#[test]
fn f1_20_migration_defines_composite_fk_users_branch_org() {
    let sql = normalize_whitespace(SCHEMA_MIGRATION_SQL);
    let expected = "ALTER TABLE public.users ADD CONSTRAINT fk_users_branch_org FOREIGN KEY (organization_id, branch_id) REFERENCES public.branches(organization_id, id) ON DELETE CASCADE;";
    assert!(
        sql.contains(expected),
        "Expected table-specific composite FK definition for users: {expected}"
    );
}

#[test]
fn f1_20_migration_defines_composite_fk_registers_branch_org() {
    let sql = normalize_whitespace(SCHEMA_MIGRATION_SQL);
    let expected = "ALTER TABLE public.registers ADD CONSTRAINT fk_registers_branch_org FOREIGN KEY (organization_id, branch_id) REFERENCES public.branches(organization_id, id) ON DELETE CASCADE;";
    assert!(
        sql.contains(expected),
        "Expected table-specific composite FK definition for registers: {expected}"
    );
}

// ============================================================
// Migration Structure — Domain Check Constraints
// ============================================================

#[test]
fn f1_20_migration_defines_organization_check_constraints() {
    let sql = normalize_whitespace(SCHEMA_MIGRATION_SQL);
    assert!(
        sql.contains("chk_organizations_name"),
        "Expected check constraint chk_organizations_name on organizations"
    );
    assert!(
        sql.contains("chk_organizations_currency"),
        "Expected check constraint chk_organizations_currency on organizations"
    );
    assert!(
        sql.contains("chk_organizations_language"),
        "Expected check constraint chk_organizations_language on organizations"
    );
}

#[test]
fn f1_20_migration_uses_correct_currency_regex() {
    let sql = normalize_whitespace(SCHEMA_MIGRATION_SQL);
    assert!(
        sql.contains("'^[A-Z]{3}$'"),
        "Expected correct ISO 4217 currency regex '^[A-Z]{{3}}$' (without trailing asterisk)"
    );
    assert!(
        !sql.contains("'^[A-Z]{3}$*'"),
        "MUST NOT contain incorrect regex '^[A-Z]{{3}}$*' with trailing asterisk"
    );
}

#[test]
fn f1_20_migration_defines_branch_check_constraints() {
    let sql = normalize_whitespace(SCHEMA_MIGRATION_SQL);
    assert!(
        sql.contains("chk_branches_name"),
        "Expected check constraint chk_branches_name on branches"
    );
    assert!(
        sql.contains("chk_branches_currency"),
        "Expected check constraint chk_branches_currency on branches"
    );
}

#[test]
fn f1_20_migration_defines_user_check_constraints() {
    let sql = normalize_whitespace(SCHEMA_MIGRATION_SQL);
    assert!(
        sql.contains("chk_users_full_name"),
        "Expected check constraint chk_users_full_name on users"
    );
    assert!(
        sql.contains("chk_users_username"),
        "Expected check constraint chk_users_username on users"
    );
}

// ============================================================
// Migration Structure — Unique Supabase Identity Mapping
// ============================================================

#[test]
fn f1_20_migration_defines_unique_supabase_user_index() {
    let sql = normalize_whitespace(SCHEMA_MIGRATION_SQL);
    assert!(
        sql.contains("uq_users_org_supabase_user"),
        "Expected unique index uq_users_org_supabase_user"
    );
    assert!(
        sql.contains("(organization_id, supabase_user_id)"),
        "Expected index on (organization_id, supabase_user_id)"
    );
    assert!(
        sql.contains("WHERE supabase_user_id IS NOT NULL"),
        "Expected partial index filtering NULL supabase_user_id values"
    );
}

// ============================================================
// Migration Structure — updated_at Trigger (Least Privilege)
// ============================================================

#[test]
fn f1_20_migration_defines_updated_at_trigger_function() {
    let sql = normalize_whitespace(SCHEMA_MIGRATION_SQL);
    assert!(
        sql.contains("FUNCTION public.handle_updated_at()"),
        "Expected trigger function public.handle_updated_at()"
    );
    assert!(
        sql.contains("SET search_path = public"),
        "Expected handle_updated_at to enforce SET search_path = public"
    );
    assert!(
        sql.contains("NEW.updated_at = now()"),
        "Expected handle_updated_at to set NEW.updated_at = now()"
    );
    assert!(
        !sql.contains(
            "FUNCTION public.handle_updated_at() RETURNS TRIGGER LANGUAGE plpgsql SECURITY DEFINER"
        ),
        "handle_updated_at should not use SECURITY DEFINER (least privilege)"
    );
}

#[test]
fn f1_20_migration_attaches_updated_at_triggers_to_all_tenant_entities() {
    let sql = normalize_whitespace(SCHEMA_MIGRATION_SQL);
    let expected_triggers = [
        "trg_updated_at_organizations",
        "trg_updated_at_branches",
        "trg_updated_at_organization_members",
        "trg_updated_at_users",
        "trg_updated_at_registers",
    ];

    for trigger in expected_triggers {
        assert!(
            sql.contains(trigger),
            "Expected updated_at trigger '{trigger}' to be defined"
        );
    }

    let before_update_count = sql.matches("BEFORE UPDATE ON public.").count();
    assert!(
        before_update_count >= 5,
        "Expected at least 5 BEFORE UPDATE triggers for updated_at, found {before_update_count}"
    );
}

// ============================================================
// Migration Structure — Sole-Owner Mutation Guard
// ============================================================

#[test]
fn f1_20_migration_upgrades_sole_owner_trigger_to_cover_update_and_cascade() {
    let sql = normalize_whitespace(SCHEMA_MIGRATION_SQL);
    assert!(
        sql.contains("BEFORE UPDATE OR DELETE ON public.organization_members"),
        "Expected prevent_orphaned_organization trigger to fire on both UPDATE and DELETE"
    );
    assert!(
        sql.contains("trg_prevent_orphaned_organization"),
        "Expected trg_prevent_orphaned_organization trigger definition"
    );
    assert!(
        sql.contains(
            "NOT EXISTS (SELECT 1 FROM public.organizations WHERE id = OLD.organization_id)"
        ),
        "Expected cascade bypass when parent organization is deleted"
    );
    assert!(
        sql.contains(
            "OLD.role = 'owner' AND (NEW.role <> 'owner' OR NEW.organization_id <> OLD.organization_id)"
        ),
        "Expected role demotion and transfer check on UPDATE"
    );
    assert!(
        sql.contains(
            "PERFORM 1 FROM public.organizations WHERE id = OLD.organization_id FOR NO KEY UPDATE;"
        ),
        "Expected row lock on organization to serialize concurrent owner mutations"
    );
    assert!(
        sql.contains("RETURN NEW;"),
        "Expected RETURN NEW on UPDATE so member updates are preserved"
    );
}

// ============================================================
// Migration Structure — Performance & Child-Side Indexes
// ============================================================

#[test]
fn f1_20_migration_defines_performance_and_supporting_indexes() {
    let sql = normalize_whitespace(SCHEMA_MIGRATION_SQL);
    let expected_indexes = [
        "idx_users_org_branch",
        "idx_registers_org_branch",
        "idx_branches_org_active",
        "idx_branches_org_name",
        "idx_users_org_active",
        "idx_members_org_role",
    ];

    for idx in expected_indexes {
        assert!(
            sql.contains(idx),
            "Expected index '{idx}' to be defined in migration 002"
        );
    }
}

// ============================================================
// Migration Safety — Does NOT Modify 001
// ============================================================

#[test]
fn f1_20_migration_does_not_recreate_tables_or_rls_policies() {
    let sql = normalize_whitespace(SCHEMA_MIGRATION_SQL);
    assert!(
        !sql.contains("CREATE TABLE"),
        "Migration 002 must not CREATE TABLE — tables are defined in immutable 001"
    );
    assert!(
        !sql.contains("ENABLE ROW LEVEL SECURITY"),
        "Migration 002 must not re-enable RLS — already enabled in 001"
    );
    assert!(
        !sql.contains("CREATE POLICY"),
        "Migration 002 must not create RLS policies — that is F1.22 scope"
    );
}

// ============================================================
// Test Suite Coverage — Behavioral Assertions Exist
// ============================================================

#[test]
fn f1_20_test_suite_covers_composite_fk_cross_tenant_rejection() {
    let sql = normalize_whitespace(SCHEMA_TEST_SQL);
    assert!(
        sql.contains("Cross-tenant user-branch attachment correctly rejected by composite FK"),
        "Expected test assertion for cross-tenant user-branch FK rejection"
    );
    assert!(
        sql.contains("Cross-tenant register-branch attachment correctly rejected by composite FK"),
        "Expected test assertion for cross-tenant register-branch FK rejection"
    );
}

#[test]
fn f1_20_test_suite_covers_sole_owner_protection() {
    let sql = normalize_whitespace(SCHEMA_TEST_SQL);
    assert!(
        sql.contains("Sole owner deletion correctly prevented by trigger"),
        "Expected test assertion for sole-owner deletion prevention"
    );
    assert!(
        sql.contains("Sole owner role demotion correctly prevented by trigger"),
        "Expected test assertion for sole-owner role demotion prevention"
    );
    assert!(
        sql.contains("Co-owner deletion correctly permitted when another owner remains"),
        "Expected test assertion for co-owner deletion permission"
    );
    assert!(
        sql.contains("SQLSTATE 'TF001'"),
        "Expected dedicated test failure SQLSTATE TF001"
    );
}

#[test]
fn f1_20_test_suite_covers_domain_check_constraints() {
    let sql = normalize_whitespace(SCHEMA_TEST_SQL);
    assert!(
        sql.contains("Empty organization name correctly rejected"),
        "Expected test assertion for empty org name rejection"
    );
    assert!(
        sql.contains("Lowercase currency code correctly rejected"),
        "Expected test assertion for invalid currency format rejection"
    );
    assert!(
        sql.contains("Empty branch name correctly rejected"),
        "Expected test assertion for empty branch name rejection"
    );
    assert!(
        sql.contains("Empty user full_name correctly rejected"),
        "Expected test assertion for empty user full_name rejection"
    );
}

#[test]
fn f1_20_test_suite_covers_updated_at_trigger() {
    let sql = normalize_whitespace(SCHEMA_TEST_SQL);
    assert!(
        sql.contains("updated_at trigger correctly advances timestamp on UPDATE"),
        "Expected test assertion for updated_at trigger behavior"
    );
}

#[test]
fn f1_20_test_suite_covers_unique_supabase_identity() {
    let sql = normalize_whitespace(SCHEMA_TEST_SQL);
    assert!(
        sql.contains("Duplicate supabase_user_id in same org correctly rejected"),
        "Expected test assertion for duplicate supabase_user_id rejection"
    );
}

#[test]
fn f1_20_test_suite_covers_cascade_deletion() {
    let sql = normalize_whitespace(SCHEMA_TEST_SQL);
    assert!(
        sql.contains(
            "Organization deletion correctly cascades to branches, members, users, and registers"
        ),
        "Expected test assertion for complete cascading organization deletion"
    );
}

// ============================================================
// F1.21 — Device / Register Cloud Identity Migration Assertions
// ============================================================

#[test]
fn f1_21_migration_defines_device_identity_columns() {
    let sql = normalize_whitespace(DEVICE_REGISTER_MIGRATION_SQL);
    assert!(
        sql.contains("ADD COLUMN IF NOT EXISTS device_identifier TEXT;"),
        "Expected device_identifier column on registers"
    );
    assert!(
        sql.contains(
            "ADD COLUMN IF NOT EXISTS device_pairing_status TEXT NOT NULL DEFAULT 'unpaired';"
        ),
        "Expected device_pairing_status column on registers"
    );
    assert!(
        sql.contains("ADD COLUMN IF NOT EXISTS device_paired_at TIMESTAMPTZ;"),
        "Expected device_paired_at column on registers"
    );
    assert!(
        sql.contains("ADD COLUMN IF NOT EXISTS device_last_seen_at TIMESTAMPTZ;"),
        "Expected device_last_seen_at column on registers"
    );
}

#[test]
fn f1_21_migration_defines_register_domain_check_constraints() {
    let sql = normalize_whitespace(DEVICE_REGISTER_MIGRATION_SQL);
    assert!(
        sql.contains("chk_registers_name")
            && sql.contains("VALIDATE CONSTRAINT chk_registers_name;"),
        "Expected check constraint chk_registers_name and validation on registers"
    );
    assert!(
        sql.contains("chk_registers_code")
            && sql.contains("VALIDATE CONSTRAINT chk_registers_code;"),
        "Expected check constraint chk_registers_code and validation on registers"
    );
    assert!(
        sql.contains("chk_registers_device_identifier")
            && sql.contains("VALIDATE CONSTRAINT chk_registers_device_identifier;"),
        "Expected check constraint chk_registers_device_identifier and validation on registers"
    );
    assert!(
        sql.contains("chk_registers_pairing_status")
            && sql.contains("VALIDATE CONSTRAINT chk_registers_pairing_status;"),
        "Expected check constraint chk_registers_pairing_status and validation on registers"
    );
    assert!(
        sql.contains("chk_registers_pairing_coherence")
            && sql.contains("VALIDATE CONSTRAINT chk_registers_pairing_coherence;"),
        "Expected check constraint chk_registers_pairing_coherence and validation on registers"
    );
}

#[test]
fn f1_21_migration_defines_device_uniqueness_indexes() {
    let sql = normalize_whitespace(DEVICE_REGISTER_MIGRATION_SQL);
    assert!(
        sql.contains("DROP INDEX IF EXISTS public.uq_registers_org_device;"),
        "Expected cleanup of redundant tenant-scoped unique index uq_registers_org_device"
    );
    assert!(
        sql.contains("CREATE UNIQUE INDEX uq_registers_global_active_device ON public.registers (device_identifier) WHERE device_identifier IS NOT NULL AND device_pairing_status = 'paired';"),
        "Expected global active device unique index uq_registers_global_active_device"
    );
}

#[test]
fn f1_21_migration_defines_supporting_performance_indexes() {
    let sql = normalize_whitespace(DEVICE_REGISTER_MIGRATION_SQL);
    assert!(
        sql.contains("idx_registers_device_id"),
        "Expected supporting index idx_registers_device_id"
    );
    assert!(
        sql.contains("idx_registers_branch_active"),
        "Expected supporting index idx_registers_branch_active"
    );
    assert!(
        sql.contains("idx_registers_pairing_status"),
        "Expected supporting index idx_registers_pairing_status"
    );
}

// ============================================================
// F1.21 — Device / Register Test Suite Coverage Assertions
// ============================================================

#[test]
fn f1_21_test_suite_covers_register_domain_constraints() {
    let sql = normalize_whitespace(DEVICE_REGISTER_TEST_SQL);
    assert!(
        sql.contains("Empty register name correctly rejected"),
        "Expected test assertion for empty register name rejection"
    );
    assert!(
        sql.contains("Whitespace-only register name correctly rejected"),
        "Expected test assertion for whitespace-only register name rejection"
    );
    assert!(
        sql.contains("256-character register name correctly rejected"),
        "Expected test assertion for 256-character register name rejection"
    );
    assert!(
        sql.contains("Empty register code correctly rejected"),
        "Expected test assertion for empty register code rejection"
    );
    assert!(
        sql.contains("Invalid register code characters correctly rejected"),
        "Expected test assertion for invalid register code characters rejection"
    );
}

#[test]
fn f1_21_test_suite_covers_device_identifier_and_pairing_lifecycle() {
    let sql = normalize_whitespace(DEVICE_REGISTER_TEST_SQL);
    assert!(
        sql.contains("Short device identifier correctly rejected"),
        "Expected test assertion for short device identifier rejection"
    );
    assert!(
        sql.contains("Invalid device identifier characters correctly rejected"),
        "Expected test assertion for invalid device identifier characters rejection"
    );
    assert!(
        sql.contains("Invalid pairing status correctly rejected"),
        "Expected test assertion for invalid pairing status rejection"
    );
    assert!(
        sql.contains("Paired status without device_identifier correctly rejected"),
        "Expected test assertion for paired status without device_identifier rejection"
    );
    assert!(
        sql.contains("Unpaired status with device_identifier correctly rejected"),
        "Expected test assertion for unpaired status with device_identifier rejection"
    );
}

#[test]
fn f1_21_test_suite_covers_active_device_uniqueness_and_revocation() {
    let sql = normalize_whitespace(DEVICE_REGISTER_TEST_SQL);
    assert!(
        sql.contains("Duplicate active device in same org correctly rejected by unique index"),
        "Expected test assertion for duplicate active device in same org rejection"
    );
    assert!(
        sql.contains(
            "Cross-tenant duplicate active device correctly rejected by global unique index"
        ),
        "Expected test assertion for cross-tenant duplicate active device rejection"
    );
    assert!(
        sql.contains("Device re-pairing succeeded after previous binding was revoked"),
        "Expected test assertion for device re-pairing after revocation"
    );
}

#[test]
fn f1_21_test_suite_covers_composite_fk_and_cascade() {
    let sql = normalize_whitespace(DEVICE_REGISTER_TEST_SQL);
    assert!(
        sql.contains("Cross-tenant branch register correctly rejected by composite FK"),
        "Expected test assertion for cross-tenant branch register rejection"
    );
    assert!(
        sql.contains("Organization deletion correctly cascades to registers"),
        "Expected test assertion for cascading organization deletion to registers"
    );
    assert!(
        sql.contains("updated_at trigger correctly advances on register UPDATE"),
        "Expected test assertion for updated_at trigger behavior on registers"
    );
}
