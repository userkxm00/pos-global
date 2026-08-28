use rusqlite::{Connection, Result, ToSql};
use std::path::Path;
use std::sync::Mutex;

pub struct DbState(pub Mutex<Connection>);

pub const MIGRATIONS: &[(&str, &str)] = &[
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
    (
        "006_quantity_precision_hardening",
        include_str!("migrations/006_quantity_precision_hardening.sql"),
    ),
    (
        "007_remove_redundant_inventory_index",
        include_str!("migrations/007_remove_redundant_inventory_index.sql"),
    ),
    (
        "008_inventory_and_schema_hardening",
        include_str!("migrations/008_inventory_and_schema_hardening.sql"),
    ),
    (
        "009_remove_redundant_product_barcode_index",
        include_str!("migrations/009_remove_redundant_product_barcode_index.sql"),
    ),
    (
        "010_registers",
        include_str!("migrations/010_registers.sql"),
    ),
    (
        "011_categories_brands_manufacturers",
        include_str!("migrations/011_categories_brands_manufacturers.sql"),
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

/// Safely escapes special LIKE wildcard characters (`%`, `_`, `\`) with a backslash escape.
pub fn escape_like_pattern(input: &str) -> String {
    let mut escaped = String::with_capacity(input.len());
    for c in input.chars() {
        if c == '%' || c == '_' || c == '\\' {
            escaped.push('\\');
        }
        escaped.push(c);
    }
    escaped
}

/// Validates website URL syntax conservatively: <= 2048 chars, no spaces, valid host part with dot.
pub fn validate_url_syntax(url: Option<&str>) -> Result<Option<String>, &'static str> {
    match url.map(str::trim).filter(|s| !s.is_empty()) {
        None => Ok(None),
        Some(s) => {
            if s.chars().count() > 2048 {
                return Err("Website URL exceeds maximum length of 2048 characters");
            }
            if s.contains(char::is_whitespace) {
                return Err("Website URL cannot contain whitespace");
            }
            let host_part = if let Some(stripped) = s.strip_prefix("https://") {
                stripped
            } else if let Some(stripped) = s.strip_prefix("http://") {
                stripped
            } else {
                s
            };
            let host = host_part.split(['/', '?', '#', ':']).next().unwrap_or("");
            let labels: Vec<&str> = host.split('.').collect();
            if labels.len() < 2 || labels.iter().any(|l| l.is_empty()) {
                return Err("Website URL must be a valid web address or domain");
            }
            Ok(Some(s.to_string()))
        }
    }
}

/// Appends a query search clause for name and description with escaped LIKE wildcards.
pub fn append_name_or_description_search(
    sql: &mut String,
    params_vec: &mut Vec<Box<dyn ToSql>>,
    query: Option<&str>,
) {
    if let Some(q) = query {
        let trimmed = q.trim();
        if !trimmed.is_empty() {
            sql.push_str(" AND (name LIKE ? ESCAPE '\\' OR description LIKE ? ESCAPE '\\')");
            let pattern = format!("%{}%", escape_like_pattern(trimmed));
            params_vec.push(Box::new(pattern.clone()));
            params_vec.push(Box::new(pattern));
        }
    }
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
        assert_eq!(applied, 11);

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
            "registers",
            "categories",
            "brands",
            "manufacturers",
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
        assert_eq!(applied, 11);

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
    fn quantity_precision_columns_are_integer_authority_columns() {
        let conn = Connection::open_in_memory().expect("open in-memory database");
        init_database(&conn).expect("migrations should succeed");

        for (table, column) in [
            ("inventory", "quantity_milli"),
            ("sale_items", "quantity_milli"),
            ("stock_movements", "quantity_delta_milli"),
            ("stock_movements", "quantity_before_milli"),
            ("stock_movements", "quantity_after_milli"),
        ] {
            let declared_type: String = conn
                .query_row(
                    "SELECT type FROM pragma_table_info(?1) WHERE name = ?2",
                    rusqlite::params![table, column],
                    |row| row.get(0),
                )
                .expect("quantity precision column should exist");
            assert_eq!(declared_type, "INTEGER", "{table}.{column} must be INTEGER");
        }

        let redundant_index_count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master
                 WHERE type = 'index' AND name = 'idx_inventory_branch_product_quantity_milli'",
                [],
                |row| row.get(0),
            )
            .expect("index metadata query should succeed");
        assert_eq!(
            redundant_index_count, 0,
            "redundant quantity index must be removed"
        );

        let redundant_barcode_index_count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master
                 WHERE type = 'index' AND name = 'idx_products_barcode'",
                [],
                |row| row.get(0),
            )
            .expect("barcode index metadata query should succeed");
        assert_eq!(
            redundant_barcode_index_count, 0,
            "redundant barcode index must be removed"
        );
    }

    #[test]
    fn inventory_identity_requires_single_non_variant_row() {
        let conn = Connection::open_in_memory().expect("open in-memory database");
        init_database(&conn).expect("migrations should succeed");

        let partial_unique_index_count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master
                 WHERE type = 'index'
                   AND name = 'ux_inventory_branch_product_no_variant'",
                [],
                |row| row.get(0),
            )
            .expect("inventory identity index query should succeed");
        assert_eq!(partial_unique_index_count, 1);

        let variant_unique_index_count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master
                 WHERE type = 'index'
                   AND name = 'ux_inventory_branch_product_variant'",
                [],
                |row| row.get(0),
            )
            .expect("variant inventory identity index query should succeed");
        assert_eq!(variant_unique_index_count, 1);
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

    #[test]
    fn test_escape_like_pattern() {
        assert_eq!(super::escape_like_pattern("100% pure"), "100\\% pure");
        assert_eq!(super::escape_like_pattern("item_one"), "item\\_one");
        assert_eq!(super::escape_like_pattern("path\\to"), "path\\\\to");
        assert_eq!(super::escape_like_pattern("normal"), "normal");
    }

    #[test]
    fn test_validate_url_syntax() {
        assert!(super::validate_url_syntax(None).unwrap().is_none());
        assert_eq!(
            super::validate_url_syntax(Some("https://example.com")).unwrap(),
            Some("https://example.com".to_string())
        );
        assert_eq!(
            super::validate_url_syntax(Some("brand.co.uk/store")).unwrap(),
            Some("brand.co.uk/store".to_string())
        );
        assert!(super::validate_url_syntax(Some("http://")).is_err());
        assert!(super::validate_url_syntax(Some("https://")).is_err());
        assert!(super::validate_url_syntax(Some(".invalid")).is_err());
        assert!(super::validate_url_syntax(Some("invalid.")).is_err());
        assert!(super::validate_url_syntax(Some("https://invalid..com")).is_err());
        assert!(super::validate_url_syntax(Some("https://invalid .com")).is_err());
    }
}
