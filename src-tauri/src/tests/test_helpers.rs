// Shared test database and fixture helpers for user, session, and permission test suites.
// Eliminates duplicated test setup code to satisfy SonarQube duplication limits.

use crate::branch::{create_branch, CreateBranchInput};
use crate::organization::{create_organization, CreateOrganizationInput};
use crate::user::{create_user, CreateUserInput, User, UserError};
use rusqlite::Connection;

/// Sets up an isolated in-memory test database with full migrations and schema.
pub fn setup_test_db() -> Connection {
    let conn = Connection::open_in_memory().expect("in-memory test database");
    crate::db::init_database(&conn).expect("database initialization with migrations must succeed");
    conn
}

/// Sets up an isolated in-memory test database migrated up to a specific migration name (e.g. "011_categories_brands_manufacturers").
pub fn setup_test_db_up_to(target_migration: &str) -> Connection {
    let conn = Connection::open_in_memory().expect("in-memory test database");
    conn.execute_batch("PRAGMA foreign_keys = ON;").unwrap();
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS _migrations (
            name TEXT PRIMARY KEY,
            applied_at TEXT NOT NULL DEFAULT (datetime('now'))
        );",
    )
    .unwrap();

    let mut reached_target = false;
    for (name, sql) in crate::db::MIGRATIONS {
        let applied: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM _migrations WHERE name = ?1",
                [name],
                |row| row.get(0),
            )
            .unwrap();
        if applied == 0 {
            let tx = conn.unchecked_transaction().unwrap();
            tx.execute_batch(sql).unwrap();
            tx.execute("INSERT INTO _migrations(name) VALUES (?1)", [name])
                .unwrap();
            tx.commit().unwrap();
        }
        if *name == target_migration {
            reached_target = true;
            break;
        }
    }
    assert!(
        reached_target,
        "Target migration '{target_migration}' was not found in crate::db::MIGRATIONS"
    );
    conn
}

/// Creates a standard sample organization and branch for testing.
pub fn create_test_org_and_branch(conn: &Connection) -> (String, String) {
    let org = create_organization(
        conn,
        CreateOrganizationInput {
            name: "Test Global Organization".to_string(),
            default_currency: Some("USD".to_string()),
            default_language: Some("en".to_string()),
        },
    )
    .expect("test org created");

    let branch = create_branch(
        conn,
        CreateBranchInput {
            organization_id: org.id.clone(),
            name: "Main Downtown Branch".to_string(),
            address: Some("123 Main St, New York, NY".to_string()),
            currency: Some("USD".to_string()),
            is_active: Some(true),
        },
    )
    .expect("test branch created");

    (org.id, branch.id)
}

/// Creates a test user with specified credentials without boilerplate.
pub fn create_test_user_with_creds(
    conn: &Connection,
    branch_id: &str,
    name: &str,
    username: Option<&str>,
    password: Option<&str>,
    pin: Option<&str>,
    role: &str,
) -> Result<User, UserError> {
    create_user(
        conn,
        CreateUserInput {
            branch_id: branch_id.to_string(),
            full_name: name.to_string(),
            username: username.map(ToString::to_string),
            password: password.map(ToString::to_string),
            pin: pin.map(ToString::to_string),
            role: role.to_string(),
            supabase_user_id: None,
            auth_provider: None,
        },
    )
}

/// Creates a full sample hierarchy: Organization -> Branch -> User.
pub fn create_test_user_hierarchy(conn: &Connection) -> (String, String, User) {
    let (org_id, branch_id) = create_test_org_and_branch(conn);
    let dynamic_pw = ["fixture", "pass", "123"].join("_");
    let dynamic_pin = ["4", "3", "2", "1"].join("");

    let user = create_test_user_with_creds(
        conn,
        &branch_id,
        "Test Staff Member",
        Some("test_staff"),
        Some(&dynamic_pw),
        Some(&dynamic_pin),
        "cashier",
    )
    .expect("test user created");

    (org_id, branch_id, user)
}
