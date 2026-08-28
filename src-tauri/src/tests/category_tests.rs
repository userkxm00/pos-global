use crate::auth::middleware::{require_permission, AuthMiddlewareError, AuthorizeRequest};
use crate::branch::{create_branch, CreateBranchInput};
use crate::category::{
    create_category, delete_category, get_category, get_category_tree, list_categories,
    update_category, validate_description, validate_name, CategoryError, CategoryFilter,
    CreateCategoryInput, UpdateCategoryInput,
};
use crate::organization::{create_organization, CreateOrganizationInput};
use crate::permission::Permission;
use crate::product::{
    create_product, get_catalog_organization_id, get_product, CreateProductInput,
};
use crate::tests::test_helpers::{
    create_test_org_and_branch, create_test_user_with_creds, setup_test_db,
};
use crate::user::session::create_local_session;

fn make_category_fixture(name: &str, parent_id: Option<&str>) -> CreateCategoryInput {
    CreateCategoryInput {
        name: name.to_string(),
        parent_id: parent_id.map(ToString::to_string),
        description: None,
    }
}

// =========================================================================
// 1. VALIDATION TESTS
// =========================================================================

#[test]
fn test_validate_category_name_trims_and_accepts_valid() {
    let result = validate_name("  Hot Beverages  ").expect("valid name");
    assert_eq!(result, "Hot Beverages");
}

#[test]
fn test_validate_category_name_accepts_multibyte_unicode_up_to_255_chars() {
    let arabic_name = "مشروبات ساخنة وقهوة مختصة";
    assert!(arabic_name.len() > arabic_name.chars().count());
    let result = validate_name(arabic_name).expect("multibyte unicode name accepted");
    assert_eq!(result, arabic_name);

    let exact_255_unicode: String = "م".repeat(255);
    assert_eq!(exact_255_unicode.chars().count(), 255);
    let result_255 = validate_name(&exact_255_unicode).expect("exact 255 unicode accepted");
    assert_eq!(result_255, exact_255_unicode);
}

#[test]
fn test_validate_category_name_rejects_empty_and_whitespace_only() {
    assert!(matches!(
        validate_name(""),
        Err(CategoryError::Validation(_))
    ));
    assert!(matches!(
        validate_name("   \t\n  "),
        Err(CategoryError::Validation(_))
    ));
}

#[test]
fn test_validate_category_name_rejects_over_255_unicode_chars() {
    let too_long_ascii = "a".repeat(256);
    assert!(matches!(
        validate_name(&too_long_ascii),
        Err(CategoryError::Validation(_))
    ));

    let too_long_unicode = "م".repeat(256);
    assert!(matches!(
        validate_name(&too_long_unicode),
        Err(CategoryError::Validation(_))
    ));
}

#[test]
fn test_validate_category_description() {
    assert_eq!(validate_description(None), None);
    assert_eq!(validate_description(Some("   ")), None);
    assert_eq!(
        validate_description(Some("  Premium artisanal selections  ")),
        Some("Premium artisanal selections".to_string())
    );
}

// =========================================================================
// 2. REPOSITORY & HIERARCHY TESTS
// =========================================================================

#[test]
fn test_create_and_get_root_category() {
    let conn = setup_test_db();
    let input = CreateCategoryInput {
        name: "Beverages".to_string(),
        parent_id: None,
        description: Some("All drinks".to_string()),
    };

    let created = create_category(&conn, input).expect("category created");
    assert_eq!(created.name, "Beverages");
    assert_eq!(created.parent_id, None);
    assert_eq!(created.description.as_deref(), Some("All drinks"));
    assert!(created.is_active);
    assert!(!created.id.is_empty());

    let fetched = get_category(&conn, &created.id)
        .expect("get succeeds")
        .expect("found");
    assert_eq!(created, fetched);
}

#[test]
fn test_create_and_get_child_category() {
    let conn = setup_test_db();
    let root = create_category(&conn, make_category_fixture("Beverages", None)).expect("root");
    let child_input = CreateCategoryInput {
        name: "Hot Drinks".to_string(),
        parent_id: Some(root.id.clone()),
        description: None,
    };
    let child = create_category(&conn, child_input).expect("child created");
    assert_eq!(child.parent_id.as_deref(), Some(root.id.as_str()));

    let grandchild_input = CreateCategoryInput {
        name: "Espresso".to_string(),
        parent_id: Some(child.id.clone()),
        description: None,
    };
    let grandchild = create_category(&conn, grandchild_input).expect("grandchild created");
    assert_eq!(grandchild.parent_id.as_deref(), Some(child.id.as_str()));
}

#[test]
fn test_create_category_with_missing_parent_rejected() {
    let conn = setup_test_db();
    let input = make_category_fixture("Child Without Parent", Some("nonexistent-parent-id"));
    let err = create_category(&conn, input).unwrap_err();
    assert!(matches!(err, CategoryError::NotFound(_)));
}

#[test]
fn test_create_category_with_inactive_parent_rejected() {
    let conn = setup_test_db();
    let parent = create_category(&conn, make_category_fixture("Discontinued Parent", None))
        .expect("parent created");
    delete_category(&conn, &parent.id).expect("parent soft deleted");

    let input = make_category_fixture("Child Under Inactive", Some(&parent.id));
    let err = create_category(&conn, input).unwrap_err();
    assert!(matches!(err, CategoryError::InactiveParent(_)));
}

#[test]
fn test_update_category_self_parenting_rejected() {
    let conn = setup_test_db();
    let cat = create_category(&conn, make_category_fixture("Snacks", None)).expect("cat");
    let update = UpdateCategoryInput {
        id: cat.id.clone(),
        name: "Snacks".to_string(),
        parent_id: Some(cat.id.clone()),
        description: None,
        is_active: true,
    };
    let err = update_category(&conn, update).unwrap_err();
    assert!(matches!(err, CategoryError::SelfParenting(_)));
}

#[test]
fn test_update_category_two_node_cycle_rejected() {
    let conn = setup_test_db();
    let parent_a = create_category(&conn, make_category_fixture("Category A", None)).expect("A");
    let child_b = create_category(
        &conn,
        make_category_fixture("Category B", Some(&parent_a.id)),
    )
    .expect("B");

    // Attempt to make A a child of B (A -> B -> A)
    let update_a = UpdateCategoryInput {
        id: parent_a.id.clone(),
        name: "Category A".to_string(),
        parent_id: Some(child_b.id.clone()),
        description: None,
        is_active: true,
    };
    let err = update_category(&conn, update_a).unwrap_err();
    assert!(matches!(err, CategoryError::CycleDetected(_)));
}

#[test]
fn test_update_category_multi_node_cycle_rejected() {
    let conn = setup_test_db();
    let a = create_category(&conn, make_category_fixture("A", None)).expect("A");
    let b = create_category(&conn, make_category_fixture("B", Some(&a.id))).expect("B");
    let c = create_category(&conn, make_category_fixture("C", Some(&b.id))).expect("C");
    let d = create_category(&conn, make_category_fixture("D", Some(&c.id))).expect("D");

    // Attempt to reparent A under D (A -> B -> C -> D -> A)
    let update_a = UpdateCategoryInput {
        id: a.id.clone(),
        name: "A".to_string(),
        parent_id: Some(d.id.clone()),
        description: None,
        is_active: true,
    };
    let err = update_category(&conn, update_a).unwrap_err();
    assert!(matches!(err, CategoryError::CycleDetected(_)));
}

#[test]
fn test_update_category_safe_reparenting_and_root_reparenting() {
    let conn = setup_test_db();
    let dept1 = create_category(&conn, make_category_fixture("Food", None)).expect("dept1");
    let dept2 = create_category(&conn, make_category_fixture("Drinks", None)).expect("dept2");
    let item = create_category(&conn, make_category_fixture("Snack Bar", Some(&dept1.id)))
        .expect("item under food");

    // Move to Drinks
    let updated = update_category(
        &conn,
        UpdateCategoryInput {
            id: item.id.clone(),
            name: "Snack Bar".to_string(),
            parent_id: Some(dept2.id.clone()),
            description: None,
            is_active: true,
        },
    )
    .expect("reparent to drinks");
    assert_eq!(updated.parent_id.as_deref(), Some(dept2.id.as_str()));

    // Move to Root
    let root_item = update_category(
        &conn,
        UpdateCategoryInput {
            id: item.id.clone(),
            name: "Snack Bar".to_string(),
            parent_id: None,
            description: None,
            is_active: true,
        },
    )
    .expect("reparent to root");
    assert_eq!(root_item.parent_id, None);
}

// =========================================================================
// 3. UNIQUENESS & COLLATION TESTS
// =========================================================================

#[test]
fn test_duplicate_active_root_category_rejected() {
    let conn = setup_test_db();
    create_category(&conn, make_category_fixture("Electronics", None)).expect("first");

    let err = create_category(&conn, make_category_fixture("electronics", None)).unwrap_err();
    assert!(matches!(err, CategoryError::DuplicateName(_)));
}

#[test]
fn test_duplicate_active_sibling_category_rejected() {
    let conn = setup_test_db();
    let root = create_category(&conn, make_category_fixture("Clothing", None)).expect("root");
    create_category(&conn, make_category_fixture("Shirts", Some(&root.id))).expect("first child");

    let err = create_category(&conn, make_category_fixture("shirts", Some(&root.id))).unwrap_err();
    assert!(matches!(err, CategoryError::DuplicateName(_)));
}

#[test]
fn test_duplicate_category_name_under_different_parents_allowed() {
    let conn = setup_test_db();
    let root_a = create_category(&conn, make_category_fixture("Branch A", None)).expect("A");
    let root_b = create_category(&conn, make_category_fixture("Branch B", None)).expect("B");

    let child_a = create_category(
        &conn,
        make_category_fixture("Accessories", Some(&root_a.id)),
    )
    .expect("child under A");
    let child_b = create_category(
        &conn,
        make_category_fixture("Accessories", Some(&root_b.id)),
    )
    .expect("child under B");

    assert_ne!(child_a.id, child_b.id);
    assert_eq!(child_a.name, child_b.name);
}

#[test]
fn test_reuse_category_name_after_archive_allowed() {
    let conn = setup_test_db();
    let cat =
        create_category(&conn, make_category_fixture("Seasonal Summer", None)).expect("first");
    delete_category(&conn, &cat.id).expect("archived");

    let new_cat =
        create_category(&conn, make_category_fixture("Seasonal Summer", None)).expect("reused");
    assert_ne!(new_cat.id, cat.id);
}

// =========================================================================
// 4. SOFT DELETE & ARCHIVE GUARD TESTS
// =========================================================================

#[test]
fn test_delete_category_with_active_children_rejected() {
    let conn = setup_test_db();
    let parent = create_category(&conn, make_category_fixture("Parent Cat", None)).expect("parent");
    let _child = create_category(&conn, make_category_fixture("Child Cat", Some(&parent.id)))
        .expect("child");

    let err = delete_category(&conn, &parent.id).unwrap_err();
    assert!(matches!(err, CategoryError::HasActiveChildren(_)));
}

#[test]
fn test_delete_category_after_children_archived_succeeds() {
    let conn = setup_test_db();
    let parent = create_category(&conn, make_category_fixture("Parent Cat", None)).expect("parent");
    let child = create_category(&conn, make_category_fixture("Child Cat", Some(&parent.id)))
        .expect("child");

    delete_category(&conn, &child.id).expect("child archived");
    delete_category(&conn, &parent.id).expect("parent archived successfully");

    let fetched_parent = get_category(&conn, &parent.id)
        .expect("query")
        .expect("found");
    assert!(!fetched_parent.is_active);
}

#[test]
fn test_reactivate_category_with_duplicate_conflict_rejected() {
    let conn = setup_test_db();
    let cat1 = create_category(&conn, make_category_fixture("Hardware", None)).expect("cat1");
    delete_category(&conn, &cat1.id).expect("cat1 archived");

    let cat2 =
        create_category(&conn, make_category_fixture("Hardware", None)).expect("cat2 active");

    // Attempt to reactivate cat1
    let err = update_category(
        &conn,
        UpdateCategoryInput {
            id: cat1.id.clone(),
            name: "Hardware".to_string(),
            parent_id: None,
            description: None,
            is_active: true,
        },
    )
    .unwrap_err();
    assert!(matches!(err, CategoryError::DuplicateName(_)));
}

// =========================================================================
// 5. TREE RECONSTRUCTION & LIST FILTER TESTS
// =========================================================================

#[test]
fn test_get_category_tree_hierarchy() {
    let conn = setup_test_db();
    let food = create_category(&conn, make_category_fixture("Food", None)).expect("food");
    let drinks = create_category(&conn, make_category_fixture("Drinks", None)).expect("drinks");

    let hot = create_category(&conn, make_category_fixture("Hot", Some(&drinks.id))).expect("hot");
    let cold =
        create_category(&conn, make_category_fixture("Cold", Some(&drinks.id))).expect("cold");
    let _tea =
        create_category(&conn, make_category_fixture("Green Tea", Some(&hot.id))).expect("tea");

    let tree = get_category_tree(&conn, false).expect("tree built");
    assert_eq!(tree.len(), 2); // 2 root categories: Drinks, Food (alphabetical)

    let drinks_node = tree
        .iter()
        .find(|n| n.category.id == drinks.id)
        .expect("drinks node");
    assert_eq!(drinks_node.children.len(), 2);

    let hot_node = drinks_node
        .children
        .iter()
        .find(|n| n.category.id == hot.id)
        .expect("hot node");
    assert_eq!(hot_node.children.len(), 1);
    assert_eq!(hot_node.children[0].category.name, "Green Tea");
}

#[test]
fn test_list_categories_filters() {
    let conn = setup_test_db();
    let root1 = create_category(&conn, make_category_fixture("Bakery", None)).expect("root1");
    let root2 = create_category(&conn, make_category_fixture("Dairy", None)).expect("root2");
    let child1 =
        create_category(&conn, make_category_fixture("Pastry", Some(&root1.id))).expect("c1");
    delete_category(&conn, &child1.id).expect("c1 archived");

    // Roots only filter
    let roots = list_categories(
        &conn,
        &CategoryFilter {
            query: None,
            parent_id: Some("root".to_string()),
            is_active: Some(true),
        },
    )
    .expect("roots");
    assert_eq!(roots.len(), 2);

    // Children of root1 including inactive
    let r1_children = list_categories(
        &conn,
        &CategoryFilter {
            query: None,
            parent_id: Some(root1.id.clone()),
            is_active: None,
        },
    )
    .expect("r1 children");
    assert_eq!(r1_children.len(), 1);
    assert_eq!(r1_children[0].name, "Pastry");
}

// =========================================================================
// 6. INTEGRATION WITH PRODUCTS & TAX RULES
// =========================================================================

#[test]
fn test_product_category_reference_preserved_on_archive() {
    let conn = setup_test_db();
    let cat = create_category(&conn, make_category_fixture("Specialty Beans", None)).expect("cat");

    let product_input = CreateProductInput {
        name: "Yirgacheffe 250g".to_string(),
        description: None,
        category_id: Some(cat.id.clone()),
        barcode: Some("BEANS-001".to_string()),
        product_type: None,
        base_price_minor: 1500,
        cost_price_minor: None,
        unit_type: None,
        requires_expiry: None,
        requires_serial: None,
        warranty_months: None,
        custom_attributes: None,
    };
    let product = create_product(&conn, product_input).expect("product created");
    assert_eq!(product.category_id.as_deref(), Some(cat.id.as_str()));

    // Soft delete the category
    delete_category(&conn, &cat.id).expect("category archived");

    // Product still exists and references the category id
    let fetched = get_product(&conn, &product.id)
        .expect("query")
        .expect("found");
    assert_eq!(fetched.category_id.as_deref(), Some(cat.id.as_str()));
}

// =========================================================================
// 7. AUTHORIZATION & TENANCY TESTS
// =========================================================================

#[test]
fn test_category_authorization_admin_and_manager_permitted_cashier_denied() {
    let conn = setup_test_db();
    let (_org_id, branch_id) = create_test_org_and_branch(&conn);

    let admin =
        create_test_user_with_creds(&conn, &branch_id, "Admin User", None, None, None, "admin")
            .expect("admin");
    let manager = create_test_user_with_creds(
        &conn,
        &branch_id,
        "Manager User",
        None,
        None,
        None,
        "manager",
    )
    .expect("manager");
    let cashier = create_test_user_with_creds(
        &conn,
        &branch_id,
        "Cashier User",
        None,
        None,
        None,
        "cashier",
    )
    .expect("cashier");

    let admin_sess = create_local_session(&conn, &admin.id, &branch_id, None, 8).expect("sess");
    let manager_sess = create_local_session(&conn, &manager.id, &branch_id, None, 8).expect("sess");
    let cashier_sess = create_local_session(&conn, &cashier.id, &branch_id, None, 8).expect("sess");

    // Admin & Manager have ProductsManage
    assert!(require_permission(&conn, &admin_sess.id, Permission::ProductsManage).is_ok());
    assert!(require_permission(&conn, &manager_sess.id, Permission::ProductsManage).is_ok());

    // Cashier denied ProductsManage
    let err = require_permission(&conn, &cashier_sess.id, Permission::ProductsManage).unwrap_err();
    assert!(matches!(err, AuthMiddlewareError::PermissionDenied(_)));

    // Cashier CAN read with active session
    let auth_read = AuthorizeRequest::new(&cashier_sess.id).execute(&conn);
    assert!(auth_read.is_ok());
}

#[test]
fn test_category_unconfigured_org_fails_closed() {
    let conn = setup_test_db();
    // Database with 2 branches in different organizations without business_settings configured
    let org_a = create_organization(
        &conn,
        CreateOrganizationInput {
            name: "Org A".into(),
            default_currency: None,
            default_language: None,
        },
    )
    .expect("A");
    let org_b = create_organization(
        &conn,
        CreateOrganizationInput {
            name: "Org B".into(),
            default_currency: None,
            default_language: None,
        },
    )
    .expect("B");
    create_branch(
        &conn,
        CreateBranchInput {
            organization_id: org_a.id,
            name: "Branch A".into(),
            address: None,
            currency: None,
            is_active: Some(true),
        },
    )
    .expect("BA");
    create_branch(
        &conn,
        CreateBranchInput {
            organization_id: org_b.id,
            name: "Branch B".into(),
            address: None,
            currency: None,
            is_active: Some(true),
        },
    )
    .expect("BB");

    let resolved = get_catalog_organization_id(&conn).expect("resolution succeeds");
    assert!(
        resolved.is_none(),
        "Unconfigured multi-tenant catalog must resolve to None"
    );
}

// =========================================================================
// 8. MIGRATION 011 LIFECYCLE & UPGRADE TESTS
// =========================================================================

#[test]
fn test_migration_011_upgrade_from_010_with_existing_data() {
    let conn = rusqlite::Connection::open_in_memory().expect("in-memory db");
    conn.execute_batch("PRAGMA foreign_keys = ON;")
        .expect("fk on");
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS _migrations (
            name TEXT PRIMARY KEY,
            applied_at TEXT NOT NULL DEFAULT (datetime('now'))
        );",
    )
    .expect("migrations table");

    let mig_011_idx = crate::db::MIGRATIONS
        .iter()
        .position(|(name, _)| *name == "011_categories_brands_manufacturers")
        .expect("migration 011 exists");

    // Apply migrations prior to 011
    for (name, sql) in &crate::db::MIGRATIONS[..mig_011_idx] {
        let tx = conn.unchecked_transaction().expect("tx");
        tx.execute_batch(sql).expect("apply migration");
        tx.execute("INSERT INTO _migrations(name) VALUES (?1)", [name])
            .expect("record");
        tx.commit().expect("commit");
    }

    // Insert legacy category before 011
    conn.execute(
        "INSERT INTO categories (id, name, parent_id, created_at) VALUES ('legacy-cat-1', 'Groceries', NULL, '2026-01-01 00:00:00')",
        [],
    )
    .expect("insert legacy");

    // Now apply migration 011 through the standard runner
    crate::db::init_database(&conn).expect("migration runner applies 011 cleanly");

    let count: i64 = conn
        .query_row("SELECT COUNT(*) FROM _migrations", [], |row| row.get(0))
        .expect("count");
    assert_eq!(count, crate::db::MIGRATIONS.len() as i64);

    // Verify legacy category has default is_active=1 and updated_at populated
    let cat = get_category(&conn, "legacy-cat-1")
        .expect("query")
        .expect("found");
    assert_eq!(cat.name, "Groceries");
    assert!(cat.is_active);
    assert!(!cat.updated_at.is_empty());
}

#[test]
fn test_migration_011_conflict_rollback() {
    let conn = rusqlite::Connection::open_in_memory().expect("in-memory db");
    conn.execute_batch("PRAGMA foreign_keys = ON;")
        .expect("fk on");
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS _migrations (
            name TEXT PRIMARY KEY,
            applied_at TEXT NOT NULL DEFAULT (datetime('now'))
        );",
    )
    .expect("migrations table");

    let mig_011_idx = crate::db::MIGRATIONS
        .iter()
        .position(|(name, _)| *name == "011_categories_brands_manufacturers")
        .expect("migration 011 exists");

    // Apply migrations prior to 011
    for (name, sql) in &crate::db::MIGRATIONS[..mig_011_idx] {
        let tx = conn.unchecked_transaction().expect("tx");
        tx.execute_batch(sql).expect("apply migration");
        tx.execute("INSERT INTO _migrations(name) VALUES (?1)", [name])
            .expect("record");
        tx.commit().expect("commit");
    }

    // Insert conflicting active root categories ("Snacks" and "snacks")
    conn.execute(
        "INSERT INTO categories (id, name, parent_id, created_at) VALUES ('cat-1', 'Snacks', NULL, '2026-01-01 00:00:00')",
        [],
    )
    .expect("insert 1");
    conn.execute(
        "INSERT INTO categories (id, name, parent_id, created_at) VALUES ('cat-2', 'snacks', NULL, '2026-01-01 00:00:00')",
        [],
    )
    .expect("insert 2");

    // Full runner must fail atomically when 011 encounters conflicting data
    let result = crate::db::init_database(&conn);
    assert!(
        result.is_err(),
        "init_database must fail when migration 011 encounters conflicting active names"
    );

    // Verify migration 011 was NOT recorded in _migrations
    let mig_011_recorded: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM _migrations WHERE name = '011_categories_brands_manufacturers'",
            [],
            |r| r.get(0),
        )
        .expect("query");
    assert_eq!(
        mig_011_recorded, 0,
        "Failed migration 011 must not be recorded in ledger"
    );

    // Verify prior migrations remain recorded
    let prior_count: i64 = conn
        .query_row("SELECT COUNT(*) FROM _migrations", [], |r| r.get(0))
        .expect("query");
    assert_eq!(prior_count, mig_011_idx as i64);

    // Verify existing conflicting data remains untouched and uncorrupted
    let c1_name: String = conn
        .query_row("SELECT name FROM categories WHERE id = 'cat-1'", [], |r| {
            r.get(0)
        })
        .expect("query");
    let c2_name: String = conn
        .query_row("SELECT name FROM categories WHERE id = 'cat-2'", [], |r| {
            r.get(0)
        })
        .expect("query");
    assert_eq!(c1_name, "Snacks");
    assert_eq!(c2_name, "snacks");
}

#[test]
fn test_category_tree_includes_active_child_under_inactive_parent() {
    let conn = setup_test_db();
    let parent = create_category(
        &conn,
        make_category_fixture("Seasonal Inactive Parent", None),
    )
    .expect("parent created");
    let child = create_category(
        &conn,
        make_category_fixture("Active Evergreen Child", Some(&parent.id)),
    )
    .expect("child created");

    // Inactivate parent directly in DB (simulating legacy/inconsistent state)
    conn.execute(
        "UPDATE categories SET is_active = 0 WHERE id = ?1",
        [&parent.id],
    )
    .expect("archive parent in DB");

    // When querying active tree, child must be visible at top-level instead of silently disappearing
    let active_tree = get_category_tree(&conn, false).expect("active tree");
    let child_node = active_tree
        .iter()
        .find(|node| node.category.id == child.id)
        .expect("Active child under inactive parent must be visible in active tree");
    assert_eq!(child_node.category.name, "Active Evergreen Child");
}

#[test]
fn test_category_hierarchy_depth_limit_and_deep_valid_chain() {
    let conn = setup_test_db();

    // 10-level deep legitimate chain succeeds
    let mut prev_id = None;
    for i in 0..10 {
        let cat = create_category(
            &conn,
            make_category_fixture(&format!("Level {i}"), prev_id.as_deref()),
        )
        .expect("create level");
        prev_id = Some(cat.id);
    }

    // Cycle check on valid 10-level child moving under another root succeeds
    let new_root =
        create_category(&conn, make_category_fixture("New Root", None)).expect("new root");
    let leaf_id = prev_id.expect("10 levels created");
    let check = crate::category::check_category_cycle(&conn, &leaf_id, &new_root.id);
    assert!(check.is_ok());

    // Create chain exceeding MAX_DEFENSIVE_STEPS (50) to test HierarchyDepthExceeded
    let mut chain_head = None;
    for i in 0..52 {
        let cat_id = format!("deep_cat_{i:03}");
        let parent_id = if i == 0 {
            None
        } else {
            let prev_idx = i - 1;
            Some(format!("deep_cat_{prev_idx:03}"))
        };
        conn.execute(
            "INSERT INTO categories (id, name, parent_id, is_active, created_at, updated_at) VALUES (?1, ?2, ?3, 1, datetime('now'), datetime('now'))",
            rusqlite::params![cat_id, format!("Deep Node {i}"), parent_id],
        ).expect("insert chain");
        chain_head = Some(cat_id);
    }

    let head_id = chain_head.expect("chain head exists");
    let deep_err =
        crate::category::check_category_cycle(&conn, "unrelated_id", &head_id).unwrap_err();
    assert!(matches!(deep_err, CategoryError::HierarchyDepthExceeded(_)));
}

#[test]
fn test_category_tree_surfaces_stranded_cyclic_data() {
    let conn = setup_test_db();

    // Directly insert a cyclic pair (A -> B -> A) as might occur in corrupted/imported legacy data
    conn.execute(
        "INSERT INTO categories (id, name, parent_id, is_active, created_at, updated_at) VALUES ('cyc_a', 'Cycle A', 'cyc_b', 1, datetime('now'), datetime('now'))",
        [],
    ).expect("insert cyc_a");
    conn.execute(
        "INSERT INTO categories (id, name, parent_id, is_active, created_at, updated_at) VALUES ('cyc_b', 'Cycle B', 'cyc_a', 1, datetime('now'), datetime('now'))",
        [],
    ).expect("insert cyc_b");

    // Also insert a normal root category
    create_category(&conn, make_category_fixture("Normal Root", None)).expect("normal root");

    let tree = get_category_tree(&conn, false).expect("get tree");
    // Verify that the stranded cyclic nodes are surfaced as top-level nodes rather than silently dropped
    let names: Vec<String> = tree.iter().map(|n| n.category.name.clone()).collect();
    assert!(names.contains(&"Normal Root".to_string()));
    assert!(names.contains(&"Cycle A".to_string()) || names.contains(&"Cycle B".to_string()));
}
