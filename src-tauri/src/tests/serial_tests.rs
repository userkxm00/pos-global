// Comprehensive unit, integration, migration, and contract tests for F2.08 Serial / IMEI / Assets.
// ADR-0010: Flexible triple-identifier model, single IMEI, global NOCASE serial uniqueness, and branch tenancy.

use crate::commands::serial::{
    create_serial_instance_impl, get_serial_instance_impl, list_serial_instances_impl,
    lookup_serial_instance_impl, update_serial_status_impl,
};
use crate::product::{create_product, CreateProductInput};
use crate::serial::*;
use crate::tests::test_helpers::{
    apply_migrations_up_to, create_test_org_and_branch, create_test_user_hierarchy,
    create_test_user_with_creds, setup_test_db, setup_test_db_up_to,
};
use crate::user::session::create_local_session;
use crate::variant::{create_variant, CreateVariantInput};
use rusqlite::{params, Connection};

// =========================================================================
// TEST FIXTURES & HELPERS
// =========================================================================

// Valid 15-digit Luhn-verified test IMEIs:
const VALID_IMEI_1: &str = "864508041234565";
const VALID_IMEI_2: &str = "358721098765430";
const VALID_IMEI_3: &str = "490154203237518";
const VALID_IMEI_4: &str = "990000862471853";

fn make_test_product(conn: &Connection, name: &str, requires_serial: bool) -> String {
    let p = create_product(
        conn,
        CreateProductInput {
            name: name.to_string(),
            description: None,
            category_id: None,
            sku: None,
            barcode: None,
            product_type: Some("simple".to_string()),
            base_price_minor: 99900,
            cost_price_minor: Some(50000),
            unit_type: Some("piece".to_string()),
            requires_expiry: None,
            requires_serial: Some(requires_serial),
            warranty_months: None,
            custom_attributes: None,
        },
    )
    .expect("create test product");
    p.id
}

fn add_product_capability(conn: &Connection, product_id: &str, cap_code: &str) {
    conn.execute(
        "INSERT OR REPLACE INTO product_capabilities (product_id, capability_id, enabled)
         SELECT ?1, id, 1 FROM capabilities WHERE code = ?2",
        params![product_id, cap_code],
    )
    .expect("add product capability");
}

fn make_test_variant(conn: &Connection, product_id: &str, sku: &str) -> String {
    let v = create_variant(
        conn,
        CreateVariantInput {
            product_id: product_id.to_string(),
            sku: Some(sku.to_string()),
            barcode: None,
            price_override_minor: None,
            cost_price_minor: None,
            attribute_value_ids: vec![],
        },
    )
    .expect("create test variant");
    v.variant.id
}

// =========================================================================
// 1. MIGRATION 017 TESTS
// =========================================================================

#[test]
fn test_migration_017_fresh_database() {
    let conn = setup_test_db();

    // Verify serial_numbers table schema
    let col_count: i64 = conn
        .query_row(
            "SELECT count(*) FROM pragma_table_info('serial_numbers')
             WHERE name IN ('variant_id', 'imei', 'asset_tag', 'cost_price_minor', 'sold_in_sale_id', 'warranty_expires_at')",
            [],
            |r| r.get(0),
        )
        .expect("query pragma_table_info");
    assert_eq!(
        col_count, 6,
        "All F2.08 columns must exist in Migration 017"
    );

    // Verify partial unique indexes exist
    let idx_count: i64 = conn
        .query_row(
            "SELECT count(*) FROM sqlite_master WHERE type = 'index' AND name IN (
                'idx_serial_numbers_serial_active',
                'idx_serial_numbers_imei_active',
                'idx_serial_numbers_asset_tag_branch'
            )",
            [],
            |r| r.get(0),
        )
        .expect("query indexes");
    assert_eq!(idx_count, 3, "All 3 partial unique indexes must exist");
}

#[test]
fn test_migration_017_upgrade_preserves_legacy_data() {
    let conn = setup_test_db_up_to("016_batches_and_expiry");
    let (_, branch_id) = create_test_org_and_branch(&conn);
    let product_id = make_test_product(&conn, "Legacy Laptop", true);

    // Insert legacy record with sold_in_sale_id and warranty_expires_at
    conn.execute(
        "INSERT INTO serial_numbers (
            id, product_id, branch_id, serial_number, status,
            sold_in_sale_id, warranty_expires_at
        ) VALUES (
            'legacy-id-001', ?1, ?2, '  SN-LEGACY-001  ', 'sold',
            'sale-12345', '2028-12-31'
        )",
        params![product_id, branch_id],
    )
    .expect("insert legacy serial");

    // Apply Migration 017
    apply_migrations_up_to(&conn, "017_serial_imei_assets");

    // Verify legacy row preserved and trimmed
    let row: (
        String,
        String,
        String,
        String,
        Option<String>,
        Option<String>,
    ) = conn
        .query_row(
            "SELECT id, serial_number, status, sold_in_sale_id, warranty_expires_at, created_at
             FROM serial_numbers WHERE id = 'legacy-id-001'",
            [],
            |r| {
                Ok((
                    r.get(0)?,
                    r.get(1)?,
                    r.get(2)?,
                    r.get(3)?,
                    r.get(4)?,
                    r.get(5)?,
                ))
            },
        )
        .expect("query migrated row");

    assert_eq!(row.0, "legacy-id-001");
    assert_eq!(
        row.1, "SN-LEGACY-001",
        "Leading/trailing whitespace must be trimmed"
    );
    assert_eq!(row.2, "sold", "Legacy status must be preserved");
    assert_eq!(row.3, "sale-12345", "sold_in_sale_id must be preserved");
    assert_eq!(
        row.4.as_deref(),
        Some("2028-12-31"),
        "warranty_expires_at must be preserved"
    );
    assert!(!row.5.is_empty(), "created_at timestamp must be backfilled");
}

#[test]
fn test_migration_017_null_serial_compatibility() {
    let conn = setup_test_db();
    let (_, branch_id) = create_test_org_and_branch(&conn);
    let product_id = make_test_product(&conn, "Smartphone", true);

    // Two records with NULL serials but distinct IMEIs must both succeed (no NULL uniqueness collision)
    conn.execute(
        "INSERT INTO serial_numbers (product_id, branch_id, imei) VALUES (?1, ?2, ?3)",
        params![product_id, branch_id, VALID_IMEI_1],
    )
    .expect("insert imei-only 1");

    conn.execute(
        "INSERT INTO serial_numbers (product_id, branch_id, imei) VALUES (?1, ?2, ?3)",
        params![product_id, branch_id, VALID_IMEI_2],
    )
    .expect("insert imei-only 2");

    let count: i64 = conn
        .query_row(
            "SELECT count(*) FROM serial_numbers WHERE product_id = ?1 AND serial_number IS NULL",
            params![product_id],
            |r| r.get(0),
        )
        .expect("query count");
    assert_eq!(
        count, 2,
        "Multiple NULL serial numbers must coexist without index conflict"
    );
}

#[test]
fn test_migration_017_fails_closed_on_empty_whitespace_serial() {
    let conn = setup_test_db_up_to("016_batches_and_expiry");
    let (_, branch_id) = create_test_org_and_branch(&conn);
    let product_id = make_test_product(&conn, "Defective Product", true);

    // Legacy row with whitespace-only serial number
    conn.execute(
        "INSERT INTO serial_numbers (id, product_id, branch_id, serial_number, status)
         VALUES ('bad-id', ?1, ?2, '   ', 'in_stock')",
        params![product_id, branch_id],
    )
    .expect("insert whitespace serial");

    let m17_sql = include_str!("../db/migrations/017_serial_imei_assets.sql");
    let res = conn.execute_batch(m17_sql);
    assert!(
        res.is_err(),
        "Migration 017 must fail closed when whitespace-only serial exists"
    );
}

#[test]
fn test_migration_017_fails_closed_on_duplicate_nocase_serial() {
    let conn = setup_test_db_up_to("016_batches_and_expiry");
    let (_, branch_id) = create_test_org_and_branch(&conn);
    let product_id = make_test_product(&conn, "Gadget", true);

    // Legacy table allowed binary distinct duplicates 'SN-100' and 'sn-100'
    conn.execute(
        "INSERT INTO serial_numbers (id, product_id, branch_id, serial_number, status)
         VALUES ('id-1', ?1, ?2, 'SN-100', 'in_stock')",
        params![product_id, branch_id],
    )
    .expect("insert SN-100");

    conn.execute(
        "INSERT INTO serial_numbers (id, product_id, branch_id, serial_number, status)
         VALUES ('id-2', ?1, ?2, 'sn-100', 'in_stock')",
        params![product_id, branch_id],
    )
    .expect("insert sn-100");

    let m17_sql = include_str!("../db/migrations/017_serial_imei_assets.sql");
    let res = conn.execute_batch(m17_sql);
    assert!(
        res.is_err(),
        "Migration 017 must fail closed on case-insensitive duplicate serial numbers"
    );
}

#[test]
fn test_migration_017_fails_closed_on_invalid_status() {
    let conn = setup_test_db_up_to("016_batches_and_expiry");
    let (_, branch_id) = create_test_org_and_branch(&conn);
    let product_id = make_test_product(&conn, "Appliance", true);

    conn.execute(
        "INSERT INTO serial_numbers (id, product_id, branch_id, serial_number, status)
         VALUES ('id-inv', ?1, ?2, 'SN-VALID-01', 'unknown_status_legacy')",
        params![product_id, branch_id],
    )
    .expect("insert invalid status");

    let m17_sql = include_str!("../db/migrations/017_serial_imei_assets.sql");
    let res = conn.execute_batch(m17_sql);
    assert!(
        res.is_err(),
        "Migration 017 must fail closed on legacy row with invalid status"
    );
}

#[test]
fn test_migration_017_fails_closed_on_orphan_foreign_keys() {
    let conn = setup_test_db_up_to("016_batches_and_expiry");
    let (_, branch_id) = create_test_org_and_branch(&conn);

    // Disable pragma temporarily to simulate corrupted legacy DB with orphaned product_id
    conn.execute_batch("PRAGMA foreign_keys = OFF;").unwrap();
    conn.execute(
        "INSERT INTO serial_numbers (id, product_id, branch_id, serial_number, status)
         VALUES ('orphan-1', 'nonexistent-prod', ?1, 'SN-ORPHAN-01', 'in_stock')",
        params![branch_id],
    )
    .expect("insert orphan row");
    conn.execute_batch("PRAGMA foreign_keys = ON;").unwrap();

    let m17_sql = include_str!("../db/migrations/017_serial_imei_assets.sql");
    let res = conn.execute_batch(m17_sql);
    assert!(
        res.is_err(),
        "Migration 017 must fail closed on orphaned foreign keys"
    );
}

#[test]
fn test_migration_017_rollback_integrity() {
    let conn = setup_test_db_up_to("016_batches_and_expiry");
    let (_, branch_id) = create_test_org_and_branch(&conn);
    let product_id = make_test_product(&conn, "Tool", true);

    // Valid row + invalid duplicate row
    conn.execute(
        "INSERT INTO serial_numbers (id, product_id, branch_id, serial_number, status)
         VALUES ('id-good', ?1, ?2, 'SN-TOOL', 'in_stock')",
        params![product_id, branch_id],
    )
    .unwrap();
    conn.execute(
        "INSERT INTO serial_numbers (id, product_id, branch_id, serial_number, status)
         VALUES ('id-dup', ?1, ?2, 'sn-tool', 'in_stock')",
        params![product_id, branch_id],
    )
    .unwrap();

    let m17_sql = include_str!("../db/migrations/017_serial_imei_assets.sql");
    let _ = conn.execute_batch(m17_sql);

    // Verify original table still exists and unmodified
    let count: i64 = conn
        .query_row("SELECT count(*) FROM serial_numbers", [], |r| r.get(0))
        .expect("query serial_numbers");
    assert_eq!(
        count, 2,
        "Original serial_numbers table must remain intact on aborted migration"
    );

    // Verify no temporary table remains
    let temp_exists: bool = conn
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE name = 'serial_numbers_new')",
            [],
            |r| r.get(0),
        )
        .expect("check temp table");
    assert!(
        !temp_exists,
        "Partial table serial_numbers_new must not remain"
    );
}

// =========================================================================
// 2. IDENTIFIERS & VALIDATION TESTS
// =========================================================================

#[test]
fn test_serial_only_identifier() {
    let conn = setup_test_db();
    let (_, branch_id) = create_test_org_and_branch(&conn);
    let product_id = make_test_product(&conn, "Desktop PC", true);

    let instance = create_serial_instance(
        &conn,
        &CreateSerialInput {
            product_id,
            branch_id,
            variant_id: None,
            serial_number: Some("PC-2026-X99".into()),
            imei: None,
            asset_tag: None,
            cost_price_minor: Some(75000),
        },
    )
    .expect("create serial only");

    assert_eq!(instance.serial_number.as_deref(), Some("PC-2026-X99"));
    assert!(instance.imei.is_none());
    assert!(instance.asset_tag.is_none());
    assert_eq!(instance.status, SerialStatus::InStock);
}

#[test]
fn test_imei_only_identifier() {
    let conn = setup_test_db();
    let (_, branch_id) = create_test_org_and_branch(&conn);
    let product_id = make_test_product(&conn, "5G Router", true);

    let instance = create_serial_instance(
        &conn,
        &CreateSerialInput {
            product_id,
            branch_id,
            variant_id: None,
            serial_number: None,
            imei: Some(VALID_IMEI_1.into()),
            asset_tag: None,
            cost_price_minor: None,
        },
    )
    .expect("create imei only");

    assert!(instance.serial_number.is_none());
    assert_eq!(instance.imei.as_deref(), Some(VALID_IMEI_1));
    assert!(instance.asset_tag.is_none());
}

#[test]
fn test_asset_tag_only_identifier() {
    let conn = setup_test_db();
    let (_, branch_id) = create_test_org_and_branch(&conn);
    let product_id = make_test_product(&conn, "Hydraulic Jack", true);

    let instance = create_serial_instance(
        &conn,
        &CreateSerialInput {
            product_id,
            branch_id,
            variant_id: None,
            serial_number: None,
            imei: None,
            asset_tag: Some("ASSET-NYC-0042".into()),
            cost_price_minor: None,
        },
    )
    .expect("create asset only");

    assert!(instance.serial_number.is_none());
    assert!(instance.imei.is_none());
    assert_eq!(instance.asset_tag.as_deref(), Some("ASSET-NYC-0042"));
}

#[test]
fn test_all_identifier_combinations() {
    let conn = setup_test_db();
    let (_, branch_id) = create_test_org_and_branch(&conn);
    let product_id = make_test_product(&conn, "Flagship Phone", true);

    // Serial + IMEI
    let inst_si = create_serial_instance(
        &conn,
        &CreateSerialInput {
            product_id: product_id.clone(),
            branch_id: branch_id.clone(),
            variant_id: None,
            serial_number: Some("SER-COMBO-1".into()),
            imei: Some(VALID_IMEI_1.into()),
            asset_tag: None,
            cost_price_minor: None,
        },
    );
    assert!(inst_si.is_ok());

    // Serial + Asset
    let inst_sa = create_serial_instance(
        &conn,
        &CreateSerialInput {
            product_id: product_id.clone(),
            branch_id: branch_id.clone(),
            variant_id: None,
            serial_number: Some("SER-COMBO-2".into()),
            imei: None,
            asset_tag: Some("TAG-COMBO-2".into()),
            cost_price_minor: None,
        },
    );
    assert!(inst_sa.is_ok());

    // IMEI + Asset
    let inst_ia = create_serial_instance(
        &conn,
        &CreateSerialInput {
            product_id: product_id.clone(),
            branch_id: branch_id.clone(),
            variant_id: None,
            serial_number: None,
            imei: Some(VALID_IMEI_2.into()),
            asset_tag: Some("TAG-COMBO-3".into()),
            cost_price_minor: None,
        },
    );
    assert!(inst_ia.is_ok());

    // All three
    let inst_all = create_serial_instance(
        &conn,
        &CreateSerialInput {
            product_id,
            branch_id,
            variant_id: None,
            serial_number: Some("SER-ALL-1".into()),
            imei: Some(VALID_IMEI_3.into()),
            asset_tag: Some("TAG-ALL-1".into()),
            cost_price_minor: None,
        },
    );
    assert!(inst_all.is_ok());
}

#[test]
fn test_zero_identifiers_rejected() {
    let conn = setup_test_db();
    let (_, branch_id) = create_test_org_and_branch(&conn);
    let product_id = make_test_product(&conn, "Any Product", true);

    let err = create_serial_instance(
        &conn,
        &CreateSerialInput {
            product_id,
            branch_id,
            variant_id: None,
            serial_number: None,
            imei: None,
            asset_tag: None,
            cost_price_minor: None,
        },
    )
    .unwrap_err();

    assert!(matches!(err, SerialError::Validation(_)));
}

#[test]
fn test_serial_global_uniqueness() {
    let conn = setup_test_db();
    let (org_id, branch_1) = create_test_org_and_branch(&conn);
    let branch_2 = crate::branch::create_branch(
        &conn,
        crate::branch::CreateBranchInput {
            organization_id: org_id,
            name: "Branch 2".into(),
            address: None,
            currency: None,
            is_active: Some(true),
        },
    )
    .unwrap()
    .id;

    let p1 = make_test_product(&conn, "Product 1", true);
    let p2 = make_test_product(&conn, "Product 2", true);

    // Serial registered at branch 1
    create_serial_instance(
        &conn,
        &CreateSerialInput {
            product_id: p1,
            branch_id: branch_1,
            variant_id: None,
            serial_number: Some("GLOBAL-SN-12345".into()),
            imei: None,
            asset_tag: None,
            cost_price_minor: None,
        },
    )
    .expect("first serial");

    // Same serial at branch 2 with different product MUST be rejected globally
    let err = create_serial_instance(
        &conn,
        &CreateSerialInput {
            product_id: p2,
            branch_id: branch_2,
            variant_id: None,
            serial_number: Some("GLOBAL-SN-12345".into()),
            imei: None,
            asset_tag: None,
            cost_price_minor: None,
        },
    )
    .unwrap_err();

    assert!(matches!(err, SerialError::DuplicateSerial(_)));
}

#[test]
fn test_serial_case_insensitive_uniqueness() {
    let conn = setup_test_db();
    let (_, branch_id) = create_test_org_and_branch(&conn);
    let product_id = make_test_product(&conn, "Tablet", true);

    create_serial_instance(
        &conn,
        &CreateSerialInput {
            product_id: product_id.clone(),
            branch_id: branch_id.clone(),
            variant_id: None,
            serial_number: Some("TAB-ABC-999".into()),
            imei: None,
            asset_tag: None,
            cost_price_minor: None,
        },
    )
    .expect("first serial");

    let err = create_serial_instance(
        &conn,
        &CreateSerialInput {
            product_id,
            branch_id,
            variant_id: None,
            serial_number: Some("tab-abc-999".into()),
            imei: None,
            asset_tag: None,
            cost_price_minor: None,
        },
    )
    .unwrap_err();

    assert!(matches!(err, SerialError::DuplicateSerial(_)));
}

#[test]
fn test_imei_global_uniqueness() {
    let conn = setup_test_db();
    let (_, branch_id) = create_test_org_and_branch(&conn);
    let product_id = make_test_product(&conn, "Cellular Device", true);

    create_serial_instance(
        &conn,
        &CreateSerialInput {
            product_id: product_id.clone(),
            branch_id: branch_id.clone(),
            variant_id: None,
            serial_number: None,
            imei: Some(VALID_IMEI_1.into()),
            asset_tag: None,
            cost_price_minor: None,
        },
    )
    .expect("first imei");

    let err = create_serial_instance(
        &conn,
        &CreateSerialInput {
            product_id,
            branch_id,
            variant_id: None,
            serial_number: None,
            imei: Some(VALID_IMEI_1.into()),
            asset_tag: None,
            cost_price_minor: None,
        },
    )
    .unwrap_err();

    assert!(matches!(err, SerialError::DuplicateImei(_)));
}

#[test]
fn test_asset_tag_branch_scoped_uniqueness() {
    let conn = setup_test_db();
    let (org_id, branch_1) = create_test_org_and_branch(&conn);
    let branch_2 = crate::branch::create_branch(
        &conn,
        crate::branch::CreateBranchInput {
            organization_id: org_id,
            name: "Branch 2".into(),
            address: None,
            currency: None,
            is_active: Some(true),
        },
    )
    .unwrap()
    .id;

    let p1 = make_test_product(&conn, "Projector", true);
    let p2 = make_test_product(&conn, "Projector 2", true);

    // Tag in Branch 1
    create_serial_instance(
        &conn,
        &CreateSerialInput {
            product_id: p1,
            branch_id: branch_1.clone(),
            variant_id: None,
            serial_number: None,
            imei: None,
            asset_tag: Some("TAG-PRJ-01".into()),
            cost_price_minor: None,
        },
    )
    .expect("tag in branch 1");

    // Same tag in Branch 2 MUST SUCCEED (branch-scoped)
    let res_b2 = create_serial_instance(
        &conn,
        &CreateSerialInput {
            product_id: p2.clone(),
            branch_id: branch_2,
            variant_id: None,
            serial_number: None,
            imei: None,
            asset_tag: Some("TAG-PRJ-01".into()),
            cost_price_minor: None,
        },
    );
    assert!(
        res_b2.is_ok(),
        "Same asset tag in different branch must be allowed"
    );

    // Duplicate tag in SAME Branch 1 (case-insensitive) MUST BE REJECTED
    let err_dup = create_serial_instance(
        &conn,
        &CreateSerialInput {
            product_id: p2,
            branch_id: branch_1,
            variant_id: None,
            serial_number: None,
            imei: None,
            asset_tag: Some("tag-prj-01".into()),
            cost_price_minor: None,
        },
    )
    .unwrap_err();
    assert!(matches!(err_dup, SerialError::DuplicateAssetTag(_)));
}

#[test]
fn test_identifier_trimming_and_max_length() {
    let conn = setup_test_db();
    let (_, branch_id) = create_test_org_and_branch(&conn);
    let product_id = make_test_product(&conn, "Scanner", true);

    // Trimming verified
    let inst = create_serial_instance(
        &conn,
        &CreateSerialInput {
            product_id: product_id.clone(),
            branch_id: branch_id.clone(),
            variant_id: None,
            serial_number: Some("   SCAN-999-XYZ   ".into()),
            imei: None,
            asset_tag: Some("   TAG-SCAN-01   ".into()),
            cost_price_minor: None,
        },
    )
    .expect("trimmed creation");
    assert_eq!(inst.serial_number.as_deref(), Some("SCAN-999-XYZ"));
    assert_eq!(inst.asset_tag.as_deref(), Some("TAG-SCAN-01"));

    // Max length exceeded (> 100 chars)
    let long_serial = "S".repeat(101);
    let err = create_serial_instance(
        &conn,
        &CreateSerialInput {
            product_id,
            branch_id,
            variant_id: None,
            serial_number: Some(long_serial),
            imei: None,
            asset_tag: None,
            cost_price_minor: None,
        },
    )
    .unwrap_err();
    assert!(matches!(err, SerialError::Validation(_)));
}

#[test]
fn test_unicode_character_count_serial_and_asset_tag() {
    let conn = setup_test_db();
    let (_, branch_id) = create_test_org_and_branch(&conn);
    let product_id = make_test_product(&conn, "Unicode Product", true);

    // 1. ASCII 100 characters accepted
    let ascii_100 = "A".repeat(100);
    assert_eq!(ascii_100.chars().count(), 100);
    assert_eq!(ascii_100.len(), 100);
    let inst_ascii_100 = create_serial_instance(
        &conn,
        &CreateSerialInput {
            product_id: product_id.clone(),
            branch_id: branch_id.clone(),
            variant_id: None,
            serial_number: Some(ascii_100.clone()),
            imei: None,
            asset_tag: Some(ascii_100),
            cost_price_minor: None,
        },
    );
    assert!(
        inst_ascii_100.is_ok(),
        "ASCII exactly 100 chars must be accepted"
    );

    // 2. ASCII 101 characters rejected
    let ascii_101 = "A".repeat(101);
    assert_eq!(ascii_101.chars().count(), 101);
    let err_ascii_101 = create_serial_instance(
        &conn,
        &CreateSerialInput {
            product_id: product_id.clone(),
            branch_id: branch_id.clone(),
            variant_id: None,
            serial_number: Some(ascii_101),
            imei: None,
            asset_tag: None,
            cost_price_minor: None,
        },
    );
    assert!(err_ascii_101.is_err(), "ASCII 101 chars must be rejected");

    // 3. Multibyte Unicode below 100 characters accepted (e.g. 50 characters of 'ñ', which is 100 UTF-8 bytes)
    let unicode_50 = "ñ".repeat(50);
    assert_eq!(unicode_50.chars().count(), 50);
    assert_eq!(unicode_50.len(), 100);
    let inst_uni_50 = create_serial_instance(
        &conn,
        &CreateSerialInput {
            product_id: product_id.clone(),
            branch_id: branch_id.clone(),
            variant_id: None,
            serial_number: Some(unicode_50.clone()),
            imei: None,
            asset_tag: Some(unicode_50),
            cost_price_minor: None,
        },
    );
    assert!(
        inst_uni_50.is_ok(),
        "Multibyte 50 chars (100 bytes) must be accepted"
    );

    // 4. Multibyte Unicode exactly 100 characters accepted (e.g. 100 characters of 'ñ', which is 200 UTF-8 bytes)
    let unicode_100 = "ñ".repeat(100);
    assert_eq!(unicode_100.chars().count(), 100);
    assert_eq!(unicode_100.len(), 200);
    let inst_uni_100 = create_serial_instance(
        &conn,
        &CreateSerialInput {
            product_id: product_id.clone(),
            branch_id: branch_id.clone(),
            variant_id: None,
            serial_number: Some(unicode_100.clone()),
            imei: None,
            asset_tag: Some(unicode_100),
            cost_price_minor: None,
        },
    );
    assert!(
        inst_uni_100.is_ok(),
        "Multibyte 100 chars (200 bytes) must be accepted"
    );

    // 5. Multibyte Unicode 101 characters rejected (101 characters of 'ñ', 202 UTF-8 bytes)
    let unicode_101 = "ñ".repeat(101);
    assert_eq!(unicode_101.chars().count(), 101);
    let err_uni_101 = create_serial_instance(
        &conn,
        &CreateSerialInput {
            product_id: product_id.clone(),
            branch_id: branch_id.clone(),
            variant_id: None,
            serial_number: Some(unicode_101.clone()),
            imei: None,
            asset_tag: None,
            cost_price_minor: None,
        },
    );
    assert!(err_uni_101.is_err(), "Multibyte 101 chars must be rejected");

    let err_tag_101 = create_serial_instance(
        &conn,
        &CreateSerialInput {
            product_id,
            branch_id,
            variant_id: None,
            serial_number: None,
            imei: None,
            asset_tag: Some(unicode_101),
            cost_price_minor: None,
        },
    );
    assert!(
        err_tag_101.is_err(),
        "Asset tag multibyte 101 chars must be rejected"
    );
}

// =========================================================================
// 3. IMEI VALIDATION TESTS
// =========================================================================

#[test]
fn test_imei_luhn_checksum_validation() {
    assert!(validate_luhn_checksum(VALID_IMEI_1));
    assert!(validate_luhn_checksum(VALID_IMEI_2));
    assert!(validate_luhn_checksum(VALID_IMEI_3));
    assert!(validate_luhn_checksum(VALID_IMEI_4));

    // Alter last digit to produce invalid checksum
    assert!(!validate_luhn_checksum("864508041234560"));
    assert!(!validate_luhn_checksum("864508041234569"));
}

#[test]
fn test_imei_invalid_length_and_non_digits() {
    // 14 digits
    assert!(validate_imei("86450804123456").is_err());
    // 16 digits
    assert!(validate_imei("8645080412345650").is_err());
    // Letters
    assert!(validate_imei("86450804123456A").is_err());
    // Special chars
    assert!(validate_imei("86450804123-456").is_err());
}

// =========================================================================
// 4. VARIANT INTEGRITY TESTS
// =========================================================================

#[test]
fn test_variant_association_valid() {
    let conn = setup_test_db();
    let (_, branch_id) = create_test_org_and_branch(&conn);
    let product_id = make_test_product(&conn, "Smartphone Matrix", true);
    let variant_id = make_test_variant(&conn, &product_id, "PHONE-256GB-BLACK");

    let inst = create_serial_instance(
        &conn,
        &CreateSerialInput {
            product_id,
            branch_id,
            variant_id: Some(variant_id.clone()),
            serial_number: Some("VAR-SN-001".into()),
            imei: None,
            asset_tag: None,
            cost_price_minor: None,
        },
    )
    .expect("create with valid variant");

    assert_eq!(inst.variant_id.as_deref(), Some(variant_id.as_str()));
}

#[test]
fn test_variant_association_wrong_product_and_missing() {
    let conn = setup_test_db();
    let (_, branch_id) = create_test_org_and_branch(&conn);
    let p1 = make_test_product(&conn, "Phone A", true);
    let p2 = make_test_product(&conn, "Phone B", true);

    let var1 = make_test_variant(&conn, &p1, "VAR-A-1");

    // Attempt to associate var1 with p2 (cross-product variant violation)
    let err_cross = create_serial_instance(
        &conn,
        &CreateSerialInput {
            product_id: p2.clone(),
            branch_id: branch_id.clone(),
            variant_id: Some(var1),
            serial_number: Some("CROSS-SN".into()),
            imei: None,
            asset_tag: None,
            cost_price_minor: None,
        },
    )
    .unwrap_err();
    assert!(matches!(err_cross, SerialError::InvalidVariant(_)));

    // Non-existent variant
    let err_missing = create_serial_instance(
        &conn,
        &CreateSerialInput {
            product_id: p2,
            branch_id,
            variant_id: Some("nonexistent-variant".into()),
            serial_number: Some("MISSING-SN".into()),
            imei: None,
            asset_tag: None,
            cost_price_minor: None,
        },
    )
    .unwrap_err();
    assert!(matches!(err_missing, SerialError::InvalidVariant(_)));
}

#[test]
fn test_variant_association_deleted_rejected() {
    let conn = setup_test_db();
    let (_, branch_id) = create_test_org_and_branch(&conn);
    let product_id = make_test_product(&conn, "Camera", true);
    let variant_id = make_test_variant(&conn, &product_id, "CAM-BODY-ONLY");

    // Soft-delete the variant
    conn.execute(
        "UPDATE product_variants SET deleted_at = datetime('now'), is_active = 0 WHERE id = ?1",
        params![variant_id],
    )
    .expect("soft delete variant");

    let err = create_serial_instance(
        &conn,
        &CreateSerialInput {
            product_id,
            branch_id,
            variant_id: Some(variant_id),
            serial_number: Some("CAM-SN-DEL".into()),
            imei: None,
            asset_tag: None,
            cost_price_minor: None,
        },
    )
    .unwrap_err();
    assert!(matches!(err, SerialError::InvalidVariant(_)));
}

// =========================================================================
// 5. CAPABILITY & COMPOSABILITY TESTS
// =========================================================================

#[test]
fn test_capability_requires_serial_flag() {
    let conn = setup_test_db();
    let product_id = make_test_product(&conn, "Flag Tracked", true);
    assert!(is_serial_tracked(&conn, &product_id).unwrap());
}

#[test]
fn test_capability_serial_code() {
    let conn = setup_test_db();
    let product_id = make_test_product(&conn, "Cap Serial Tracked", false);
    assert!(!is_serial_tracked(&conn, &product_id).unwrap());

    add_product_capability(&conn, &product_id, "SERIAL");
    assert!(is_serial_tracked(&conn, &product_id).unwrap());
}

#[test]
fn test_capability_imei_code() {
    let conn = setup_test_db();
    let product_id = make_test_product(&conn, "Cap IMEI Tracked", false);
    assert!(!is_serial_tracked(&conn, &product_id).unwrap());

    add_product_capability(&conn, &product_id, "IMEI");
    assert!(is_serial_tracked(&conn, &product_id).unwrap());
}

#[test]
fn test_non_serialized_product_rejected() {
    let conn = setup_test_db();
    let (_, branch_id) = create_test_org_and_branch(&conn);
    let product_id = make_test_product(&conn, "Generic Candy", false);

    let err = create_serial_instance(
        &conn,
        &CreateSerialInput {
            product_id,
            branch_id,
            variant_id: None,
            serial_number: Some("CANDY-001".into()),
            imei: None,
            asset_tag: None,
            cost_price_minor: None,
        },
    )
    .unwrap_err();

    assert!(matches!(err, SerialError::ProductNotSerialized(_)));
}

#[test]
fn test_composable_capabilities_weighted_and_serial() {
    let conn = setup_test_db();
    let (_, branch_id) = create_test_org_and_branch(&conn);
    let product_id = make_test_product(&conn, "Pre-Packaged Precious Metal Bar", true);

    // Composability: add WEIGHT capability to serial-tracked item
    add_product_capability(&conn, &product_id, "WEIGHT");
    assert!(is_serial_tracked(&conn, &product_id).unwrap());

    let inst = create_serial_instance(
        &conn,
        &CreateSerialInput {
            product_id,
            branch_id,
            variant_id: None,
            serial_number: Some("GOLD-BAR-9999-01".into()),
            imei: None,
            asset_tag: None,
            cost_price_minor: Some(150000),
        },
    );
    assert!(
        inst.is_ok(),
        "Weighted + serial coexistence must be permitted"
    );
}

#[test]
fn test_composable_capabilities_batch_and_serial() {
    let conn = setup_test_db();
    let (_, branch_id) = create_test_org_and_branch(&conn);
    let product_id = make_test_product(&conn, "Medical Implant", true);

    // Composability: add BATCH capability
    add_product_capability(&conn, &product_id, "BATCH");
    assert!(is_serial_tracked(&conn, &product_id).unwrap());

    let inst = create_serial_instance(
        &conn,
        &CreateSerialInput {
            product_id,
            branch_id,
            variant_id: None,
            serial_number: Some("MED-IMPLANT-0042".into()),
            imei: None,
            asset_tag: None,
            cost_price_minor: None,
        },
    );
    assert!(inst.is_ok(), "Batch + serial coexistence must be permitted");
}

// =========================================================================
// 6. LIFECYCLE & STATE TRANSITIONS
// =========================================================================

#[test]
fn test_lifecycle_status_transitions() {
    let conn = setup_test_db();
    let (_, branch_id) = create_test_org_and_branch(&conn);
    let product_id = make_test_product(&conn, "Motor Unit", true);

    let inst = create_serial_instance(
        &conn,
        &CreateSerialInput {
            product_id,
            branch_id: branch_id.clone(),
            variant_id: None,
            serial_number: Some("MOTOR-01".into()),
            imei: None,
            asset_tag: None,
            cost_price_minor: None,
        },
    )
    .expect("create");

    // InStock -> Reserved
    let s_reserved = update_serial_status(
        &conn,
        &UpdateSerialStatusInput {
            id: inst.id.clone(),
            branch_id: branch_id.clone(),
            status: SerialStatus::Reserved,
        },
    )
    .expect("reserve");
    assert_eq!(s_reserved.status, SerialStatus::Reserved);

    // Reserved -> Sold
    let s_sold = update_serial_status(
        &conn,
        &UpdateSerialStatusInput {
            id: inst.id.clone(),
            branch_id: branch_id.clone(),
            status: SerialStatus::Sold,
        },
    )
    .expect("sell");
    assert_eq!(s_sold.status, SerialStatus::Sold);

    // Sold -> Defective (Customer returned defective)
    let s_defective = update_serial_status(
        &conn,
        &UpdateSerialStatusInput {
            id: inst.id,
            branch_id,
            status: SerialStatus::Defective,
        },
    )
    .expect("defective");
    assert_eq!(s_defective.status, SerialStatus::Defective);
}

#[test]
fn test_lifecycle_terminal_status_cannot_transition() {
    let conn = setup_test_db();
    let (_, branch_id) = create_test_org_and_branch(&conn);
    let product_id = make_test_product(&conn, "Component", true);

    let inst = create_serial_instance(
        &conn,
        &CreateSerialInput {
            product_id,
            branch_id: branch_id.clone(),
            variant_id: None,
            serial_number: Some("TERMINAL-TEST".into()),
            imei: None,
            asset_tag: None,
            cost_price_minor: None,
        },
    )
    .expect("create");

    // Transition to Recalled
    let s_recalled = update_serial_status(
        &conn,
        &UpdateSerialStatusInput {
            id: inst.id.clone(),
            branch_id: branch_id.clone(),
            status: SerialStatus::Recalled,
        },
    )
    .expect("recalled");
    assert_eq!(s_recalled.status, SerialStatus::Recalled);

    // Attempt transition out of Recalled -> MUST FAIL
    let err = update_serial_status(
        &conn,
        &UpdateSerialStatusInput {
            id: inst.id,
            branch_id,
            status: SerialStatus::InStock,
        },
    )
    .unwrap_err();
    assert!(matches!(err, SerialError::TerminalStatus(_)));
}

// =========================================================================
// 7. QUANTITY & LEDGER FIREWALL TESTS
// =========================================================================

#[test]
fn test_quantity_and_stock_ledger_firewall_untouched() {
    let conn = setup_test_db();
    let (_, branch_id) = create_test_org_and_branch(&conn);
    let product_id = make_test_product(&conn, "Firewall Test Laptop", true);

    // Verify initial inventory state
    let initial_movements: i64 = conn
        .query_row("SELECT count(*) FROM stock_movements", [], |r| r.get(0))
        .expect("count movements");
    assert_eq!(initial_movements, 0);

    // Create 3 serial instances
    for i in 1..=3 {
        create_serial_instance(
            &conn,
            &CreateSerialInput {
                product_id: product_id.clone(),
                branch_id: branch_id.clone(),
                variant_id: None,
                serial_number: Some(format!("FIREWALL-SN-{i}")),
                imei: None,
                asset_tag: None,
                cost_price_minor: None,
            },
        )
        .expect("create instance");
    }

    // Assert zero stock movements written
    let final_movements: i64 = conn
        .query_row("SELECT count(*) FROM stock_movements", [], |r| r.get(0))
        .expect("count movements");
    assert_eq!(
        final_movements, 0,
        "F2.08 must NEVER write to stock_movements (strictly deferred to F2.11)"
    );

    // Assert inventory balance table is unmutated
    let inv_count: i64 = conn
        .query_row(
            "SELECT count(*) FROM inventory WHERE product_id = ?1",
            params![product_id],
            |r| r.get(0),
        )
        .expect("count inventory");
    assert_eq!(
        inv_count, 0,
        "F2.08 must NEVER mutate inventory balances (strictly deferred to F2.11)"
    );
}

// =========================================================================
// 8. IPC COMMANDS & SECURITY TESTS
// =========================================================================

#[test]
fn test_create_serial_ipc_command_authorization() {
    let conn = setup_test_db();
    let (_, branch_id) = create_test_org_and_branch(&conn);
    let manager = create_test_user_with_creds(
        &conn,
        &branch_id,
        "Serial Manager",
        Some("ser_mgr"),
        Some("pass123"),
        Some("1234"),
        "manager",
    )
    .expect("manager");
    let cashier = create_test_user_with_creds(
        &conn,
        &branch_id,
        "Serial Cashier",
        Some("ser_cashier"),
        Some("pass123"),
        Some("1234"),
        "cashier",
    )
    .expect("cashier");

    let product_id = make_test_product(&conn, "IPC Tablet", true);

    let session_mgr =
        create_local_session(&conn, &manager.id, &branch_id, "pin", None).expect("session mgr");
    let session_cashier =
        create_local_session(&conn, &cashier.id, &branch_id, "pin", None).expect("session cashier");

    let req = CreateSerialInput {
        product_id,
        branch_id,
        variant_id: None,
        serial_number: Some("IPC-SN-001".into()),
        imei: None,
        asset_tag: None,
        cost_price_minor: None,
    };

    // Cashier denied
    let err_cashier = create_serial_instance_impl(&conn, &session_cashier.id, &req).unwrap_err();
    assert!(err_cashier.contains("Permission denied") || err_cashier.contains("permission"));

    // Manager succeeds
    let inst = create_serial_instance_impl(&conn, &session_mgr.id, &req).expect("manager created");
    assert_eq!(inst.serial_number.as_deref(), Some("IPC-SN-001"));
}

#[test]
fn test_get_and_lookup_serial_ipc_tenancy_boundary_prevent_leakage() {
    let conn = setup_test_db();
    let (org_id, branch_1, user) = create_test_user_hierarchy(&conn);
    let branch_2 = crate::branch::create_branch(
        &conn,
        crate::branch::CreateBranchInput {
            organization_id: org_id,
            name: "Branch 2".into(),
            address: None,
            currency: None,
            is_active: Some(true),
        },
    )
    .unwrap()
    .id;

    let product_id = make_test_product(&conn, "Tenancy Device", true);

    // Create serial in Branch 2
    let inst_b2 = create_serial_instance(
        &conn,
        &CreateSerialInput {
            product_id,
            branch_id: branch_2.clone(),
            variant_id: None,
            serial_number: Some("SECRET-BR2-SN".into()),
            imei: None,
            asset_tag: None,
            cost_price_minor: None,
        },
    )
    .expect("create in branch 2");

    // Session scoped strictly to branch 1
    let session_b1 =
        create_local_session(&conn, &user.id, &branch_1, "pin", None).expect("session");

    // Attempting get_serial_instance for branch 2 item from branch 1 session must fail closed
    let res_get = get_serial_instance_impl(&conn, &session_b1.id, &inst_b2.id);
    assert!(
        res_get.is_err(),
        "Cross-branch access must fail closed to prevent existence leakage"
    );

    // Lookup in branch 1 must return None
    let res_lookup = lookup_serial_instance_impl(&conn, &session_b1.id, "SECRET-BR2-SN", &branch_1)
        .expect("lookup in branch 1");
    assert!(
        res_lookup.is_none(),
        "Lookup in branch 1 must not reveal branch 2 serial"
    );

    // Lookup targeting branch 2 using branch 1 session must be denied by middleware
    let res_leak = lookup_serial_instance_impl(&conn, &session_b1.id, "SECRET-BR2-SN", &branch_2);
    assert!(res_leak.is_err(), "Cross-branch lookup must be denied");
}

#[test]
fn test_update_serial_status_ipc_command() {
    let conn = setup_test_db();
    let (_, branch_id) = create_test_org_and_branch(&conn);
    let manager = create_test_user_with_creds(
        &conn,
        &branch_id,
        "Status Manager",
        Some("status_mgr"),
        Some("pass123"),
        Some("1234"),
        "manager",
    )
    .expect("manager");

    let session_mgr =
        create_local_session(&conn, &manager.id, &branch_id, "pin", None).expect("session");

    let product_id = make_test_product(&conn, "Device", true);
    let inst = create_serial_instance(
        &conn,
        &CreateSerialInput {
            product_id,
            branch_id: branch_id.clone(),
            variant_id: None,
            serial_number: Some("STATUS-IPC-SN".into()),
            imei: None,
            asset_tag: None,
            cost_price_minor: None,
        },
    )
    .expect("create");

    let updated = update_serial_status_impl(
        &conn,
        &session_mgr.id,
        &UpdateSerialStatusInput {
            id: inst.id,
            branch_id,
            status: SerialStatus::Reserved,
        },
    )
    .expect("update status");

    assert_eq!(updated.status, SerialStatus::Reserved);
}

#[test]
fn test_list_serial_instances_ipc_command() {
    let conn = setup_test_db();
    let (org_id, branch_id, user) = create_test_user_hierarchy(&conn);
    let _ = org_id;
    let session = create_local_session(&conn, &user.id, &branch_id, "pin", None).expect("session");

    let p1 = make_test_product(&conn, "Product A", true);
    let p2 = make_test_product(&conn, "Product B", true);

    create_serial_instance(
        &conn,
        &CreateSerialInput {
            product_id: p1.clone(),
            branch_id: branch_id.clone(),
            variant_id: None,
            serial_number: Some("SN-LIST-A1".into()),
            imei: None,
            asset_tag: None,
            cost_price_minor: None,
        },
    )
    .unwrap();

    create_serial_instance(
        &conn,
        &CreateSerialInput {
            product_id: p2,
            branch_id: branch_id.clone(),
            variant_id: None,
            serial_number: Some("SN-LIST-B1".into()),
            imei: None,
            asset_tag: None,
            cost_price_minor: None,
        },
    )
    .unwrap();

    // List all for branch
    let all = list_serial_instances_impl(
        &conn,
        &session.id,
        &SerialFilter {
            branch_id: branch_id.clone(),
            product_id: None,
            variant_id: None,
            status: None,
        },
    )
    .expect("list all");
    assert_eq!(all.len(), 2);

    // Filter by product_id
    let filtered = list_serial_instances_impl(
        &conn,
        &session.id,
        &SerialFilter {
            branch_id,
            product_id: Some(p1),
            variant_id: None,
            status: None,
        },
    )
    .expect("filter by product");
    assert_eq!(filtered.len(), 1);
    assert_eq!(filtered[0].serial_number.as_deref(), Some("SN-LIST-A1"));
}

#[test]
fn test_update_serial_status_auth_and_leakage_order() {
    let conn = setup_test_db();
    let (org_id, branch_1, _) = create_test_user_hierarchy(&conn);
    let branch_2 = crate::branch::create_branch(
        &conn,
        crate::branch::CreateBranchInput {
            organization_id: org_id,
            name: "Branch 2".into(),
            address: None,
            currency: None,
            is_active: Some(true),
        },
    )
    .unwrap()
    .id;

    let manager_b1 = create_test_user_with_creds(
        &conn,
        &branch_1,
        "Manager B1",
        Some("mgr_b1"),
        Some("pass123"),
        Some("1234"),
        "manager",
    )
    .unwrap();
    let cashier_b1 = create_test_user_with_creds(
        &conn,
        &branch_1,
        "Cashier B1",
        Some("cashier_b1"),
        Some("pass123"),
        Some("1234"),
        "cashier",
    )
    .unwrap();

    let product_id = make_test_product(&conn, "Security Device", true);

    // Create item in branch 2
    let inst_b2 = create_serial_instance(
        &conn,
        &CreateSerialInput {
            product_id: product_id.clone(),
            branch_id: branch_2,
            variant_id: None,
            serial_number: Some("B2-SECRET-ITEM".into()),
            imei: None,
            asset_tag: None,
            cost_price_minor: None,
        },
    )
    .unwrap();

    // Create item in branch 1
    let inst_b1 = create_serial_instance(
        &conn,
        &CreateSerialInput {
            product_id,
            branch_id: branch_1.clone(),
            variant_id: None,
            serial_number: Some("B1-ITEM".into()),
            imei: None,
            asset_tag: None,
            cost_price_minor: None,
        },
    )
    .unwrap();

    let session_mgr_b1 =
        create_local_session(&conn, &manager_b1.id, &branch_1, "pin", None).unwrap();
    let session_cashier_b1 =
        create_local_session(&conn, &cashier_b1.id, &branch_1, "pin", None).unwrap();

    // 1. Unauthenticated request denied
    let err_unauth = update_serial_status_impl(
        &conn,
        "invalid_session_id",
        &UpdateSerialStatusInput {
            id: inst_b1.id.clone(),
            branch_id: branch_1.clone(),
            status: SerialStatus::Reserved,
        },
    )
    .unwrap_err();
    assert!(err_unauth.contains("Authentication required"));

    // 2. Authenticated but unauthorized (cashier lacking inventory.adjust)
    let err_perm = update_serial_status_impl(
        &conn,
        &session_cashier_b1.id,
        &UpdateSerialStatusInput {
            id: inst_b1.id.clone(),
            branch_id: branch_1.clone(),
            status: SerialStatus::Reserved,
        },
    )
    .unwrap_err();
    assert!(err_perm.contains("Permission denied") || err_perm.contains("permission"));

    // 3. Nonexistent ID with manager session
    let err_nonexistent = update_serial_status_impl(
        &conn,
        &session_mgr_b1.id,
        &UpdateSerialStatusInput {
            id: "nonexistent-id".into(),
            branch_id: branch_1.clone(),
            status: SerialStatus::Reserved,
        },
    )
    .unwrap_err();
    assert_eq!(
        err_nonexistent,
        "Serial instance 'nonexistent-id' not found or inaccessible for this session"
    );

    // 4. Branch 2 item queried using Branch 1 credentials -> Must return identical "not found or inaccessible" error
    // (zero existence leakage: manager cannot discover whether B2 item exists)
    let err_leakage = update_serial_status_impl(
        &conn,
        &session_mgr_b1.id,
        &UpdateSerialStatusInput {
            id: inst_b2.id.clone(),
            branch_id: branch_1.clone(),
            status: SerialStatus::Reserved,
        },
    )
    .unwrap_err();
    assert_eq!(
        err_leakage,
        format!(
            "Serial instance '{}' not found or inaccessible for this session",
            inst_b2.id
        )
    );

    // 5. Authorized update succeeds
    let updated = update_serial_status_impl(
        &conn,
        &session_mgr_b1.id,
        &UpdateSerialStatusInput {
            id: inst_b1.id,
            branch_id: branch_1,
            status: SerialStatus::Reserved,
        },
    )
    .expect("authorized update succeeds");
    assert_eq!(updated.status, SerialStatus::Reserved);
}

#[test]
fn test_map_sqlite_collision_error_coverage() {
    let conn = setup_test_db();
    let (_, branch_id) = create_test_org_and_branch(&conn);
    let product_id = make_test_product(&conn, "Collision Product", true);

    create_serial_instance(
        &conn,
        &CreateSerialInput {
            product_id: product_id.clone(),
            branch_id: branch_id.clone(),
            variant_id: None,
            serial_number: Some("COLLIDE-SN".into()),
            imei: Some(VALID_IMEI_1.into()),
            asset_tag: Some("COLLIDE-TAG".into()),
            cost_price_minor: None,
        },
    )
    .unwrap();

    // Trigger raw SQLite unique violation on serial_number
    let err_ser = conn
        .execute(
            "INSERT INTO serial_numbers (product_id, branch_id, serial_number) VALUES (?1, ?2, 'collide-sn')",
            params![product_id, branch_id],
        )
        .unwrap_err();
    let mapped_ser = SerialError::from(err_ser);
    assert!(matches!(mapped_ser, SerialError::Database(_)));

    // Verify duplicate error mappings through domain engine
    let dup_sn = create_serial_instance(
        &conn,
        &CreateSerialInput {
            product_id: product_id.clone(),
            branch_id: branch_id.clone(),
            variant_id: None,
            serial_number: Some("COLLIDE-SN".into()),
            imei: None,
            asset_tag: None,
            cost_price_minor: None,
        },
    )
    .unwrap_err();
    assert!(matches!(dup_sn, SerialError::DuplicateSerial(_)));

    let dup_imei = create_serial_instance(
        &conn,
        &CreateSerialInput {
            product_id: product_id.clone(),
            branch_id: branch_id.clone(),
            variant_id: None,
            serial_number: None,
            imei: Some(VALID_IMEI_1.into()),
            asset_tag: None,
            cost_price_minor: None,
        },
    )
    .unwrap_err();
    assert!(matches!(dup_imei, SerialError::DuplicateImei(_)));

    let dup_tag = create_serial_instance(
        &conn,
        &CreateSerialInput {
            product_id,
            branch_id,
            variant_id: None,
            serial_number: None,
            imei: None,
            asset_tag: Some("collide-tag".into()),
            cost_price_minor: None,
        },
    )
    .unwrap_err();
    assert!(matches!(dup_tag, SerialError::DuplicateAssetTag(_)));
}
