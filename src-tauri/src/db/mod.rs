use rusqlite::{Connection, Result};
use std::path::Path;
use std::sync::Mutex;

pub struct DbState(pub Mutex<Connection>);

const MIGRATIONS: &[(&str, &str)] = &[
    ("001_initial", include_str!("migrations/001_initial.sql")),
    (
        "002_sync_and_suppliers",
        include_str!("migrations/002_sync_and_suppliers.sql"),
    ),
    (
        "003_global_commerce_foundation",
        include_str!("migrations/003_global_commerce_foundation.sql"),
    ),
    (
        "004_exact_money_and_identity",
        include_str!("migrations/004_exact_money_and_identity.sql"),
    ),
    (
        "005_tenancy_and_financial_hardening",
        include_str!("migrations/005_tenancy_and_financial_hardening.sql"),
    ),
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

#[cfg(test)]
mod tests {
    use super::init_database;
    use rusqlite::Connection;

    #[test]
    fn all_foundation_migrations_apply_to_empty_database() {
        let conn = Connection::open_in_memory().expect("open in-memory database");
        init_database(&conn).expect("all migrations should apply cleanly");

        let applied: i64 = conn
            .query_row("SELECT COUNT(*) FROM _migrations", [], |row| row.get(0))
            .expect("migration ledger should exist");
        assert_eq!(applied, 5);

        for table in [
            "business_settings",
            "branches",
            "products",
            "product_variants",
            "inventory",
            "suppliers",
            "purchase_orders",
            "outbox_events",
            "permissions",
            "organizations",
            "cash_movements",
            "debt_ledger",
            "loyalty_ledger",
        ] {
            let exists: i64 = conn
                .query_row(
                    "SELECT COUNT(*) FROM sqlite_master WHERE type = 'table' AND name = ?1",
                    [table],
                    |row| row.get(0),
                )
                .expect("schema query should succeed");
            assert_eq!(exists, 1, "expected table {table} to exist");
        }
    }

    #[test]
    fn migration_initialization_is_repeatable() {
        let conn = Connection::open_in_memory().expect("open in-memory database");
        init_database(&conn).expect("first initialization should succeed");
        init_database(&conn).expect("second initialization should be a no-op");

        let applied: i64 = conn
            .query_row("SELECT COUNT(*) FROM _migrations", [], |row| row.get(0))
            .expect("migration ledger should exist");
        assert_eq!(applied, 5);

        let capability_count: i64 = conn
            .query_row("SELECT COUNT(*) FROM capabilities", [], |row| row.get(0))
            .expect("seeded capabilities should remain queryable");
        assert_eq!(capability_count, 26);
    }

    #[test]
    fn financial_minor_columns_are_integer_authority_columns() {
        let conn = Connection::open_in_memory().expect("open in-memory database");
        init_database(&conn).expect("migrations should succeed");

        for (table, column) in [
            ("sales", "subtotal_minor"),
            ("sales", "discount_amount_minor"),
            ("sales", "tax_amount_minor"),
            ("sales", "total_minor"),
            ("sale_items", "unit_price_minor"),
            ("sale_items", "line_total_minor"),
            ("sale_payments", "amount_minor"),
            ("shifts", "opening_balance_minor"),
            ("shifts", "closing_balance_minor"),
            ("purchase_orders", "total_cost_minor"),
            ("purchase_order_items", "unit_cost_minor"),
            ("cash_movements", "amount_minor"),
            ("debt_ledger", "amount_minor"),
        ] {
            let declared_type: String = conn
                .query_row(
                    "SELECT type FROM pragma_table_info(?1) WHERE name = ?2",
                    rusqlite::params![table, column],
                    |row| row.get(0),
                )
                .expect("financial minor column should exist");
            assert_eq!(declared_type, "INTEGER", "{table}.{column} must be INTEGER");
        }
    }

    #[test]
    fn failed_transaction_rolls_back_all_changes() {
        let mut conn = Connection::open_in_memory().expect("open in-memory database");
        conn.execute_batch(
            "CREATE TABLE rollback_probe (id INTEGER PRIMARY KEY, value TEXT NOT NULL UNIQUE);",
        )
        .expect("setup should succeed");

        {
            let tx = conn.transaction().expect("transaction should start");
            tx.execute(
                "INSERT INTO rollback_probe(id, value) VALUES (1, 'first')",
                [],
            )
            .expect("first insert should succeed");
            let duplicate = tx.execute(
                "INSERT INTO rollback_probe(id, value) VALUES (2, 'first')",
                [],
            );
            assert!(duplicate.is_err(), "duplicate insert must fail");
        }

        let count: i64 = conn
            .query_row("SELECT COUNT(*) FROM rollback_probe", [], |row| row.get(0))
            .expect("rollback probe should remain queryable");
        assert_eq!(count, 0);
    }
}
