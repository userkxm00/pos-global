use rusqlite::{Connection, Result};
use std::path::Path;

const MIGRATIONS: &[(&str, &str)] = &[
    ("001_initial", include_str!("migrations/001_initial.sql")),
    ("002_sync_and_suppliers", include_str!("migrations/002_sync_and_suppliers.sql")),
    ("003_global_commerce_foundation", include_str!("migrations/003_global_commerce_foundation.sql")),
    ("004_exact_money_and_identity", include_str!("migrations/004_exact_money_and_identity.sql")),
    ("005_tenancy_and_financial_hardening", include_str!("migrations/005_tenancy_and_financial_hardening.sql")),
];

pub fn open_database(path: &Path) -> Result<Connection> {
    let conn = Connection::open(path)?;
    init_database(&conn)?;
    Ok(conn)
}

pub fn init_database(conn: &Connection) -> Result<()> {
    conn.execute_batch("PRAGMA foreign_keys = ON;")?;
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS _migrations (
            name TEXT PRIMARY KEY,
            applied_at TEXT NOT NULL DEFAULT (datetime('now'))
        );",
    )?;

    for (name, sql) in MIGRATIONS {
        let applied: i64 = conn.query_row(
            "SELECT COUNT(*) FROM _migrations WHERE name = ?1",
            [name],
            |row| row.get(0),
        )?;
        if applied == 0 {
            let tx = conn.unchecked_transaction()?;
            tx.execute_batch(sql)?;
            tx.execute("INSERT INTO _migrations(name) VALUES (?1)", [name])?;
            tx.commit()?;
        }
    }
    Ok(())
}
