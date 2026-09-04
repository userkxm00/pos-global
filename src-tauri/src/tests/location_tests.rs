// Comprehensive test suite for F2.10 — Locations and Bins Master Data Architecture
// Covers ADR-0012: Migration 019, discrete entity models, same-branch hierarchy DAG,
// cycle prevention, traversal safety, code uniqueness, deactivation guards, and SettingsManage auth.

use crate::commands::location::{
    create_bin_impl, create_location_impl, deactivate_bin_impl, deactivate_location_impl,
    get_bin_impl, get_location_impl, get_location_tree_impl, list_bins_impl, list_locations_impl,
    reactivate_bin_impl, reactivate_location_impl, update_bin_impl, update_location_impl,
};
use crate::location::{
    create_bin, create_location, deactivate_bin, deactivate_location, get_bin, get_location,
    get_location_tree, list_bins, list_locations, reactivate_bin, reactivate_location, update_bin,
    update_location, BinFilter, CreateBinInput, CreateLocationInput, LocationError, LocationFilter,
    UpdateBinInput, UpdateLocationInput,
};
use crate::tests::test_helpers::{
    apply_migrations_up_to, create_test_org_and_branch, create_test_user_with_creds, setup_test_db,
};
use crate::user::session::create_local_session;
use rusqlite::Connection;

/// Helper to provision a second branch in the test organization.
fn create_second_branch(conn: &Connection, org_id: &str) -> String {
    let branch = crate::branch::create_branch(
        conn,
        crate::branch::CreateBranchInput {
            organization_id: org_id.to_string(),
            name: "Secondary Uptowm Branch".to_string(),
            address: Some("456 Uptown Ave, New York, NY".to_string()),
            currency: Some("USD".to_string()),
            is_active: Some(true),
        },
    )
    .expect("second branch created");
    branch.id
}

/// Helper to provision an authenticated session with a specific role.
fn create_auth_session(conn: &Connection, branch_id: &str, role: &str) -> String {
    let username = format!("user_{}_{}", role, &branch_id[..6]);
    let user = create_test_user_with_creds(
        conn,
        branch_id,
        &format!("Test {}", role),
        Some(&username),
        Some("Password123!"),
        Some("1234"),
        role,
    )
    .expect("user created");

    let session =
        create_local_session(conn, &user.id, branch_id, "password", None).expect("session created");
    session.id
}

// =========================================================================
// 1. MIGRATION 019 INTEGRITY TESTS
// =========================================================================

#[test]
fn test_migration_019_fresh_application_and_idempotency() {
    let conn = Connection::open_in_memory().expect("in-memory db");

    // Apply up to 019
    apply_migrations_up_to(&conn, "019_locations_bins");

    // Verify migration registered
    let count: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM _migrations WHERE name = '019_locations_bins'",
            [],
            |row| row.get(0),
        )
        .expect("query migration count");
    assert_eq!(count, 1);

    // Verify tables exist
    let loc_table_exists: bool = conn
        .query_row(
            "SELECT COUNT(*) > 0 FROM sqlite_master WHERE type = 'table' AND name = 'locations'",
            [],
            |row| row.get(0),
        )
        .expect("locations table check");
    assert!(loc_table_exists);

    let bin_table_exists: bool = conn
        .query_row(
            "SELECT COUNT(*) > 0 FROM sqlite_master WHERE type = 'table' AND name = 'bins'",
            [],
            |row| row.get(0),
        )
        .expect("bins table check");
    assert!(bin_table_exists);

    // Verify full init_database rerun is idempotent
    crate::db::init_database(&conn).expect("init_database must be idempotent");
}

// =========================================================================
// 2. LOCATION DOMAIN & VALIDATION TESTS
// =========================================================================

#[test]
fn test_create_location_success_and_sanitization() {
    let conn = setup_test_db();
    let (_org_id, branch_id) = create_test_org_and_branch(&conn);

    // Test whitespace trimming
    let loc = create_location(
        &conn,
        CreateLocationInput {
            branch_id: format!("  {branch_id}  "),
            parent_id: None,
            name: "  Warehouse Zone A  ".to_string(),
            code: "  ZONE-A  ".to_string(),
            location_type: Some("  Storage Bay  ".to_string()),
        },
    )
    .expect("location created");

    assert_eq!(loc.branch_id, branch_id);
    assert_eq!(loc.name, "Warehouse Zone A");
    assert_eq!(loc.code, "ZONE-A");
    assert_eq!(loc.location_type, Some("Storage Bay".to_string()));
    assert!(loc.is_active);
    assert!(loc.parent_id.is_none());
}

#[test]
fn test_location_empty_and_whitespace_rejection() {
    let conn = setup_test_db();
    let (_org_id, branch_id) = create_test_org_and_branch(&conn);

    // Empty name
    let err_name = create_location(
        &conn,
        CreateLocationInput {
            branch_id: branch_id.clone(),
            parent_id: None,
            name: "   ".to_string(),
            code: "CODE-1".to_string(),
            location_type: None,
        },
    )
    .unwrap_err();
    assert!(matches!(err_name, LocationError::Validation(_)));

    // Empty code
    let err_code = create_location(
        &conn,
        CreateLocationInput {
            branch_id: branch_id.clone(),
            parent_id: None,
            name: "Zone 1".to_string(),
            code: "\t\n ".to_string(),
            location_type: None,
        },
    )
    .unwrap_err();
    assert!(matches!(err_code, LocationError::Validation(_)));
}

#[test]
fn test_location_unicode_preservation() {
    let conn = setup_test_db();
    let (_org_id, branch_id) = create_test_org_and_branch(&conn);

    // Unicode in name and code (Arabic, French, special chars)
    let loc = create_location(
        &conn,
        CreateLocationInput {
            branch_id,
            parent_id: None,
            name: "مستودع رئيسي — Entrepôt Principal #1".to_string(),
            code: "موقع-أ / ZONE-#1".to_string(),
            location_type: Some("منطقة تخزين".to_string()),
        },
    )
    .expect("unicode location created");

    assert_eq!(loc.name, "مستودع رئيسي — Entrepôt Principal #1");
    assert_eq!(loc.code, "موقع-أ / ZONE-#1");
    assert_eq!(loc.location_type, Some("منطقة تخزين".to_string()));
}

#[test]
fn test_location_case_insensitive_code_uniqueness_scoped_to_branch() {
    let conn = setup_test_db();
    let (org_id, branch_a) = create_test_org_and_branch(&conn);
    let branch_b = create_second_branch(&conn, &org_id);

    // Create ZONE-A in Branch A
    create_location(
        &conn,
        CreateLocationInput {
            branch_id: branch_a.clone(),
            parent_id: None,
            name: "Zone A".to_string(),
            code: "zone-a".to_string(),
            location_type: None,
        },
    )
    .expect("zone a created");

    // Colliding code in same branch (case-insensitive) fails
    let err = create_location(
        &conn,
        CreateLocationInput {
            branch_id: branch_a.clone(),
            parent_id: None,
            name: "Duplicate Zone A".to_string(),
            code: "ZONE-A".to_string(),
            location_type: None,
        },
    )
    .unwrap_err();
    assert!(matches!(err, LocationError::DuplicateCode(_)));

    // Same code in different branch (Branch B) succeeds
    let loc_b = create_location(
        &conn,
        CreateLocationInput {
            branch_id: branch_b,
            parent_id: None,
            name: "Branch B Zone A".to_string(),
            code: "ZONE-A".to_string(),
            location_type: None,
        },
    )
    .expect("same code in branch B succeeds");
    assert_eq!(loc_b.code, "ZONE-A");
}

// =========================================================================
// 3. HIERARCHY & CYCLE INVARIANTS TESTS
// =========================================================================

#[test]
fn test_location_valid_hierarchy_and_tree() {
    let conn = setup_test_db();
    let (_org_id, branch_id) = create_test_org_and_branch(&conn);

    // 1. Root: Warehouse
    let root = create_location(
        &conn,
        CreateLocationInput {
            branch_id: branch_id.clone(),
            parent_id: None,
            name: "Main Warehouse".to_string(),
            code: "WH-MAIN".to_string(),
            location_type: Some("warehouse".to_string()),
        },
    )
    .expect("root created");

    // 2. Child: Aisle 1
    let aisle1 = create_location(
        &conn,
        CreateLocationInput {
            branch_id: branch_id.clone(),
            parent_id: Some(root.id.clone()),
            name: "Aisle 1".to_string(),
            code: "AISLE-01".to_string(),
            location_type: Some("aisle".to_string()),
        },
    )
    .expect("aisle 1 created");
    assert_eq!(aisle1.parent_id, Some(root.id.clone()));

    // 3. Grandchild: Shelf A
    let shelf_a = create_location(
        &conn,
        CreateLocationInput {
            branch_id: branch_id.clone(),
            parent_id: Some(aisle1.id.clone()),
            name: "Shelf A".to_string(),
            code: "SHELF-A".to_string(),
            location_type: Some("shelf".to_string()),
        },
    )
    .expect("shelf a created");
    assert_eq!(shelf_a.parent_id, Some(aisle1.id.clone()));

    // 4. Bin in Shelf A
    let bin = create_bin(
        &conn,
        CreateBinInput {
            location_id: shelf_a.id.clone(),
            name: "Slot 1".to_string(),
            code: "BIN-01".to_string(),
        },
    )
    .expect("bin created");

    // 5. Verify hierarchy tree
    let tree = get_location_tree(&conn, &branch_id, false).expect("tree generated");
    assert_eq!(tree.len(), 1);
    assert_eq!(tree[0].location.id, root.id);
    assert_eq!(tree[0].children.len(), 1);
    assert_eq!(tree[0].children[0].location.id, aisle1.id);
    assert_eq!(tree[0].children[0].children.len(), 1);
    assert_eq!(tree[0].children[0].children[0].location.id, shelf_a.id);
    assert_eq!(tree[0].children[0].children[0].bins.len(), 1);
    assert_eq!(tree[0].children[0].children[0].bins[0].id, bin.id);
}

#[test]
fn test_location_cross_branch_parent_rejection() {
    let conn = setup_test_db();
    let (org_id, branch_a) = create_test_org_and_branch(&conn);
    let branch_b = create_second_branch(&conn, &org_id);

    // Parent in Branch A
    let parent_a = create_location(
        &conn,
        CreateLocationInput {
            branch_id: branch_a,
            parent_id: None,
            name: "Branch A Warehouse".to_string(),
            code: "WH-A".to_string(),
            location_type: None,
        },
    )
    .expect("parent a created");

    // Child in Branch B attempting to set parent in Branch A fails
    let err = create_location(
        &conn,
        CreateLocationInput {
            branch_id: branch_b,
            parent_id: Some(parent_a.id),
            name: "Branch B Zone".to_string(),
            code: "ZONE-B".to_string(),
            location_type: None,
        },
    )
    .unwrap_err();

    assert!(matches!(err, LocationError::CrossBranchParent(_)));
}

#[test]
fn test_location_self_parenting_and_cycle_rejection() {
    let conn = setup_test_db();
    let (_org_id, branch_id) = create_test_org_and_branch(&conn);

    // Create nodes A, B, C
    let loc_a = create_location(
        &conn,
        CreateLocationInput {
            branch_id: branch_id.clone(),
            parent_id: None,
            name: "Node A".to_string(),
            code: "A".to_string(),
            location_type: None,
        },
    )
    .expect("node a created");

    let loc_b = create_location(
        &conn,
        CreateLocationInput {
            branch_id: branch_id.clone(),
            parent_id: Some(loc_a.id.clone()),
            name: "Node B".to_string(),
            code: "B".to_string(),
            location_type: None,
        },
    )
    .expect("node b created");

    let loc_c = create_location(
        &conn,
        CreateLocationInput {
            branch_id: branch_id.clone(),
            parent_id: Some(loc_b.id.clone()),
            name: "Node C".to_string(),
            code: "C".to_string(),
            location_type: None,
        },
    )
    .expect("node c created");

    // 1. Direct self-parenting: update A to parent under A
    let err_self = update_location(
        &conn,
        UpdateLocationInput {
            id: loc_a.id.clone(),
            name: None,
            code: None,
            parent_id: Some(Some(loc_a.id.clone())),
            location_type: None,
            is_active: None,
        },
    )
    .unwrap_err();
    assert!(matches!(err_self, LocationError::SelfParenting(_)));

    // 2. Transitive cycle: update A to parent under its grandchild C (A -> B -> C -> A)
    let err_cycle = update_location(
        &conn,
        UpdateLocationInput {
            id: loc_a.id.clone(),
            name: None,
            code: None,
            parent_id: Some(Some(loc_c.id.clone())),
            location_type: None,
            is_active: None,
        },
    )
    .unwrap_err();
    assert!(matches!(err_cycle, LocationError::CycleDetected(_)));

    // 3. Valid parent reassignment: reassign C to parent under A directly
    let updated_c = update_location(
        &conn,
        UpdateLocationInput {
            id: loc_c.id.clone(),
            name: None,
            code: None,
            parent_id: Some(Some(loc_a.id.clone())),
            location_type: None,
            is_active: None,
        },
    )
    .expect("reassignment to A succeeds");
    assert_eq!(updated_c.parent_id, Some(loc_a.id.clone()));

    // 4. Valid unsetting: convert C to root
    let root_c = update_location(
        &conn,
        UpdateLocationInput {
            id: loc_c.id.clone(),
            name: None,
            code: None,
            parent_id: Some(None),
            location_type: None,
            is_active: None,
        },
    )
    .expect("convert C to root succeeds");
    assert_eq!(root_c.parent_id, None);
}

// =========================================================================
// 4. DEACTIVATION GUARDS & LIFECYCLE TESTS
// =========================================================================

#[test]
fn test_location_deactivation_blocked_by_active_children_or_bins() {
    let conn = setup_test_db();
    let (_org_id, branch_id) = create_test_org_and_branch(&conn);

    let parent = create_location(
        &conn,
        CreateLocationInput {
            branch_id: branch_id.clone(),
            parent_id: None,
            name: "Parent Zone".to_string(),
            code: "P-ZONE".to_string(),
            location_type: None,
        },
    )
    .expect("parent created");

    let child = create_location(
        &conn,
        CreateLocationInput {
            branch_id: branch_id.clone(),
            parent_id: Some(parent.id.clone()),
            name: "Child Aisle".to_string(),
            code: "C-AISLE".to_string(),
            location_type: None,
        },
    )
    .expect("child created");

    // Attempting to deactivate parent fails while child is active
    let err_child = deactivate_location(&conn, &parent.id).unwrap_err();
    assert!(matches!(err_child, LocationError::DeactivationBlocked(_)));

    // Deactivate child first
    deactivate_location(&conn, &child.id).expect("child deactivated");

    // Add bin to parent
    let bin = create_bin(
        &conn,
        CreateBinInput {
            location_id: parent.id.clone(),
            name: "Parent Bin".to_string(),
            code: "PB-01".to_string(),
        },
    )
    .expect("bin created");

    // Attempting to deactivate parent fails while bin is active
    let err_bin = deactivate_location(&conn, &parent.id).unwrap_err();
    assert!(matches!(err_bin, LocationError::DeactivationBlocked(_)));

    // Deactivate bin
    deactivate_bin(&conn, &bin.id).expect("bin deactivated");

    // Now deactivating parent succeeds
    let deactivated_parent =
        deactivate_location(&conn, &parent.id).expect("parent deactivation succeeds");
    assert!(!deactivated_parent.is_active);

    // Reactivation succeeds
    let reactivated = reactivate_location(&conn, &parent.id).expect("parent reactivated");
    assert!(reactivated.is_active);
}

#[test]
fn test_location_updated_at_timestamp_changes_on_update() {
    let conn = setup_test_db();
    let (_org_id, branch_id) = create_test_org_and_branch(&conn);

    let loc = create_location(
        &conn,
        CreateLocationInput {
            branch_id,
            parent_id: None,
            name: "Initial Name".to_string(),
            code: "LOC-INIT".to_string(),
            location_type: None,
        },
    )
    .expect("loc created");

    // Manually backdate updated_at in database to test that UPDATE modifies it
    conn.execute(
        "UPDATE locations SET updated_at = '2020-01-01 00:00:00' WHERE id = ?1",
        [&loc.id],
    )
    .expect("backdate updated_at");

    let updated = update_location(
        &conn,
        UpdateLocationInput {
            id: loc.id.clone(),
            name: Some("Renamed Location".to_string()),
            code: None,
            parent_id: None,
            location_type: None,
            is_active: None,
        },
    )
    .expect("update succeeds");

    assert_eq!(updated.name, "Renamed Location");
    assert_ne!(updated.updated_at, "2020-01-01 00:00:00");
}

// =========================================================================
// 5. BIN DOMAIN & VALIDATION TESTS
// =========================================================================

#[test]
fn test_bin_crud_and_code_uniqueness_per_location() {
    let conn = setup_test_db();
    let (_org_id, branch_id) = create_test_org_and_branch(&conn);

    let loc_1 = create_location(
        &conn,
        CreateLocationInput {
            branch_id: branch_id.clone(),
            parent_id: None,
            name: "Location 1".to_string(),
            code: "LOC-1".to_string(),
            location_type: None,
        },
    )
    .expect("loc 1 created");

    let loc_2 = create_location(
        &conn,
        CreateLocationInput {
            branch_id,
            parent_id: None,
            name: "Location 2".to_string(),
            code: "LOC-2".to_string(),
            location_type: None,
        },
    )
    .expect("loc 2 created");

    // Create Bin A-1 in Location 1
    let bin1 = create_bin(
        &conn,
        CreateBinInput {
            location_id: loc_1.id.clone(),
            name: "Bin A-1".to_string(),
            code: "a-01".to_string(),
        },
    )
    .expect("bin 1 created");
    assert_eq!(bin1.code, "a-01");
    assert!(bin1.is_active);

    // Duplicate bin code in Location 1 fails (case-insensitive)
    let err_dup = create_bin(
        &conn,
        CreateBinInput {
            location_id: loc_1.id.clone(),
            name: "Duplicate Bin".to_string(),
            code: "A-01".to_string(),
        },
    )
    .unwrap_err();
    assert!(matches!(err_dup, LocationError::DuplicateCode(_)));

    // Same bin code in Location 2 succeeds
    let bin2 = create_bin(
        &conn,
        CreateBinInput {
            location_id: loc_2.id.clone(),
            name: "Location 2 Bin A-1".to_string(),
            code: "A-01".to_string(),
        },
    )
    .expect("bin with same code in loc 2 succeeds");
    assert_eq!(bin2.code, "A-01");

    // Update bin
    let updated_bin = update_bin(
        &conn,
        UpdateBinInput {
            id: bin1.id.clone(),
            name: Some("Updated Slot A-1".to_string()),
            code: Some("A-01-NEW".to_string()),
            is_active: None,
        },
    )
    .expect("update bin succeeds");
    assert_eq!(updated_bin.name, "Updated Slot A-1");
    assert_eq!(updated_bin.code, "A-01-NEW");

    // Deactivate and reactivate bin
    let deactivated = deactivate_bin(&conn, &bin1.id).expect("bin deactivated");
    assert!(!deactivated.is_active);

    let reactivated = reactivate_bin(&conn, &bin1.id).expect("bin reactivated");
    assert!(reactivated.is_active);
}

// =========================================================================
// 6. IPC COMMAND SECURITY & AUTHORIZATION TESTS
// =========================================================================

#[test]
fn test_ipc_commands_settings_manage_authorization() {
    let conn = setup_test_db();
    let (org_id, branch_a) = create_test_org_and_branch(&conn);
    let branch_b = create_second_branch(&conn, &org_id);

    let admin_session = create_auth_session(&conn, &branch_a, "admin");
    let manager_session = create_auth_session(&conn, &branch_a, "manager");
    let cashier_session = create_auth_session(&conn, &branch_a, "cashier");
    let other_branch_admin = create_auth_session(&conn, &branch_b, "admin");

    // 1. Cashier lacks Permission::SettingsManage -> rejected
    let cashier_err = create_location_impl(
        &conn,
        &cashier_session,
        CreateLocationInput {
            branch_id: branch_a.clone(),
            parent_id: None,
            name: "Unauthorized Zone".to_string(),
            code: "NO-AUTH".to_string(),
            location_type: None,
        },
    )
    .unwrap_err();
    assert!(cashier_err.contains("Permission denied"));

    // 2. Manager has Permission::SettingsManage -> succeeds
    let loc_mgr = create_location_impl(
        &conn,
        &manager_session,
        CreateLocationInput {
            branch_id: branch_a.clone(),
            parent_id: None,
            name: "Manager Zone".to_string(),
            code: "MGR-ZONE".to_string(),
            location_type: None,
        },
    )
    .expect("manager can create location");
    assert_eq!(loc_mgr.name, "Manager Zone");

    // 3. Admin has Permission::SettingsManage -> succeeds
    let loc_admin = create_location_impl(
        &conn,
        &admin_session,
        CreateLocationInput {
            branch_id: branch_a.clone(),
            parent_id: Some(loc_mgr.id.clone()),
            name: "Admin Subzone".to_string(),
            code: "ADM-SUB".to_string(),
            location_type: None,
        },
    )
    .expect("admin can create location");
    assert_eq!(loc_admin.parent_id, Some(loc_mgr.id.clone()));

    // 4. Cross-branch admin cannot mutate branch A location
    let cross_err = update_location_impl(
        &conn,
        &other_branch_admin,
        UpdateLocationInput {
            id: loc_mgr.id.clone(),
            name: Some("Hacked Name".to_string()),
            code: None,
            parent_id: None,
            location_type: None,
            is_active: None,
        },
    )
    .unwrap_err();
    assert!(cross_err.contains("Scope mismatch"));

    // 5. Anti-existence leakage: accessing cross-branch location returns Ok(None) (indistinguishable from not found)
    let leak_res = get_location_impl(&conn, &other_branch_admin, &loc_mgr.id).unwrap();
    assert!(leak_res.is_none());

    // 6. Mutation permission-first check: unauthorized user without SettingsManage is rejected before DB lookup
    let cashier_probe_err = update_location_impl(
        &conn,
        &cashier_a,
        UpdateLocationInput {
            id: "non-existent-probe-id".to_string(),
            name: Some("Probe".to_string()),
            code: None,
            parent_id: None,
            location_type: None,
            is_active: None,
        },
    )
    .unwrap_err();
    assert!(cashier_probe_err.contains("Permission denied"));
}

#[test]
fn test_ipc_bin_authorization_and_anti_existence_leakage() {
    let conn = setup_test_db();
    let (org_id, branch_a) = create_test_org_and_branch(&conn);
    let branch_b = create_second_branch(&conn, &org_id);

    let manager_a = create_auth_session(&conn, &branch_a, "manager");
    let cashier_a = create_auth_session(&conn, &branch_a, "cashier");
    let admin_b = create_auth_session(&conn, &branch_b, "admin");

    let loc_a = create_location(
        &conn,
        CreateLocationInput {
            branch_id: branch_a.clone(),
            parent_id: None,
            name: "Storage Zone".to_string(),
            code: "SZ-01".to_string(),
            location_type: None,
        },
    )
    .expect("loc created");

    // Cashier denied bin creation
    let err_cashier = create_bin_impl(
        &conn,
        &cashier_a,
        CreateBinInput {
            location_id: loc_a.id.clone(),
            name: "Bin 1".to_string(),
            code: "B-1".to_string(),
        },
    )
    .unwrap_err();
    assert!(err_cashier.contains("Permission denied"));

    // Manager in branch A creates bin
    let bin_a = create_bin_impl(
        &conn,
        &manager_a,
        CreateBinInput {
            location_id: loc_a.id.clone(),
            name: "Bin 1".to_string(),
            code: "B-1".to_string(),
        },
    )
    .expect("bin created by manager");

    // Admin B attempting to read bin in Branch A gets Ok(None) (airtight anti-leakage)
    let leak_bin_res = get_bin_impl(&conn, &admin_b, &bin_a.id).unwrap();
    assert!(leak_bin_res.is_none());

    // Empty filter on list_bins_impl fails closed
    let empty_filter_err = list_bins_impl(&conn, &manager_a, BinFilter::default()).unwrap_err();
    assert!(empty_filter_err.contains("A branch_id or location_id filter is required"));

    // Correctly scoped filter returns bins
    let scoped_bins = list_bins_impl(
        &conn,
        &manager_a,
        BinFilter {
            location_id: Some(loc_a.id.clone()),
            branch_id: None,
            is_active: None,
            query: None,
        },
    )
    .expect("scoped bin list succeeds");
    assert_eq!(scoped_bins.len(), 1);
    assert_eq!(scoped_bins[0].id, bin_a.id);
}

#[test]
fn test_update_location_reactivation_fails_when_parent_is_inactive() {
    let conn = setup_test_db();
    let (_org_id, branch_id) = create_test_org_and_branch(&conn);

    let parent = create_location(
        &conn,
        CreateLocationInput {
            branch_id: branch_id.clone(),
            parent_id: None,
            name: "Parent Zone".to_string(),
            code: "P-ZONE-ACT".to_string(),
            location_type: None,
        },
    )
    .expect("parent created");

    let child = create_location(
        &conn,
        CreateLocationInput {
            branch_id: branch_id.clone(),
            parent_id: Some(parent.id.clone()),
            name: "Child Aisle".to_string(),
            code: "C-AISLE-ACT".to_string(),
            location_type: None,
        },
    )
    .expect("child created");

    // Deactivate child first, then deactivate parent
    deactivate_location(&conn, &child.id).expect("child deactivated");
    deactivate_location(&conn, &parent.id).expect("parent deactivated");

    // Attempting to reactivate child via generic update_location while parent is inactive must fail with InactiveParent
    let err = update_location(
        &conn,
        UpdateLocationInput {
            id: child.id.clone(),
            name: None,
            code: None,
            parent_id: None,
            location_type: None,
            is_active: Some(true),
        },
    )
    .unwrap_err();

    assert!(matches!(err, LocationError::InactiveParent(_)));

    // Reactivating parent first allows child reactivation
    reactivate_location(&conn, &parent.id).expect("parent reactivated");
    let reactivated_child = update_location(
        &conn,
        UpdateLocationInput {
            id: child.id.clone(),
            name: None,
            code: None,
            parent_id: None,
            location_type: None,
            is_active: Some(true),
        },
    )
    .expect("child reactivated via update_location");
    assert!(reactivated_child.is_active);
}

#[test]
fn test_update_location_input_tri_state_serde_deserialization() {
    // 1. Omitted fields -> None (preserve existing)
    let json_omitted = r#"{"id": "loc_1"}"#;
    let input_omitted: UpdateLocationInput = serde_json::from_str(json_omitted).unwrap();
    assert_eq!(input_omitted.parent_id, None);
    assert_eq!(input_omitted.location_type, None);

    // 2. Explicit null -> Some(None) (clear field to NULL)
    let json_null = r#"{"id": "loc_1", "parent_id": null, "location_type": null}"#;
    let input_null: UpdateLocationInput = serde_json::from_str(json_null).unwrap();
    assert_eq!(input_null.parent_id, Some(None));
    assert_eq!(input_null.location_type, Some(None));

    // 3. Explicit value -> Some(Some(val)) (set new value)
    let json_val = r#"{"id": "loc_1", "parent_id": "loc_parent", "location_type": "zone"}"#;
    let input_val: UpdateLocationInput = serde_json::from_str(json_val).unwrap();
    assert_eq!(input_val.parent_id, Some(Some("loc_parent".to_string())));
    assert_eq!(input_val.location_type, Some(Some("zone".to_string())));
}
