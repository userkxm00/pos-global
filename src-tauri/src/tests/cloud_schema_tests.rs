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

fn normalize_whitespace(input: &str) -> String {
    input.split_whitespace().collect::<Vec<_>>().join(" ")
}

// ============================================================
// Migration Structure — Composite Foreign Keys
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
    assert!(
        sql.contains("fk_users_branch_org"),
        "Expected composite FK fk_users_branch_org on users"
    );
    assert!(
        sql.contains("FOREIGN KEY (organization_id, branch_id) REFERENCES public.branches(organization_id, id)"),
        "Expected users composite FK to reference branches(organization_id, id)"
    );
}

#[test]
fn f1_20_migration_defines_composite_fk_registers_branch_org() {
    let sql = normalize_whitespace(SCHEMA_MIGRATION_SQL);
    assert!(
        sql.contains("fk_registers_branch_org"),
        "Expected composite FK fk_registers_branch_org on registers"
    );
    assert!(
        sql.contains("FOREIGN KEY (organization_id, branch_id) REFERENCES public.branches(organization_id, id)"),
        "Expected registers composite FK to reference branches(organization_id, id)"
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
    // Verify correct regex ^[A-Z]{3}$ — NOT ^[A-Z]{3}$*
    assert!(
        sql.contains("'^[A-Z]{3}$'"),
        "Expected correct ISO 4217 currency regex '^[A-Z]{3}$' (without trailing asterisk)"
    );
    assert!(
        !sql.contains("'^[A-Z]{3}$*'"),
        "MUST NOT contain incorrect regex '^[A-Z]{3}$*' with trailing asterisk"
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
// Migration Structure — updated_at Trigger
// ============================================================

#[test]
fn f1_20_migration_defines_updated_at_trigger_function() {
    let sql = normalize_whitespace(SCHEMA_MIGRATION_SQL);
    assert!(
        sql.contains("FUNCTION public.handle_updated_at()"),
        "Expected trigger function public.handle_updated_at()"
    );
    assert!(
        sql.contains("SECURITY DEFINER"),
        "Expected handle_updated_at to be SECURITY DEFINER"
    );
    assert!(
        sql.contains("SET search_path = public"),
        "Expected handle_updated_at to enforce SET search_path = public"
    );
    assert!(
        sql.contains("NEW.updated_at = now()"),
        "Expected handle_updated_at to set NEW.updated_at = now()"
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

    // All must be BEFORE UPDATE triggers
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
fn f1_20_migration_upgrades_sole_owner_trigger_to_cover_update() {
    let sql = normalize_whitespace(SCHEMA_MIGRATION_SQL);
    assert!(
        sql.contains("BEFORE UPDATE OR DELETE ON public.organization_members"),
        "Expected prevent_orphaned_organization trigger to fire on both UPDATE and DELETE"
    );
    assert!(
        sql.contains("trg_prevent_orphaned_organization"),
        "Expected trg_prevent_orphaned_organization trigger definition"
    );
}

// ============================================================
// Migration Structure — Performance Indexes
// ============================================================

#[test]
fn f1_20_migration_defines_performance_indexes() {
    let sql = normalize_whitespace(SCHEMA_MIGRATION_SQL);
    let expected_indexes = [
        "idx_branches_org_active",
        "idx_branches_org_name",
        "idx_users_org_active",
        "idx_members_org_role",
    ];

    for idx in expected_indexes {
        assert!(
            sql.contains(idx),
            "Expected performance index '{idx}' to be defined"
        );
    }
}

// ============================================================
// Migration Safety — Does NOT Modify 001
// ============================================================

#[test]
fn f1_20_migration_does_not_recreate_tables_or_rls_policies() {
    let sql = normalize_whitespace(SCHEMA_MIGRATION_SQL);
    // Must not contain CREATE TABLE (we are only ALTERing existing tables)
    assert!(
        !sql.contains("CREATE TABLE"),
        "Migration 002 must not CREATE TABLE — tables are defined in immutable 001"
    );
    // Must not contain ENABLE ROW LEVEL SECURITY (already done in 001)
    assert!(
        !sql.contains("ENABLE ROW LEVEL SECURITY"),
        "Migration 002 must not re-enable RLS — already enabled in 001"
    );
    // Must not contain CREATE POLICY (RLS policies are F1.22 scope)
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
        sql.contains("Organization deletion correctly cascades"),
        "Expected test assertion for cascading organization deletion"
    );
}
