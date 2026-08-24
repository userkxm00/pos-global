use crate::branch::{create_branch, CreateBranchInput};
use crate::organization::{create_organization, CreateOrganizationInput};
use crate::register::{
    create_register, get_register, list_registers, update_register, CreateRegisterInput,
    RegisterError, UpdateRegisterInput,
};
use rusqlite::Connection;

fn setup_db() -> Connection {
    let conn = Connection::open_in_memory().expect("open in-memory database");
    crate::db::init_database(&conn).expect("apply all migrations");
    conn
}

fn create_test_org_and_branch(
    conn: &Connection,
    org_name: &str,
    branch_name: &str,
) -> (String, String) {
    let org = create_organization(
        conn,
        CreateOrganizationInput {
            name: org_name.into(),
            default_currency: Some("USD".into()),
            default_language: Some("en".into()),
        },
    )
    .expect("test organization creation should succeed");

    let branch = create_branch(
        conn,
        CreateBranchInput {
            organization_id: org.id.clone(),
            name: branch_name.into(),
            address: None,
            currency: Some("USD".into()),
            is_active: Some(true),
        },
    )
    .expect("test branch creation should succeed");

    (org.id, branch.id)
}

#[test]
fn rejects_empty_name() {
    let conn = setup_db();
    let (org_id, branch_id) = create_test_org_and_branch(&conn, "Org", "Branch");

    let input_empty = CreateRegisterInput {
        organization_id: org_id.clone(),
        branch_id: branch_id.clone(),
        name: "".into(),
        code: None,
        is_active: Some(true),
    };
    assert!(create_register(&conn, input_empty).is_err());

    let input_whitespace = CreateRegisterInput {
        organization_id: org_id,
        branch_id,
        name: "   \t\n  ".into(),
        code: None,
        is_active: Some(true),
    };
    assert!(create_register(&conn, input_whitespace).is_err());
}

#[test]
fn accepts_unicode_name_255_chars() {
    let conn = setup_db();
    let (org_id, branch_id) = create_test_org_and_branch(&conn, "Org", "Branch");

    let unicode_255 = "ص".repeat(255);
    assert_eq!(unicode_255.chars().count(), 255);
    assert!(unicode_255.len() > 255);

    let input = CreateRegisterInput {
        organization_id: org_id,
        branch_id,
        name: unicode_255.clone(),
        code: Some("REG-01".into()),
        is_active: Some(true),
    };
    let reg = create_register(&conn, input).expect("255-character unicode name should be accepted");
    assert_eq!(reg.name, unicode_255);
}

#[test]
fn rejects_unicode_name_256_chars() {
    let conn = setup_db();
    let (org_id, branch_id) = create_test_org_and_branch(&conn, "Org", "Branch");

    let unicode_256 = "ص".repeat(256);
    assert_eq!(unicode_256.chars().count(), 256);

    let input = CreateRegisterInput {
        organization_id: org_id,
        branch_id,
        name: unicode_256,
        code: None,
        is_active: Some(true),
    };
    assert!(create_register(&conn, input).is_err());
}

#[test]
fn rejects_empty_ids() {
    let conn = setup_db();
    let (org_id, branch_id) = create_test_org_and_branch(&conn, "Org", "Branch");

    assert!(get_register(&conn, "   ").is_err());

    let input_bad_org = CreateRegisterInput {
        organization_id: "   ".into(),
        branch_id: branch_id.clone(),
        name: "Register 1".into(),
        code: None,
        is_active: Some(true),
    };
    assert!(create_register(&conn, input_bad_org).is_err());

    let input_bad_branch = CreateRegisterInput {
        organization_id: org_id,
        branch_id: "   ".into(),
        name: "Register 1".into(),
        code: None,
        is_active: Some(true),
    };
    assert!(create_register(&conn, input_bad_branch).is_err());
}

#[test]
fn rejects_non_existent_organization() {
    let conn = setup_db();
    let (_org_id, branch_id) = create_test_org_and_branch(&conn, "Org", "Branch");

    let input = CreateRegisterInput {
        organization_id: "non-existent-org".into(),
        branch_id,
        name: "Register 1".into(),
        code: None,
        is_active: Some(true),
    };
    let result = create_register(&conn, input);
    assert!(matches!(result, Err(RegisterError::InvalidOrganization(_))));
}

#[test]
fn rejects_non_existent_branch() {
    let conn = setup_db();
    let (org_id, _branch_id) = create_test_org_and_branch(&conn, "Org", "Branch");

    let input = CreateRegisterInput {
        organization_id: org_id,
        branch_id: "non-existent-branch".into(),
        name: "Register 1".into(),
        code: None,
        is_active: Some(true),
    };
    let result = create_register(&conn, input);
    assert!(matches!(result, Err(RegisterError::InvalidBranch(_))));
}

#[test]
fn rejects_branch_belonging_to_another_organization() {
    let conn = setup_db();
    let (org_a, _branch_a) = create_test_org_and_branch(&conn, "Org A", "Branch A");
    let (_org_b, branch_b) = create_test_org_and_branch(&conn, "Org B", "Branch B");

    // Attempt to create register under Org A but pointing to Branch B
    let input = CreateRegisterInput {
        organization_id: org_a,
        branch_id: branch_b,
        name: "Cross Tenant Register".into(),
        code: None,
        is_active: Some(true),
    };
    let result = create_register(&conn, input);
    assert!(matches!(result, Err(RegisterError::InvalidBranch(_))));
}

#[test]
fn successful_creation_with_explicit_and_default_values() {
    let conn = setup_db();
    let (org_id, branch_id) = create_test_org_and_branch(&conn, "Acme Retail", "Main Branch");

    let input = CreateRegisterInput {
        organization_id: org_id.clone(),
        branch_id: branch_id.clone(),
        name: "Checkout 01".into(),
        code: Some("REG-01".into()),
        is_active: None,
    };

    let reg = create_register(&conn, input).expect("register creation should succeed");
    assert!(!reg.id.is_empty());
    assert_eq!(reg.organization_id, org_id);
    assert_eq!(reg.branch_id, branch_id);
    assert_eq!(reg.name, "Checkout 01");
    assert_eq!(reg.code, Some("REG-01".into()));
    assert!(reg.is_active);
    assert!(!reg.created_at.is_empty());
}

#[test]
fn uuid_uniqueness_across_registers() {
    let conn = setup_db();
    let (org_id, branch_id) = create_test_org_and_branch(&conn, "Org", "Branch");

    let r1 = create_register(
        &conn,
        CreateRegisterInput {
            organization_id: org_id.clone(),
            branch_id: branch_id.clone(),
            name: "Reg 1".into(),
            code: None,
            is_active: Some(true),
        },
    )
    .unwrap();

    let r2 = create_register(
        &conn,
        CreateRegisterInput {
            organization_id: org_id,
            branch_id,
            name: "Reg 2".into(),
            code: None,
            is_active: Some(true),
        },
    )
    .unwrap();

    assert_ne!(r1.id, r2.id);
}

#[test]
fn get_register_existing_and_non_existing() {
    let conn = setup_db();
    let (org_id, branch_id) = create_test_org_and_branch(&conn, "Org", "Branch");

    let created = create_register(
        &conn,
        CreateRegisterInput {
            organization_id: org_id,
            branch_id,
            name: "Front Register".into(),
            code: Some("FR-1".into()),
            is_active: Some(true),
        },
    )
    .expect("creation should succeed");

    let fetched = get_register(&conn, &created.id)
        .expect("query should succeed")
        .expect("register should exist");

    assert_eq!(created, fetched);

    let missing = get_register(&conn, "non-existent-id").expect("query should succeed");
    assert!(missing.is_none());
}

#[test]
fn successful_update_modifies_fields() {
    let conn = setup_db();
    let (org_id, branch_id) = create_test_org_and_branch(&conn, "Org", "Branch");

    let created = create_register(
        &conn,
        CreateRegisterInput {
            organization_id: org_id,
            branch_id,
            name: "Old Name".into(),
            code: Some("OLD-CODE".into()),
            is_active: Some(true),
        },
    )
    .expect("creation should succeed");

    let updated = update_register(
        &conn,
        UpdateRegisterInput {
            id: created.id.clone(),
            name: "New Name".into(),
            code: Some("NEW-CODE".into()),
            is_active: false,
        },
    )
    .expect("update should succeed");

    assert_eq!(updated.id, created.id);
    assert_eq!(updated.name, "New Name");
    assert_eq!(updated.code, Some("NEW-CODE".into()));
    assert!(!updated.is_active);

    let fetched = get_register(&conn, &created.id)
        .expect("query should succeed")
        .expect("register should exist");
    assert_eq!(fetched, updated);
}

#[test]
fn update_fails_for_missing_register() {
    let conn = setup_db();

    let result = update_register(
        &conn,
        UpdateRegisterInput {
            id: "missing-register-id".into(),
            name: "New Name".into(),
            code: None,
            is_active: true,
        },
    );

    assert!(result.is_err());
}

#[test]
fn update_rejects_invalid_inputs() {
    let conn = setup_db();
    let (org_id, branch_id) = create_test_org_and_branch(&conn, "Org", "Branch");

    let created = create_register(
        &conn,
        CreateRegisterInput {
            organization_id: org_id,
            branch_id,
            name: "Valid Register".into(),
            code: None,
            is_active: Some(true),
        },
    )
    .expect("creation should succeed");

    let res_empty_name = update_register(
        &conn,
        UpdateRegisterInput {
            id: created.id,
            name: "   ".into(),
            code: None,
            is_active: true,
        },
    );
    assert!(res_empty_name.is_err());
}

#[test]
fn list_registers_returns_deterministic_ordering() {
    let conn = setup_db();
    let (org_id, branch_id) = create_test_org_and_branch(&conn, "Listing Org", "Listing Branch");

    // Insert with explicit distinct timestamps to verify created_at ASC ordering
    conn.execute(
        "INSERT INTO registers (id, organization_id, branch_id, name, code, is_active, created_at)
         VALUES ('reg-earlier', ?1, ?2, 'Earlier Register', NULL, 1, '2026-01-01 10:00:00')",
        [&org_id, &branch_id],
    )
    .unwrap();

    conn.execute(
        "INSERT INTO registers (id, organization_id, branch_id, name, code, is_active, created_at)
         VALUES ('reg-later', ?1, ?2, 'Later Register', NULL, 1, '2026-01-02 10:00:00')",
        [&org_id, &branch_id],
    )
    .unwrap();

    // Insert two registers sharing the exact same timestamp to verify id ASC tie-breaking
    conn.execute(
        "INSERT INTO registers (id, organization_id, branch_id, name, code, is_active, created_at)
         VALUES ('reg-same-z', ?1, ?2, 'Same Z', NULL, 1, '2026-01-03 12:00:00')",
        [&org_id, &branch_id],
    )
    .unwrap();

    conn.execute(
        "INSERT INTO registers (id, organization_id, branch_id, name, code, is_active, created_at)
         VALUES ('reg-same-a', ?1, ?2, 'Same A', NULL, 1, '2026-01-03 12:00:00')",
        [&org_id, &branch_id],
    )
    .unwrap();

    let list = list_registers(&conn, &branch_id).expect("listing should succeed");
    assert_eq!(list.len(), 4);

    let ids: Vec<String> = list.into_iter().map(|r| r.id).collect();
    assert_eq!(
        ids,
        vec![
            "reg-earlier".to_string(),
            "reg-later".to_string(),
            "reg-same-a".to_string(),
            "reg-same-z".to_string(),
        ]
    );
}

#[test]
fn strict_multi_tenant_isolation() {
    let conn = setup_db();

    let (org_a, branch_a) = create_test_org_and_branch(&conn, "Org A", "Branch A");
    let (org_b, branch_b) = create_test_org_and_branch(&conn, "Org B", "Branch B");

    let reg_a1 = create_register(
        &conn,
        CreateRegisterInput {
            organization_id: org_a.clone(),
            branch_id: branch_a.clone(),
            name: "Reg A1".into(),
            code: None,
            is_active: Some(true),
        },
    )
    .unwrap();

    let reg_a2 = create_register(
        &conn,
        CreateRegisterInput {
            organization_id: org_a,
            branch_id: branch_a.clone(),
            name: "Reg A2".into(),
            code: None,
            is_active: Some(true),
        },
    )
    .unwrap();

    let reg_b1 = create_register(
        &conn,
        CreateRegisterInput {
            organization_id: org_b,
            branch_id: branch_b.clone(),
            name: "Reg B1".into(),
            code: None,
            is_active: Some(true),
        },
    )
    .unwrap();

    let list_a = list_registers(&conn, &branch_a).expect("listing branch a registers");
    let list_b = list_registers(&conn, &branch_b).expect("listing branch b registers");

    let ids_a: Vec<String> = list_a.into_iter().map(|r| r.id).collect();
    let ids_b: Vec<String> = list_b.into_iter().map(|r| r.id).collect();

    assert_eq!(ids_a.len(), 2);
    assert!(ids_a.contains(&reg_a1.id));
    assert!(ids_a.contains(&reg_a2.id));
    assert!(!ids_a.contains(&reg_b1.id));

    assert_eq!(ids_b.len(), 1);
    assert_eq!(ids_b, vec![reg_b1.id]);
}

#[test]
fn legacy_null_foreign_key_returns_database_error() {
    let conn = Connection::open_in_memory().expect("open in-memory database");

    // Temporarily disable foreign keys and create legacy table structure to simulate corrupt NULL foreign key
    conn.execute_batch(
        "PRAGMA foreign_keys = OFF;
         CREATE TABLE registers (
             id TEXT PRIMARY KEY,
             organization_id TEXT,
             branch_id TEXT,
             name TEXT,
             code TEXT,
             is_active INTEGER,
             created_at TEXT
         );
         INSERT INTO registers (id, organization_id, branch_id, name, code, is_active, created_at)
         VALUES ('corrupt-reg', 'org-1', NULL, 'Corrupt Register', NULL, 1, datetime('now'));
         PRAGMA foreign_keys = ON;",
    )
    .unwrap();

    let get_result = get_register(&conn, "corrupt-reg");
    assert!(
        matches!(get_result, Err(RegisterError::Database(ref msg)) if msg.contains("corrupt or NULL branch_id"))
    );
}
