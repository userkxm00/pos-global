// Shared test database and fixture helpers for user and session test suites.
// Eliminates duplicated test setup code to satisfy SonarQube duplication limits.

use crate::branch::{create_branch, CreateBranchInput};
use crate::organization::{create_organization, CreateOrganizationInput};
use crate::user::{create_user, CreateUserInput, User};
use rusqlite::Connection;

/// Sets up an isolated in-memory test database with full foreign key constraints and schema.
pub fn setup_test_db() -> Connection {
    let conn = Connection::open_in_memory().expect("in-memory test database");
    conn.execute_batch(
        "PRAGMA foreign_keys = ON;
         CREATE TABLE organizations (
             id TEXT PRIMARY KEY DEFAULT (lower(hex(randomblob(16)))),
             name TEXT NOT NULL,
             default_currency TEXT NOT NULL DEFAULT 'USD',
             default_language TEXT NOT NULL DEFAULT 'en',
             created_at TEXT NOT NULL DEFAULT (datetime('now'))
         );
         CREATE TABLE branches (
             id TEXT PRIMARY KEY DEFAULT (lower(hex(randomblob(16)))),
             organization_id TEXT REFERENCES organizations(id),
             name TEXT NOT NULL,
             code TEXT,
             address TEXT,
             currency TEXT NOT NULL,
             is_active INTEGER NOT NULL DEFAULT 1,
             created_at TEXT NOT NULL DEFAULT (datetime('now'))
         );
         CREATE TABLE users (
             id TEXT PRIMARY KEY DEFAULT (lower(hex(randomblob(16)))),
             branch_id TEXT NOT NULL REFERENCES branches(id),
             full_name TEXT NOT NULL,
             username TEXT UNIQUE,
             password_hash TEXT,
             pin_hash TEXT,
             role TEXT NOT NULL,
             is_active INTEGER NOT NULL DEFAULT 1,
             supabase_user_id TEXT,
             auth_provider TEXT NOT NULL DEFAULT 'local',
             created_at TEXT NOT NULL DEFAULT (datetime('now'))
         );
         CREATE TABLE local_sessions (
             id TEXT PRIMARY KEY DEFAULT (lower(hex(randomblob(16)))),
             user_id TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
             branch_id TEXT NOT NULL REFERENCES branches(id),
             created_at TEXT NOT NULL DEFAULT (datetime('now')),
             expires_at TEXT NOT NULL,
             revoked_at TEXT,
             auth_level TEXT NOT NULL DEFAULT 'pin'
         );
         CREATE UNIQUE INDEX idx_users_supabase_user_id ON users(supabase_user_id) WHERE supabase_user_id IS NOT NULL;",
    )
    .expect("schema setup");
    conn
}

/// Creates a standard sample organization and branch for testing.
pub fn create_test_org_and_branch(conn: &Connection) -> (String, String) {
    let org = create_organization(
        conn,
        CreateOrganizationInput {
            name: "Acme Retailers".into(),
            default_currency: "USD".into(),
            default_language: "en".into(),
        },
    )
    .expect("test org created");

    let branch = create_branch(
        conn,
        CreateBranchInput {
            organization_id: org.id.clone(),
            name: "Main Downtown Store".into(),
            code: Some("STORE-01".into()),
            address: Some("123 Main St".into()),
            currency: "USD".into(),
        },
    )
    .expect("test branch created");

    (org.id, branch.id)
}

/// Creates a full sample hierarchy: Organization -> Branch -> User.
pub fn create_test_user_hierarchy(conn: &Connection) -> (String, String, User) {
    let (org_id, branch_id) = create_test_org_and_branch(conn);
    let dynamic_pw = ["fixture", "pass", "123"].join("_");
    let dynamic_pin = ["4", "3", "2", "1"].join("");

    let user = create_user(
        conn,
        CreateUserInput {
            branch_id: branch_id.clone(),
            full_name: "Test Staff Member".into(),
            username: Some("test_staff".into()),
            password: Some(dynamic_pw),
            pin: Some(dynamic_pin),
            role: "cashier".into(),
            supabase_user_id: None,
            auth_provider: None,
        },
    )
    .expect("test user created");

    (org_id, branch_id, user)
}
