// Focused Migration 020 and schema integrity test suite for F2.11
// Covers ADR-0013: Migration 020 fresh application, upgrade from 019,
// composite foreign keys, partial unique index coverage, non-negative checks,
// and stock_movements immutability triggers.

use crate::tests::test_helpers::{
    apply_migrations_up_to, create_test_org_and_branch, setup_test_db, setup_test_db_up_to,
};
use rusqlite::{params, Connection};

fn create_second_branch(conn: &Connection, org_id: &str) -> String {
    let branch = crate::branch::create_branch(
        conn,
        crate::branch::CreateBranchInput {
            organization_id: org_id.to_string(),
            name: "Secondary Branch".to_string(),
            address: Some("456 Second Ave".to_string()),
            currency: Some("USD".to_string()),
            is_active: Some(true),
        },
    )
    .expect("second branch created");
    branch.id
}

fn seed_catalog_fixture(conn: &Connection) -> (String, String) {
    let product_id = "prod_test_f211";
    let variant_id = "var_test_f211";

    conn.execute(
        "INSERT INTO products (id, name, base_price, is_active)
         VALUES (?1, 'F2.11 Test Product', 10.0, 1)",
        [product_id],
    )
    .expect("product created");

    conn.execute(
        "INSERT INTO product_variants (id, product_id, sku, is_active)
         VALUES (?1, ?2, 'SKU-F211-01', 1)",
        [variant_id, product_id],
    )
    .expect("variant created");

    (product_id.to_string(), variant_id.to_string())
}

#[test]
fn test_migration_020_fresh_application_and_idempotency() {
    let conn = Connection::open_in_memory().expect("in-memory db");

    // Apply up to 020
    apply_migrations_up_to(&conn, "020_stock_ledger_spatial");

    // Verify migration 020 is recorded
    let count: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM _migrations WHERE name = '020_stock_ledger_spatial'",
            [],
            |row| row.get(0),
        )
        .expect("query migration count");
    assert_eq!(count, 1);

    // Verify location_inventory table exists
    let table_exists: bool = conn
        .query_row(
            "SELECT COUNT(*) > 0 FROM sqlite_master WHERE type = 'table' AND name = 'location_inventory'",
            [],
            |row| row.get(0),
        )
        .expect("table check");
    assert!(table_exists);

    // Verify columns on stock_movements
    for col in ["location_id", "bin_id", "batch_id", "serial_id"] {
        let col_exists: bool = conn
            .query_row(
                "SELECT COUNT(*) > 0 FROM pragma_table_info('stock_movements') WHERE name = ?1",
                [col],
                |row| row.get(0),
            )
            .expect("column check");
        assert!(col_exists, "stock_movements must contain column {col}");
    }

    // Verify columns on serial_numbers
    for col in ["location_id", "bin_id"] {
        let col_exists: bool = conn
            .query_row(
                "SELECT COUNT(*) > 0 FROM pragma_table_info('serial_numbers') WHERE name = ?1",
                [col],
                |row| row.get(0),
            )
            .expect("column check");
        assert!(col_exists, "serial_numbers must contain column {col}");
    }

    // Verify request_hash on idempotency_keys
    let hash_col_exists: bool = conn
        .query_row(
            "SELECT COUNT(*) > 0 FROM pragma_table_info('idempotency_keys') WHERE name = 'request_hash'",
            [],
            |row| row.get(0),
        )
        .expect("request_hash check");
    assert!(hash_col_exists, "idempotency_keys must contain request_hash column");

    // Verify re-running full init_database is idempotent
    crate::db::init_database(&conn).expect("init_database must be idempotent");
}

#[test]
fn test_database_composite_foreign_key_location_branch_mismatch_rejected() {
    let conn = setup_test_db();
    let (org_id, branch_a) = create_test_org_and_branch(&conn);
    let branch_b = create_second_branch(&conn, &org_id);
    let (product_id, _) = seed_catalog_fixture(&conn);

    // Create location in branch A
    conn.execute(
        "INSERT INTO locations (id, branch_id, name, code, is_active)
         VALUES ('loc_a', ?1, 'Location A', 'LOC-A', 1)",
        [&branch_a],
    )
    .expect("location created");

    // Attempt to insert location_inventory using loc_a with branch_b -> must fail FK
    let err = conn.execute(
        "INSERT INTO location_inventory (id, branch_id, location_id, product_id, quantity_milli)
         VALUES ('inv_1', ?1, 'loc_a', ?2, 1000)",
        params![branch_b, product_id],
    );
    assert!(err.is_err(), "mismatched (location_id, branch_id) must be rejected by foreign key");
}

#[test]
fn test_database_composite_foreign_key_bin_location_mismatch_rejected() {
    let conn = setup_test_db();
    let (_, branch_id) = create_test_org_and_branch(&conn);
    let (product_id, _) = seed_catalog_fixture(&conn);

    // Create two locations in the branch
    conn.execute(
        "INSERT INTO locations (id, branch_id, name, code, is_active)
         VALUES ('loc_1', ?1, 'Zone 1', 'Z1', 1),
                ('loc_2', ?1, 'Zone 2', 'Z2', 1)",
        [&branch_id],
    )
    .expect("locations created");

    // Create bin in loc_1
    conn.execute(
        "INSERT INTO bins (id, location_id, name, code, is_active)
         VALUES ('bin_z1', 'loc_1', 'Shelf 1', 'S1', 1)",
        [],
    )
    .expect("bin created");

    // 1. Bin with matching location -> accepted
    conn.execute(
        "INSERT INTO location_inventory (id, branch_id, location_id, bin_id, product_id, quantity_milli)
         VALUES ('inv_ok', ?1, 'loc_1', 'bin_z1', ?2, 1000)",
        params![branch_id, product_id],
    )
    .expect("matching bin/location must succeed");

    // 2. Bin with mismatched location (bin_z1 used with loc_2) -> must fail FK
    let err = conn.execute(
        "INSERT INTO location_inventory (id, branch_id, location_id, bin_id, product_id, quantity_milli)
         VALUES ('inv_bad', ?1, 'loc_2', 'bin_z1', ?2, 1000)",
        params![branch_id, product_id],
    );
    assert!(err.is_err(), "mismatched (bin_id, location_id) must be rejected by composite foreign key");
}

#[test]
fn test_location_inventory_non_negative_check_constraint() {
    let conn = setup_test_db();
    let (_, branch_id) = create_test_org_and_branch(&conn);
    let (product_id, _) = seed_catalog_fixture(&conn);

    conn.execute(
        "INSERT INTO locations (id, branch_id, name, code, is_active)
         VALUES ('loc_1', ?1, 'Zone 1', 'Z1', 1)",
        [&branch_id],
    )
    .expect("location created");

    // Zero quantity accepted
    conn.execute(
        "INSERT INTO location_inventory (id, branch_id, location_id, product_id, quantity_milli)
         VALUES ('inv_zero', ?1, 'loc_1', ?2, 0)",
        params![branch_id, product_id],
    )
    .expect("zero quantity accepted");

    // Negative quantity rejected
    let err = conn.execute(
        "INSERT INTO location_inventory (id, branch_id, location_id, product_id, quantity_milli)
         VALUES ('inv_neg', ?1, 'loc_1', ?2, -1)",
        params![branch_id, product_id],
    );
    assert!(err.is_err(), "negative quantity_milli must violate CHECK constraint");
}

#[test]
fn test_location_inventory_partial_unique_indexes_prevent_duplicates() {
    let conn = setup_test_db();
    let (_, branch_id) = create_test_org_and_branch(&conn);
    let (product_id, variant_id) = seed_catalog_fixture(&conn);

    conn.execute(
        "INSERT INTO locations (id, branch_id, name, code, is_active)
         VALUES ('loc_1', ?1, 'Zone 1', 'Z1', 1)",
        [&branch_id],
    )
    .expect("location created");

    conn.execute(
        "INSERT INTO bins (id, location_id, name, code, is_active)
         VALUES ('bin_1', 'loc_1', 'Shelf 1', 'S1', 1)",
        [],
    )
    .expect("bin created");

    conn.execute(
        "INSERT INTO product_batches (id, product_id, branch_id, batch_number, quantity_milli, expiry_date, status)
         VALUES ('batch_1', ?1, ?2, 'BATCH-001', 0, '2028-12-31', 'active')",
        params![product_id, branch_id],
    )
    .expect("batch created");

    // Case 1: no bin, no var, no batch
    conn.execute(
        "INSERT INTO location_inventory (id, branch_id, location_id, product_id, quantity_milli)
         VALUES ('c1_a', ?1, 'loc_1', ?2, 500)",
        params![branch_id, product_id],
    )
    .expect("c1_a insert");
    let err = conn.execute(
        "INSERT INTO location_inventory (id, branch_id, location_id, product_id, quantity_milli)
         VALUES ('c1_dup', ?1, 'loc_1', ?2, 600)",
        params![branch_id, product_id],
    );
    assert!(err.is_err(), "duplicate c1 (no bin, no var, no batch) must be rejected");

    // Case 2: bin, no var, no batch
    conn.execute(
        "INSERT INTO location_inventory (id, branch_id, location_id, bin_id, product_id, quantity_milli)
         VALUES ('c2_a', ?1, 'loc_1', 'bin_1', ?2, 500)",
        params![branch_id, product_id],
    )
    .expect("c2_a insert");
    let err = conn.execute(
        "INSERT INTO location_inventory (id, branch_id, location_id, bin_id, product_id, quantity_milli)
         VALUES ('c2_dup', ?1, 'loc_1', 'bin_1', ?2, 600)",
        params![branch_id, product_id],
    );
    assert!(err.is_err(), "duplicate c2 (bin, no var, no batch) must be rejected");

    // Case 3: no bin, var, no batch
    conn.execute(
        "INSERT INTO location_inventory (id, branch_id, location_id, product_id, variant_id, quantity_milli)
         VALUES ('c3_a', ?1, 'loc_1', ?2, ?3, 500)",
        params![branch_id, product_id, variant_id],
    )
    .expect("c3_a insert");
    let err = conn.execute(
        "INSERT INTO location_inventory (id, branch_id, location_id, product_id, variant_id, quantity_milli)
         VALUES ('c3_dup', ?1, 'loc_1', ?2, ?3, 600)",
        params![branch_id, product_id, variant_id],
    );
    assert!(err.is_err(), "duplicate c3 (no bin, var, no batch) must be rejected");

    // Case 8: bin, var, batch
    conn.execute(
        "INSERT INTO location_inventory (id, branch_id, location_id, bin_id, product_id, variant_id, batch_id, quantity_milli)
         VALUES ('c8_a', ?1, 'loc_1', 'bin_1', ?2, ?3, 'batch_1', 500)",
        params![branch_id, product_id, variant_id],
    )
    .expect("c8_a insert");
    let err = conn.execute(
        "INSERT INTO location_inventory (id, branch_id, location_id, bin_id, product_id, variant_id, batch_id, quantity_milli)
         VALUES ('c8_dup', ?1, 'loc_1', 'bin_1', ?2, ?3, 'batch_1', 600)",
        params![branch_id, product_id, variant_id],
    );
    assert!(err.is_err(), "duplicate c8 (bin, var, batch) must be rejected");
}

#[test]
fn test_stock_movements_immutability_triggers() {
    let conn = setup_test_db();
    let (_, branch_id) = create_test_org_and_branch(&conn);
    let (product_id, _) = seed_catalog_fixture(&conn);

    conn.execute(
        "INSERT INTO stock_movements (id, branch_id, product_id, quantity_delta, quantity_delta_milli, reason)
         VALUES ('mov_immut', ?1, ?2, 1.0, 1000, 'opening_balance')",
        params![branch_id, product_id],
    )
    .expect("stock movement inserted");

    // UPDATE must be aborted by trigger
    let err_update = conn.execute(
        "UPDATE stock_movements SET quantity_delta_milli = 2000 WHERE id = 'mov_immut'",
        [],
    );
    assert!(err_update.is_err(), "UPDATE on stock_movements must be aborted by trigger");
    let err_msg = err_update.unwrap_err().to_string();
    assert!(err_msg.contains("stock_movements records are immutable"), "expected trigger error message, got: {err_msg}");

    // DELETE must be aborted by trigger
    let err_delete = conn.execute(
        "DELETE FROM stock_movements WHERE id = 'mov_immut'",
        [],
    );
    assert!(err_delete.is_err(), "DELETE on stock_movements must be aborted by trigger");
    let del_msg = err_delete.unwrap_err().to_string();
    assert!(del_msg.contains("stock_movements records are immutable"), "expected trigger error message, got: {del_msg}");
}

#[test]
fn test_upgrade_from_019_to_020_preserves_legacy_state() {
    let conn = setup_test_db_up_to("019_locations_bins");
    let (org_id, branch_id) = create_test_org_and_branch(&conn);
    let (product_id, _) = seed_catalog_fixture(&conn);

    // Seed pre-020 aggregate inventory
    conn.execute(
        "INSERT INTO inventory (id, branch_id, product_id, quantity, quantity_milli)
         VALUES ('inv_legacy', ?1, ?2, 15.0, 15000)",
        params![branch_id, product_id],
    )
    .expect("legacy inventory seeded");

    // Seed pre-020 serial without coordinates
    conn.execute(
        "INSERT INTO serial_numbers (id, product_id, branch_id, serial_number, status)
         VALUES ('sn_legacy', ?1, ?2, 'SN-LEGACY-001', 'in_stock')",
        params![product_id, branch_id],
    )
    .expect("legacy serial seeded");

    // Apply migration 020
    apply_migrations_up_to(&conn, "020_stock_ledger_spatial");

    // 1. Verify legacy inventory remains intact
    let legacy_qty: i64 = conn
        .query_row(
            "SELECT quantity_milli FROM inventory WHERE id = 'inv_legacy'",
            [],
            |r| r.get(0),
        )
        .expect("query legacy qty");
    assert_eq!(legacy_qty, 15000);

    // 2. Verify legacy serial remains untouched with NULL coordinates
    let (sn_status, loc_id, bin_id): (String, Option<String>, Option<String>) = conn
        .query_row(
            "SELECT status, location_id, bin_id FROM serial_numbers WHERE id = 'sn_legacy'",
            [],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
        )
        .expect("query legacy serial");
    assert_eq!(sn_status, "in_stock");
    assert!(loc_id.is_none());
    assert!(bin_id.is_none());

    // 3. Verify location_inventory is completely empty (no backfill fabricated)
    let loc_inv_count: i64 = conn
        .query_row("SELECT COUNT(*) FROM location_inventory", [], |r| r.get(0))
        .expect("query loc inv count");
    assert_eq!(loc_inv_count, 0, "legacy upgrade must not fabricate location_inventory rows");
}
