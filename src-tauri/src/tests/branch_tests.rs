use crate::branch::{
    create_branch, get_branch, list_branches, update_branch, CreateBranchInput, UpdateBranchInput,
};
use crate::organization::{create_organization, CreateOrganizationInput};
use rusqlite::Connection;

fn setup_db() -> Connection {
    let conn = Connection::open_in_memory().expect("open in-memory database");
    crate::db::init_database(&conn).expect("apply all migrations");
    conn
}

fn create_test_org(conn: &Connection, name: &str, currency: &str) -> String {
    let org = create_organization(
        conn,
        CreateOrganizationInput {
            name: name.into(),
            default_currency: Some(currency.into()),
            default_language: Some("en".into()),
        },
    )
    .expect("test organization creation should succeed");
    org.id
}

#[test]
fn rejects_empty_name() {
    let conn = setup_db();
    let org_id = create_test_org(&conn, "Org Name", "USD");

    let input_empty = CreateBranchInput {
        organization_id: org_id.clone(),
        name: "".into(),
        address: None,
        currency: Some("USD".into()),
        is_active: Some(true),
    };
    assert!(create_branch(&conn, input_empty).is_err());

    let input_whitespace = CreateBranchInput {
        organization_id: org_id,
        name: "   \t\n  ".into(),
        address: None,
        currency: Some("USD".into()),
        is_active: Some(true),
    };
    assert!(create_branch(&conn, input_whitespace).is_err());
}

#[test]
fn rejects_overlong_name() {
    let conn = setup_db();
    let org_id = create_test_org(&conn, "Org Name", "USD");

    let input = CreateBranchInput {
        organization_id: org_id,
        name: "a".repeat(256),
        address: None,
        currency: Some("USD".into()),
        is_active: Some(true),
    };
    assert!(create_branch(&conn, input).is_err());
}

#[test]
fn rejects_invalid_currency() {
    let conn = setup_db();
    let org_id = create_test_org(&conn, "Org Name", "USD");

    for invalid_currency in ["usd", "US", "USDT", "123", "US$", "   "] {
        let input = CreateBranchInput {
            organization_id: org_id.clone(),
            name: "Main Branch".into(),
            address: None,
            currency: Some(invalid_currency.into()),
            is_active: Some(true),
        };
        assert!(create_branch(&conn, input).is_err());
    }
}

#[test]
fn rejects_empty_id_on_lookup() {
    let conn = setup_db();
    assert!(get_branch(&conn, "   ").is_err());
}

#[test]
fn rejects_non_existent_organization() {
    let conn = setup_db();

    let input = CreateBranchInput {
        organization_id: "non-existent-org-id".into(),
        name: "Main Branch".into(),
        address: None,
        currency: Some("USD".into()),
        is_active: Some(true),
    };

    let result = create_branch(&conn, input);
    assert!(result.is_err());
}

#[test]
fn successful_creation_with_explicit_values() {
    let conn = setup_db();
    let org_id = create_test_org(&conn, "Acme Corp", "USD");

    let input = CreateBranchInput {
        organization_id: org_id.clone(),
        name: "Downtown Branch".into(),
        address: Some("123 Main St".into()),
        currency: Some("EUR".into()),
        is_active: Some(true),
    };

    let branch = create_branch(&conn, input).expect("branch creation should succeed");
    assert!(!branch.id.is_empty());
    assert_eq!(branch.organization_id, org_id);
    assert_eq!(branch.name, "Downtown Branch");
    assert_eq!(branch.address, Some("123 Main St".into()));
    assert_eq!(branch.currency, "EUR");
    assert!(branch.is_active);
    assert!(!branch.created_at.is_empty());
}

#[test]
fn currency_inherits_from_organization_when_omitted() {
    let conn = setup_db();
    let org_id = create_test_org(&conn, "Global Market Dz", "DZD");

    let input = CreateBranchInput {
        organization_id: org_id.clone(),
        name: "Algiers Central".into(),
        address: None,
        currency: None,
        is_active: None,
    };

    let branch = create_branch(&conn, input).expect("branch creation should succeed");
    assert_eq!(branch.organization_id, org_id);
    assert_eq!(branch.currency, "DZD");
    assert!(branch.is_active);
}

#[test]
fn is_active_defaults_to_true_when_omitted() {
    let conn = setup_db();
    let org_id = create_test_org(&conn, "Test Org", "USD");

    let input = CreateBranchInput {
        organization_id: org_id,
        name: "North Branch".into(),
        address: None,
        currency: Some("USD".into()),
        is_active: None,
    };

    let branch = create_branch(&conn, input).expect("creation should succeed");
    assert!(branch.is_active);
}

#[test]
fn uuid_uniqueness_across_branches() {
    let conn = setup_db();
    let org_id = create_test_org(&conn, "Multi Branch Org", "USD");

    let b1 = create_branch(
        &conn,
        CreateBranchInput {
            organization_id: org_id.clone(),
            name: "Branch 1".into(),
            address: None,
            currency: Some("USD".into()),
            is_active: Some(true),
        },
    )
    .unwrap();

    let b2 = create_branch(
        &conn,
        CreateBranchInput {
            organization_id: org_id,
            name: "Branch 2".into(),
            address: None,
            currency: Some("USD".into()),
            is_active: Some(true),
        },
    )
    .unwrap();

    assert_ne!(b1.id, b2.id);
}

#[test]
fn get_branch_existing_and_non_existing() {
    let conn = setup_db();
    let org_id = create_test_org(&conn, "Test Org", "USD");

    let created = create_branch(
        &conn,
        CreateBranchInput {
            organization_id: org_id,
            name: "Main Branch".into(),
            address: Some("456 Elm St".into()),
            currency: Some("USD".into()),
            is_active: Some(true),
        },
    )
    .expect("creation should succeed");

    let fetched = get_branch(&conn, &created.id)
        .expect("query should succeed")
        .expect("branch should exist");

    assert_eq!(created, fetched);

    let missing = get_branch(&conn, "non-existent-id").expect("query should succeed");
    assert!(missing.is_none());
}

#[test]
fn successful_update_modifies_fields() {
    let conn = setup_db();
    let org_id = create_test_org(&conn, "Update Test Org", "USD");

    let created = create_branch(
        &conn,
        CreateBranchInput {
            organization_id: org_id,
            name: "Old Name".into(),
            address: Some("Old Address".into()),
            currency: Some("USD".into()),
            is_active: Some(true),
        },
    )
    .expect("creation should succeed");

    let updated = update_branch(
        &conn,
        UpdateBranchInput {
            id: created.id.clone(),
            name: "New Name".into(),
            address: Some("New Address".into()),
            currency: "EUR".into(),
            is_active: false,
        },
    )
    .expect("update should succeed");

    assert_eq!(updated.id, created.id);
    assert_eq!(updated.name, "New Name");
    assert_eq!(updated.address, Some("New Address".into()));
    assert_eq!(updated.currency, "EUR");
    assert!(!updated.is_active);

    let fetched = get_branch(&conn, &created.id)
        .expect("query should succeed")
        .expect("branch should exist");
    assert_eq!(fetched, updated);
}

#[test]
fn update_fails_for_missing_branch() {
    let conn = setup_db();

    let result = update_branch(
        &conn,
        UpdateBranchInput {
            id: "missing-branch-id".into(),
            name: "New Name".into(),
            address: None,
            currency: "USD".into(),
            is_active: true,
        },
    );

    assert!(result.is_err());
}

#[test]
fn update_rejects_invalid_inputs() {
    let conn = setup_db();
    let org_id = create_test_org(&conn, "Org", "USD");

    let created = create_branch(
        &conn,
        CreateBranchInput {
            organization_id: org_id,
            name: "Valid Branch".into(),
            address: None,
            currency: Some("USD".into()),
            is_active: Some(true),
        },
    )
    .expect("creation should succeed");

    let res_empty_name = update_branch(
        &conn,
        UpdateBranchInput {
            id: created.id.clone(),
            name: "   ".into(),
            address: None,
            currency: "USD".into(),
            is_active: true,
        },
    );
    assert!(res_empty_name.is_err());

    let res_invalid_curr = update_branch(
        &conn,
        UpdateBranchInput {
            id: created.id,
            name: "Valid Name".into(),
            address: None,
            currency: "invalid".into(),
            is_active: true,
        },
    );
    assert!(res_invalid_curr.is_err());
}

#[test]
fn list_branches_returns_deterministic_ordering() {
    let conn = setup_db();
    let org_id = create_test_org(&conn, "Listing Org", "USD");

    let b1 = create_branch(
        &conn,
        CreateBranchInput {
            organization_id: org_id.clone(),
            name: "Branch Alpha".into(),
            address: None,
            currency: Some("USD".into()),
            is_active: Some(true),
        },
    )
    .unwrap();

    let b2 = create_branch(
        &conn,
        CreateBranchInput {
            organization_id: org_id.clone(),
            name: "Branch Beta".into(),
            address: None,
            currency: Some("USD".into()),
            is_active: Some(true),
        },
    )
    .unwrap();

    let list = list_branches(&conn, &org_id).expect("listing should succeed");
    assert_eq!(list.len(), 2);
    assert_eq!(list[0].id, b1.id);
    assert_eq!(list[1].id, b2.id);
}

#[test]
fn strict_multi_tenant_isolation() {
    let conn = setup_db();

    let org_a = create_test_org(&conn, "Tenant Alpha", "USD");
    let org_b = create_test_org(&conn, "Tenant Beta", "EUR");

    assert_ne!(org_a, org_b);

    let branch_a1 = create_branch(
        &conn,
        CreateBranchInput {
            organization_id: org_a.clone(),
            name: "Alpha Branch 1".into(),
            address: None,
            currency: Some("USD".into()),
            is_active: Some(true),
        },
    )
    .unwrap();

    let branch_a2 = create_branch(
        &conn,
        CreateBranchInput {
            organization_id: org_a.clone(),
            name: "Alpha Branch 2".into(),
            address: None,
            currency: Some("USD".into()),
            is_active: Some(true),
        },
    )
    .unwrap();

    let branch_b1 = create_branch(
        &conn,
        CreateBranchInput {
            organization_id: org_b.clone(),
            name: "Beta Branch 1".into(),
            address: None,
            currency: Some("EUR".into()),
            is_active: Some(true),
        },
    )
    .unwrap();

    let list_a = list_branches(&conn, &org_a).expect("listing org a branches");
    let list_b = list_branches(&conn, &org_b).expect("listing org b branches");

    let ids_a: Vec<String> = list_a.into_iter().map(|b| b.id).collect();
    let ids_b: Vec<String> = list_b.into_iter().map(|b| b.id).collect();

    assert_eq!(ids_a, vec![branch_a1.id, branch_a2.id]);
    assert_eq!(ids_b, vec![branch_b1.id]);
    assert!(!ids_a.contains(&branch_b1.id));
}
