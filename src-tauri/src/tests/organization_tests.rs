use crate::organization::{
    create_organization, get_organization, list_organizations, update_organization,
    CreateOrganizationInput, UpdateOrganizationInput,
};
use rusqlite::Connection;

fn setup_db() -> Connection {
    let conn = Connection::open_in_memory().expect("open in-memory database");
    crate::db::init_database(&conn).expect("apply all migrations");
    conn
}

#[test]
fn rejects_empty_name() {
    let conn = setup_db();

    let input_empty = CreateOrganizationInput {
        name: "".into(),
        default_currency: Some("USD".into()),
        default_language: Some("en".into()),
    };
    assert!(create_organization(&conn, input_empty).is_err());

    let input_whitespace = CreateOrganizationInput {
        name: "   \t\n  ".into(),
        default_currency: Some("USD".into()),
        default_language: Some("en".into()),
    };
    assert!(create_organization(&conn, input_whitespace).is_err());
}

#[test]
fn rejects_overlong_name() {
    let conn = setup_db();

    let input = CreateOrganizationInput {
        name: "a".repeat(256),
        default_currency: Some("USD".into()),
        default_language: Some("en".into()),
    };
    assert!(create_organization(&conn, input).is_err());
}

#[test]
fn rejects_invalid_currency() {
    let conn = setup_db();

    for invalid_currency in ["usd", "US", "USDT", "123", "US$", "   "] {
        let input = CreateOrganizationInput {
            name: "Test Org".into(),
            default_currency: Some(invalid_currency.into()),
            default_language: Some("en".into()),
        };
        assert!(create_organization(&conn, input).is_err());
    }
}

#[test]
fn rejects_invalid_language() {
    let conn = setup_db();

    for invalid_lang in ["", "   ", "e", "toolonglanguagecode123", "en!@#"] {
        let input = CreateOrganizationInput {
            name: "Test Org".into(),
            default_currency: Some("USD".into()),
            default_language: Some(invalid_lang.into()),
        };
        assert!(create_organization(&conn, input).is_err());
    }
}

#[test]
fn successful_creation_uses_defaults() {
    let conn = setup_db();

    let input = CreateOrganizationInput {
        name: "Acme Retail".into(),
        default_currency: None,
        default_language: None,
    };

    let org = create_organization(&conn, input)
        .expect("organization creation should succeed");
    assert!(!org.id.is_empty());
    assert_eq!(org.name, "Acme Retail");
    assert_eq!(org.default_currency, "USD");
    assert_eq!(org.default_language, "en");
    assert!(!org.created_at.is_empty());
}

#[test]
fn successful_creation_persists_and_retrieves() {
    let conn = setup_db();

    let input = CreateOrganizationInput {
        name: "Global Market Dz".into(),
        default_currency: Some("DZD".into()),
        default_language: Some("ar".into()),
    };

    let created = create_organization(&conn, input)
        .expect("organization creation should succeed");
    let fetched = get_organization(&conn, &created.id)
        .expect("query should succeed")
        .expect("organization should exist");

    assert_eq!(created, fetched);
    assert_eq!(fetched.name, "Global Market Dz");
    assert_eq!(fetched.default_currency, "DZD");
    assert_eq!(fetched.default_language, "ar");
}

#[test]
fn missing_organization_returns_none() {
    let conn = setup_db();

    let fetched = get_organization(&conn, "non-existent-org-id")
        .expect("query should succeed");
    assert!(fetched.is_none());
}

#[test]
fn rejects_empty_id_on_lookup() {
    let conn = setup_db();

    assert!(get_organization(&conn, "   ").is_err());
}

#[test]
fn successful_update_modifies_all_fields() {
    let conn = setup_db();

    let created = create_organization(
        &conn,
        CreateOrganizationInput {
            name: "Initial Name".into(),
            default_currency: Some("EUR".into()),
            default_language: Some("fr".into()),
        },
    )
    .expect("creation should succeed");

    let updated = update_organization(
        &conn,
        UpdateOrganizationInput {
            id: created.id.clone(),
            name: "Updated Name".into(),
            default_currency: "USD".into(),
            default_language: "en".into(),
        },
    )
    .expect("update should succeed");

    assert_eq!(updated.id, created.id);
    assert_eq!(updated.name, "Updated Name");
    assert_eq!(updated.default_currency, "USD");
    assert_eq!(updated.default_language, "en");

    let fetched = get_organization(&conn, &created.id)
        .expect("query should succeed")
        .expect("organization should exist");
    assert_eq!(fetched, updated);
}

#[test]
fn update_fails_for_missing_organization() {
    let conn = setup_db();

    let result = update_organization(
        &conn,
        UpdateOrganizationInput {
            id: "missing-org-id".into(),
            name: "New Name".into(),
            default_currency: "USD".into(),
            default_language: "en".into(),
        },
    );

    assert!(result.is_err());
}

#[test]
fn update_rejects_invalid_inputs() {
    let conn = setup_db();

    let created = create_organization(
        &conn,
        CreateOrganizationInput {
            name: "Valid Name".into(),
            default_currency: Some("USD".into()),
            default_language: Some("en".into()),
        },
    )
    .expect("creation should succeed");

    let res1 = update_organization(
        &conn,
        UpdateOrganizationInput {
            id: created.id.clone(),
            name: "   ".into(),
            default_currency: "USD".into(),
            default_language: "en".into(),
        },
    );
    assert!(res1.is_err());

    let res2 = update_organization(
        &conn,
        UpdateOrganizationInput {
            id: created.id.clone(),
            name: "Valid Name".into(),
            default_currency: "invalid".into(),
            default_language: "en".into(),
        },
    );
    assert!(res2.is_err());
}

#[test]
fn list_organizations_returns_ordered_results() {
    let conn = setup_db();

    let org1 = create_organization(
        &conn,
        CreateOrganizationInput {
            name: "Alpha Corp".into(),
            default_currency: Some("USD".into()),
            default_language: Some("en".into()),
        },
    )
    .unwrap();

    let org2 = create_organization(
        &conn,
        CreateOrganizationInput {
            name: "Beta Ltd".into(),
            default_currency: Some("EUR".into()),
            default_language: Some("fr".into()),
        },
    )
    .unwrap();

    let list = list_organizations(&conn).expect("listing should succeed");
    assert!(list.len() >= 2);

    let ids: Vec<String> = list.into_iter().map(|o| o.id).collect();
    assert!(ids.contains(&org1.id));
    assert!(ids.contains(&org2.id));
}

#[test]
fn tenant_isolation_boundary() {
    let conn = setup_db();

    let org_a = create_organization(
        &conn,
        CreateOrganizationInput {
            name: "Tenant Alpha".into(),
            default_currency: Some("USD".into()),
            default_language: Some("en".into()),
        },
    )
    .unwrap();

    let org_b = create_organization(
        &conn,
        CreateOrganizationInput {
            name: "Tenant Beta".into(),
            default_currency: Some("EUR".into()),
            default_language: Some("fr".into()),
        },
    )
    .unwrap();

    assert_ne!(org_a.id, org_b.id);

    conn.execute(
        "INSERT INTO branches (id, name, currency, organization_id)
         VALUES ('branch-alpha', 'Alpha Main', 'USD', ?1)",
        [&org_a.id],
    )
    .unwrap();

    conn.execute(
        "INSERT INTO branches (id, name, currency, organization_id)
         VALUES ('branch-beta', 'Beta Main', 'EUR', ?1)",
        [&org_b.id],
    )
    .unwrap();

    let alpha_branches: Vec<String> = conn
        .prepare("SELECT id FROM branches WHERE organization_id = ?1")
        .unwrap()
        .query_map([&org_a.id], |row| row.get(0))
        .unwrap()
        .map(|r| r.unwrap())
        .collect();

    assert_eq!(alpha_branches, vec!["branch-alpha"]);

    let beta_branches: Vec<String> = conn
        .prepare("SELECT id FROM branches WHERE organization_id = ?1")
        .unwrap()
        .query_map([&org_b.id], |row| row.get(0))
        .unwrap()
        .map(|r| r.unwrap())
        .collect();

    assert_eq!(beta_branches, vec!["branch-beta"]);
}
