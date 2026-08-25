// Supabase RLS Migration & Policy Static Validation Tests.
// F1.08 — Supabase RLS policies

use crate::permission::PERMISSION_CATALOG;

const RLS_MIGRATION_SQL: &str =
    include_str!("../../../supabase/migrations/001_phase1_identity_and_rls.sql");
const RLS_TEST_SQL: &str = include_str!("../../../supabase/tests/rls_policies_test.sql");

#[test]
fn rls_migration_enables_row_level_security_on_all_tables() {
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
            RLS_MIGRATION_SQL.contains(&pattern),
            "Expected RLS to be enabled on table public.{table}"
        );
    }
}

#[test]
fn rls_migration_defines_security_definer_functions_with_safe_search_path() {
    let expected_functions = [
        "get_user_organization_ids",
        "is_org_member",
        "is_org_admin_or_owner",
        "is_org_manager_or_above",
    ];

    for func in expected_functions {
        assert!(
            RLS_MIGRATION_SQL.contains(&format!("FUNCTION public.{func}")),
            "Expected function public.{func} to be defined"
        );
    }

    assert!(
        RLS_MIGRATION_SQL.contains("SECURITY DEFINER"),
        "Expected helper functions to use SECURITY DEFINER"
    );
    assert!(
        RLS_MIGRATION_SQL.contains("SET search_path = public"),
        "Expected helper functions to enforce search_path = public against hijacking"
    );
}

#[test]
fn rls_migration_defines_explicit_policies_for_all_tenant_entities() {
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
            RLS_MIGRATION_SQL.contains(policy),
            "Expected policy '{policy}' to be defined in RLS migration"
        );
    }
}

#[test]
fn rls_permission_catalog_matches_canonical_rust_catalog() {
    for entry in PERMISSION_CATALOG {
        let code_literal = format!("'{}'", entry.code);
        assert!(
            RLS_MIGRATION_SQL.contains(&code_literal),
            "Expected permission code '{}' in Supabase RLS seed catalog",
            entry.code
        );
    }
}

#[test]
fn rls_test_suite_covers_cross_tenant_isolation_and_negative_cases() {
    assert!(
        RLS_TEST_SQL.contains("RLS SECURITY VIOLATION"),
        "Expected negative security assertions in RLS test suite"
    );
    assert!(
        RLS_TEST_SQL.contains("User A Owner must NOT see Organization B"),
        "Expected cross-organization negative assertion"
    );
    assert!(
        RLS_TEST_SQL.contains("Non-member saw"),
        "Expected unauthenticated/non-member negative assertion"
    );
}
