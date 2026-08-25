// Supabase RLS Migration & Policy Static Validation Tests.
// F1.08 — Supabase RLS policies

use crate::permission::PERMISSION_CATALOG;

const RLS_MIGRATION_SQL: &str =
    include_str!("../../../supabase/migrations/001_phase1_identity_and_rls.sql");
const RLS_TEST_SQL: &str = include_str!("../../../supabase/tests/rls_policies_test.sql");

fn normalize_whitespace(input: &str) -> String {
    input.split_whitespace().collect::<Vec<_>>().join(" ")
}

#[test]
fn rls_migration_enables_row_level_security_on_all_tables() {
    let normalized_sql = normalize_whitespace(RLS_MIGRATION_SQL);
    let expected_tables = [
        "organizations",
        "organization_members",
        "branches",
        "registers",
        "users",
        "permissions",
        "role_permissions",
        "user_permissions",
    ];

    for table in expected_tables {
        let pattern = format!("ALTER TABLE public.{table} ENABLE ROW LEVEL SECURITY;");
        assert!(
            normalized_sql.contains(&pattern),
            "Expected RLS to be enabled on table public.{table}"
        );
    }
}

#[test]
fn rls_migration_defines_security_definer_functions_with_safe_search_path() {
    let normalized_sql = normalize_whitespace(RLS_MIGRATION_SQL);
    let expected_functions = [
        "get_user_organization_ids",
        "is_org_member",
        "is_org_admin_or_owner",
        "is_org_manager_or_above",
        "can_delete_organization_member",
        "prevent_orphaned_organization",
        "handle_new_organization_owner",
    ];

    for func in expected_functions {
        assert!(
            normalized_sql.contains(&format!("FUNCTION public.{func}")),
            "Expected function public.{func} to be defined"
        );
    }

    assert!(
        normalized_sql.contains("SECURITY DEFINER"),
        "Expected helper functions to use SECURITY DEFINER"
    );
    assert!(
        normalized_sql.contains("SET search_path = public"),
        "Expected helper functions to enforce search_path = public against hijacking"
    );
}

#[test]
fn rls_migration_enforces_sole_owner_and_branch_boundary_protection() {
    let normalized_sql = normalize_whitespace(RLS_MIGRATION_SQL);

    assert!(
        normalized_sql.contains("can_delete_organization_member"),
        "Expected can_delete_organization_member function to be used in delete policy"
    );
    assert!(
        normalized_sql.contains("prevent_orphaned_organization"),
        "Expected prevent_orphaned_organization trigger to protect against zero-owner organizations"
    );
    assert!(
        normalized_sql.contains("trg_prevent_orphaned_organization"),
        "Expected trg_prevent_orphaned_organization trigger definition"
    );
    assert!(
        normalized_sql.contains(
            "b.id = registers.branch_id AND b.organization_id = registers.organization_id"
        ),
        "Expected table-qualified branch tenant boundary check in registers mutation policies"
    );
    assert!(
        normalized_sql
            .contains("b.id = users.branch_id AND b.organization_id = users.organization_id"),
        "Expected table-qualified branch tenant boundary check in users mutation policies"
    );
}

#[test]
fn rls_migration_defines_explicit_policies_for_all_tenant_entities() {
    let normalized_sql = normalize_whitespace(RLS_MIGRATION_SQL);
    let expected_policies = [
        "organizations_select_policy",
        "organizations_insert_policy",
        "organizations_update_policy",
        "organizations_delete_policy",
        "members_select_policy",
        "members_insert_policy",
        "members_update_policy",
        "members_delete_policy",
        "branches_select_policy",
        "branches_insert_policy",
        "branches_update_policy",
        "branches_delete_policy",
        "registers_select_policy",
        "registers_insert_policy",
        "registers_update_policy",
        "registers_delete_policy",
        "users_select_policy",
        "users_insert_policy",
        "users_update_policy",
        "users_delete_policy",
        "permissions_select_policy",
        "role_permissions_select_policy",
        "user_permissions_select_policy",
        "user_permissions_mutate_policy",
    ];

    for policy in expected_policies {
        assert!(
            normalized_sql.contains(policy),
            "Expected policy '{policy}' to be defined in RLS migration"
        );
    }
}

#[test]
fn rls_permission_catalog_matches_canonical_rust_catalog() {
    let normalized_sql = normalize_whitespace(RLS_MIGRATION_SQL);
    for entry in PERMISSION_CATALOG {
        let code_literal = format!("'{}'", entry.code);
        assert!(
            normalized_sql.contains(&code_literal),
            "Expected permission code '{}' in Supabase RLS seed catalog",
            entry.code
        );
    }
}

#[test]
fn rls_test_suite_covers_authenticated_execution_and_sole_owner_cases() {
    let normalized_test_sql = normalize_whitespace(RLS_TEST_SQL);

    assert!(
        normalized_test_sql.contains("SET LOCAL ROLE authenticated"),
        "Expected RLS tests to execute under authenticated role"
    );
    assert!(
        normalized_test_sql.contains("auth.uid()"),
        "Expected test harness to validate and provide auth.uid() function"
    );
    assert!(
        normalized_test_sql.contains("RLS SECURITY VIOLATION"),
        "Expected negative security assertions in RLS test suite"
    );
    assert!(
        normalized_test_sql.contains("User A Owner saw Org B"),
        "Expected cross-organization negative assertion"
    );
    assert!(
        normalized_test_sql.contains("Manager A attached a register to Branch B across tenants"),
        "Expected cross-tenant branch mismatch rejection assertion for registers"
    );
    assert!(
        normalized_test_sql.contains("Admin A attached a user to Branch B across tenants"),
        "Expected cross-tenant branch mismatch rejection assertion for users"
    );
    assert!(
        normalized_test_sql.contains("Sole owner was able to delete their own membership"),
        "Expected sole-owner deletion prevention assertion"
    );
    assert!(
        normalized_test_sql
            .contains("Co-owner should be permitted to leave when another owner remains"),
        "Expected multi-owner leave assertion"
    );
}
