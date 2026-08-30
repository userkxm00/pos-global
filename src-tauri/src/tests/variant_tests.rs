// Unit, migration, and domain contract tests for F2.05 Variants / Matrix.

use crate::product::{create_product, CreateProductInput};
use crate::tests::test_helpers::{apply_migrations_up_to, setup_test_db, setup_test_db_up_to};
use crate::variant::{
    create_attribute_definition, create_attribute_value, create_variant, get_attribute_definition,
    get_variant, list_attribute_definitions, list_attribute_values_by_definition,
    list_variants_by_product, soft_delete_variant, update_variant, validate_attribute_name,
    validate_attribute_value, validate_price_minor, CreateAttributeDefinitionInput,
    CreateAttributeValueInput, CreateVariantInput, UpdateVariantInput, VariantError,
};
use rusqlite::params;

fn create_sample_product(conn: &rusqlite::Connection, name: &str) -> String {
    let prod = create_product(
        conn,
        CreateProductInput {
            name: name.to_string(),
            description: Some("Test Product".to_string()),
            category_id: None,
            sku: None,
            barcode: None,
            product_type: Some("standard".to_string()),
            base_price_minor: 5000,
            cost_price_minor: Some(2500),
            unit_type: None,
            requires_expiry: None,
            requires_serial: None,
            warranty_months: None,
            custom_attributes: None,
        },
    )
    .expect("create product");
    prod.id
}

// ---------------------------------------------------------------------------
// 1. MIGRATION 014 TESTS
// ---------------------------------------------------------------------------

#[test]
fn test_migration_014_applies_cleanly_to_fresh_database() {
    let conn = setup_test_db();
    let applied: i64 = conn
        .query_row("SELECT COUNT(*) FROM _migrations", [], |row| row.get(0))
        .expect("query migrations");
    assert_eq!(applied, 14);

    // Verify product_variants new columns exist
    let columns = [
        "price_override_minor",
        "cost_price_minor",
        "created_at",
        "updated_at",
        "deleted_at",
    ];
    for col in columns {
        let count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM pragma_table_info('product_variants') WHERE name = ?1",
                [col],
                |row| row.get(0),
            )
            .expect("pragma query");
        assert_eq!(count, 1, "column {col} must exist in product_variants");
    }

    // Verify indexes exist
    let indexes = [
        "idx_attribute_definitions_name_nocase",
        "idx_attribute_values_def_val_nocase",
        "idx_product_variants_sku_active",
        "idx_product_variants_barcode_active",
        "idx_product_variants_product",
        "idx_variant_attribute_values_variant",
        "idx_variant_attribute_values_value",
    ];
    for idx in indexes {
        let count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE type = 'index' AND name = ?1",
                [idx],
                |row| row.get(0),
            )
            .expect("index query");
        assert_eq!(count, 1, "index {idx} must exist");
    }
}

#[test]
fn test_migration_014_upgrades_from_013_with_representative_data() {
    // 1. Initialize up to 013
    let conn = setup_test_db_up_to("013_units_conversions_hardening");

    let pre_count: i64 = conn
        .query_row("SELECT COUNT(*) FROM _migrations", [], |row| row.get(0))
        .expect("count");
    assert_eq!(pre_count, 13);

    // Seed product before 014
    let prod_id = uuid::Uuid::new_v4().to_string();
    conn.execute(
        "INSERT INTO products (id, name, base_price, is_active, created_at, updated_at)
         VALUES (?1, 'Legacy T-Shirt', 20.0, 1, '2026-01-01 00:00:00', '2026-01-01 00:00:00')",
        params![prod_id],
    )
    .expect("seed product");

    // Seed legacy attribute definition
    let def_id = uuid::Uuid::new_v4().to_string();
    conn.execute(
        "INSERT INTO attribute_definitions (id, name) VALUES (?1, 'Size')",
        params![def_id],
    )
    .expect("seed attribute def");

    // Seed legacy attribute value
    let val_id = uuid::Uuid::new_v4().to_string();
    conn.execute(
        "INSERT INTO attribute_values (id, attribute_definition_id, value) VALUES (?1, ?2, 'Medium')",
        params![val_id, def_id],
    )
    .expect("seed attribute value");

    // Seed legacy product variant with legacy price_override REAL = 24.99
    let var_id = uuid::Uuid::new_v4().to_string();
    conn.execute(
        "INSERT INTO product_variants (id, product_id, sku, barcode, price_override, is_active)
         VALUES (?1, ?2, 'TSHIRT-M', '112233445566', 24.99, 1)",
        params![var_id, prod_id],
    )
    .expect("seed legacy variant");

    // 2. Apply Migration 014
    apply_migrations_up_to(&conn, "014_product_variants_hardening");

    let post_count: i64 = conn
        .query_row("SELECT COUNT(*) FROM _migrations", [], |row| row.get(0))
        .expect("count");
    assert_eq!(post_count, 14);

    // 3. Verify legacy data and identity retention
    let (backfilled_sku, backfilled_barcode, backfilled_minor, var_created_at, var_updated_at): (
        String,
        String,
        i64,
        String,
        String,
    ) = conn
        .query_row(
            "SELECT sku, barcode, price_override_minor, created_at, updated_at FROM product_variants WHERE id = ?1",
            params![var_id],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?, row.get(4)?)),
        )
        .expect("query variant row");
    assert_eq!(backfilled_sku, "TSHIRT-M");
    assert_eq!(backfilled_barcode, "112233445566");
    assert_eq!(backfilled_minor, 2499);
    assert!(!var_created_at.is_empty());
    assert!(!var_updated_at.is_empty());

    // 4. Verify sort_order defaulted to 0 and created_at was populated
    let (def_sort_order, def_created_at): (i64, String) = conn
        .query_row(
            "SELECT sort_order, created_at FROM attribute_definitions WHERE id = ?1",
            params![def_id],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .expect("query sort_order and created_at");
    assert_eq!(def_sort_order, 0);
    assert!(!def_created_at.is_empty());

    let (val_sort_order, val_created_at): (i64, String) = conn
        .query_row(
            "SELECT sort_order, created_at FROM attribute_values WHERE id = ?1",
            params![val_id],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .expect("query sort_order and created_at");
    assert_eq!(val_sort_order, 0);
    assert!(!val_created_at.is_empty());

    // 5. Verify all 7 indexes exist
    let indexes = [
        "idx_attribute_definitions_name_nocase",
        "idx_attribute_values_def_val_nocase",
        "idx_product_variants_sku_active",
        "idx_product_variants_barcode_active",
        "idx_product_variants_product",
        "idx_variant_attribute_values_variant",
        "idx_variant_attribute_values_value",
    ];
    for idx in indexes {
        let count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE type = 'index' AND name = ?1",
                [idx],
                |row| row.get(0),
            )
            .expect("index query");
        assert_eq!(count, 1, "index {idx} must exist after upgrade");
    }
}

#[test]
fn test_migration_014_repeatability() {
    let conn = setup_test_db();
    let count1: i64 = conn
        .query_row("SELECT COUNT(*) FROM _migrations", [], |row| row.get(0))
        .expect("count");
    assert_eq!(count1, 14);

    // Re-running init_database is an idempotent no-op
    crate::db::init_database(&conn).expect("repeatable init must succeed");

    let count2: i64 = conn
        .query_row("SELECT COUNT(*) FROM _migrations", [], |row| row.get(0))
        .expect("count");
    assert_eq!(count2, 14);
}

// ---------------------------------------------------------------------------
// 2. DOMAIN CONTRACT & VALIDATION TESTS
// ---------------------------------------------------------------------------

#[test]
fn test_attribute_definition_validation_and_uniqueness() {
    let conn = setup_test_db();

    // 1. Validation: Empty name rejected
    assert!(matches!(
        validate_attribute_name(""),
        Err(VariantError::Validation(_))
    ));
    assert!(matches!(
        validate_attribute_name("   "),
        Err(VariantError::Validation(_))
    ));

    // 2. Validation: Long name rejected
    let long_name = "a".repeat(101);
    assert!(matches!(
        validate_attribute_name(&long_name),
        Err(VariantError::Validation(_))
    ));

    // 3. Create valid definition
    let def = create_attribute_definition(
        &conn,
        CreateAttributeDefinitionInput {
            name: "Color".to_string(),
            sort_order: Some(1),
        },
    )
    .expect("create attribute definition");
    assert_eq!(def.name, "Color");
    assert_eq!(def.sort_order, 1);

    // 4. Duplicate name rejected case-insensitively
    let dup_err = create_attribute_definition(
        &conn,
        CreateAttributeDefinitionInput {
            name: "color".to_string(),
            sort_order: Some(2),
        },
    )
    .unwrap_err();
    assert!(matches!(dup_err, VariantError::DuplicateName(_)));

    // 5. Query and list
    let loaded = get_attribute_definition(&conn, &def.id)
        .expect("get")
        .expect("found");
    assert_eq!(loaded.id, def.id);

    let list = list_attribute_definitions(&conn).expect("list");
    assert_eq!(list.len(), 1);
}

#[test]
fn test_attribute_value_validation_and_scoping() {
    let conn = setup_test_db();

    let def1 = create_attribute_definition(
        &conn,
        CreateAttributeDefinitionInput {
            name: "Size".to_string(),
            sort_order: Some(1),
        },
    )
    .expect("def1");

    let def2 = create_attribute_definition(
        &conn,
        CreateAttributeDefinitionInput {
            name: "Material".to_string(),
            sort_order: Some(2),
        },
    )
    .expect("def2");

    // 1. Validation: Empty value rejected
    assert!(matches!(
        validate_attribute_value(""),
        Err(VariantError::Validation(_))
    ));

    // 2. Missing parent definition rejected
    let not_found = create_attribute_value(
        &conn,
        CreateAttributeValueInput {
            attribute_definition_id: "non-existent-def".to_string(),
            value: "Large".to_string(),
            sort_order: None,
        },
    )
    .unwrap_err();
    assert!(matches!(not_found, VariantError::NotFound(_)));

    // 3. Create valid values
    let val_small = create_attribute_value(
        &conn,
        CreateAttributeValueInput {
            attribute_definition_id: def1.id.clone(),
            value: "Small".to_string(),
            sort_order: Some(1),
        },
    )
    .expect("small");
    assert_eq!(val_small.value, "Small");

    // 4. Duplicate value in same definition rejected case-insensitively
    let dup_err = create_attribute_value(
        &conn,
        CreateAttributeValueInput {
            attribute_definition_id: def1.id.clone(),
            value: "small".to_string(),
            sort_order: Some(2),
        },
    )
    .unwrap_err();
    assert!(matches!(dup_err, VariantError::DuplicateValue(_)));

    // 5. Same value in different definition allowed
    let val_mat = create_attribute_value(
        &conn,
        CreateAttributeValueInput {
            attribute_definition_id: def2.id.clone(),
            value: "Small".to_string(), // e.g. Small weave
            sort_order: Some(1),
        },
    )
    .expect("small in material");
    assert_eq!(val_mat.value, "Small");

    // 6. List values scoped by definition
    let def1_vals = list_attribute_values_by_definition(&conn, &def1.id).expect("list");
    assert_eq!(def1_vals.len(), 1);
    assert_eq!(def1_vals[0].id, val_small.id);
}

#[test]
fn test_product_variant_lifecycle_and_validation() {
    let conn = setup_test_db();
    let product_id = create_sample_product(&conn, "Classic Polo");

    let def_size = create_attribute_definition(
        &conn,
        CreateAttributeDefinitionInput {
            name: "Size".to_string(),
            sort_order: Some(1),
        },
    )
    .expect("def_size");

    let val_m = create_attribute_value(
        &conn,
        CreateAttributeValueInput {
            attribute_definition_id: def_size.id.clone(),
            value: "Medium".to_string(),
            sort_order: Some(1),
        },
    )
    .expect("val_m");

    let val_l = create_attribute_value(
        &conn,
        CreateAttributeValueInput {
            attribute_definition_id: def_size.id.clone(),
            value: "Large".to_string(),
            sort_order: Some(2),
        },
    )
    .expect("val_l");

    // 1. Validation: Negative price override rejected
    assert!(matches!(
        validate_price_minor(Some(-100)),
        Err(VariantError::Validation(_))
    ));

    // 2. Missing parent product rejected
    let bad_prod_err = create_variant(
        &conn,
        CreateVariantInput {
            product_id: "non-existent-prod".to_string(),
            sku: Some("POLO-M".to_string()),
            barcode: None,
            price_override_minor: Some(5500),
            cost_price_minor: Some(2800),
            attribute_value_ids: vec![val_m.id.clone()],
        },
    )
    .unwrap_err();
    assert!(matches!(bad_prod_err, VariantError::NotFound(_)));

    // 3. Create valid variant with attributes
    let v_m = create_variant(
        &conn,
        CreateVariantInput {
            product_id: product_id.clone(),
            sku: Some("POLO-M".to_string()),
            barcode: Some("123456789012".to_string()),
            price_override_minor: Some(5500),
            cost_price_minor: Some(2800),
            attribute_value_ids: vec![val_m.id.clone()],
        },
    )
    .expect("create v_m");

    assert_eq!(v_m.variant.sku, Some("POLO-M".to_string()));
    assert_eq!(v_m.variant.price_override_minor, Some(5500));
    assert_eq!(v_m.attribute_values.len(), 1);
    assert_eq!(v_m.attribute_values[0].value, "Medium");

    // 4. Duplicate combination rejected
    let dup_comb_err = create_variant(
        &conn,
        CreateVariantInput {
            product_id: product_id.clone(),
            sku: Some("POLO-M-2".to_string()),
            barcode: None,
            price_override_minor: None,
            cost_price_minor: None,
            attribute_value_ids: vec![val_m.id.clone()],
        },
    )
    .unwrap_err();
    assert!(matches!(
        dup_comb_err,
        VariantError::DuplicateCombination(_)
    ));

    // 5. Create variant with different combination (Large)
    let v_l = create_variant(
        &conn,
        CreateVariantInput {
            product_id: product_id.clone(),
            sku: Some("POLO-L".to_string()),
            barcode: None,
            price_override_minor: Some(6000),
            cost_price_minor: Some(3000),
            attribute_value_ids: vec![val_l.id.clone()],
        },
    )
    .expect("create v_l");
    assert_eq!(v_l.variant.sku, Some("POLO-L".to_string()));

    // 6. List active variants for product
    let active_variants = list_variants_by_product(&conn, &product_id, Some(true)).expect("list");
    assert_eq!(active_variants.len(), 2);

    // 7. Update variant
    let updated = update_variant(
        &conn,
        UpdateVariantInput {
            id: v_m.variant.id.clone(),
            sku: Some("POLO-M-UPDATED".to_string()),
            barcode: Some("999999999999".to_string()),
            price_override_minor: Some(5800),
            cost_price_minor: Some(2900),
            is_active: true,
        },
    )
    .expect("update");
    assert_eq!(updated.sku, Some("POLO-M-UPDATED".to_string()));
    assert_eq!(updated.price_override_minor, Some(5800));

    // 8. Soft delete variant
    soft_delete_variant(&conn, &v_m.variant.id).expect("soft delete");

    let loaded_deleted = get_variant(&conn, &v_m.variant.id)
        .expect("get")
        .expect("found");
    assert!(!loaded_deleted.is_active);
    assert!(loaded_deleted.deleted_at.is_some());

    let active_now = list_variants_by_product(&conn, &product_id, Some(true)).expect("list");
    assert_eq!(active_now.len(), 1);
    assert_eq!(active_now[0].id, v_l.variant.id);
}

#[test]
fn test_product_variant_multiple_values_for_same_definition_rejected() {
    let conn = setup_test_db();
    let product_id = create_sample_product(&conn, "T-Shirt");

    let def_size = create_attribute_definition(
        &conn,
        CreateAttributeDefinitionInput {
            name: "Size".to_string(),
            sort_order: Some(1),
        },
    )
    .expect("def");

    let val_s = create_attribute_value(
        &conn,
        CreateAttributeValueInput {
            attribute_definition_id: def_size.id.clone(),
            value: "Small".to_string(),
            sort_order: Some(1),
        },
    )
    .expect("val_s");

    let val_m = create_attribute_value(
        &conn,
        CreateAttributeValueInput {
            attribute_definition_id: def_size.id.clone(),
            value: "Medium".to_string(),
            sort_order: Some(2),
        },
    )
    .expect("val_m");

    // Providing both "Small" and "Medium" (both Size) for one variant must fail validation
    let err = create_variant(
        &conn,
        CreateVariantInput {
            product_id,
            sku: Some("TSHIRT-SM".to_string()),
            barcode: None,
            price_override_minor: None,
            cost_price_minor: None,
            attribute_value_ids: vec![val_s.id, val_m.id],
        },
    )
    .unwrap_err();

    assert!(matches!(err, VariantError::Validation(_)));
}

#[test]
fn test_duplicate_variant_barcode_maps_to_typed_error() {
    let conn = setup_test_db();
    let prod1_id = create_sample_product(&conn, "Product 1");
    let prod2_id = create_sample_product(&conn, "Product 2");

    let def_size = create_attribute_definition(
        &conn,
        CreateAttributeDefinitionInput {
            name: "Size".to_string(),
            sort_order: Some(1),
        },
    )
    .expect("def");

    let val_s = create_attribute_value(
        &conn,
        CreateAttributeValueInput {
            attribute_definition_id: def_size.id.clone(),
            value: "Small".to_string(),
            sort_order: Some(1),
        },
    )
    .expect("val_s");

    let val_m = create_attribute_value(
        &conn,
        CreateAttributeValueInput {
            attribute_definition_id: def_size.id.clone(),
            value: "Medium".to_string(),
            sort_order: Some(2),
        },
    )
    .expect("val_m");

    // 1. Create first variant with barcode
    create_variant(
        &conn,
        CreateVariantInput {
            product_id: prod1_id,
            sku: Some("P1-S".to_string()),
            barcode: Some("987654321098".to_string()),
            price_override_minor: None,
            cost_price_minor: None,
            attribute_value_ids: vec![val_s.id],
        },
    )
    .expect("variant 1");

    // 2. Attempt to create second variant with identical barcode
    let err = create_variant(
        &conn,
        CreateVariantInput {
            product_id: prod2_id,
            sku: Some("P2-M".to_string()),
            barcode: Some("987654321098".to_string()),
            price_override_minor: None,
            cost_price_minor: None,
            attribute_value_ids: vec![val_m.id],
        },
    )
    .unwrap_err();

    assert!(
        matches!(err, VariantError::DuplicateBarcode(_)),
        "Expected VariantError::DuplicateBarcode, got: {:?}",
        err
    );
}

#[test]
fn test_create_variant_atomicity_rollback_on_association_failure() {
    let conn = setup_test_db();
    let product_id = create_sample_product(&conn, "Atomicity Product");

    let pre_variant_count: i64 = conn
        .query_row("SELECT COUNT(*) FROM product_variants", [], |row| {
            row.get(0)
        })
        .expect("count");
    let pre_assoc_count: i64 = conn
        .query_row("SELECT COUNT(*) FROM variant_attribute_values", [], |row| {
            row.get(0)
        })
        .expect("count");

    // Attempt to create variant with a non-existent attribute value id
    let err = create_variant(
        &conn,
        CreateVariantInput {
            product_id,
            sku: Some("ATOM-1".to_string()),
            barcode: Some("555555555555".to_string()),
            price_override_minor: Some(1000),
            cost_price_minor: None,
            attribute_value_ids: vec!["non-existent-attr-val-id".to_string()],
        },
    )
    .unwrap_err();

    assert!(matches!(err, VariantError::NotFound(_)));

    // Verify transaction rollback: zero orphaned variant rows or association rows
    let post_variant_count: i64 = conn
        .query_row("SELECT COUNT(*) FROM product_variants", [], |row| {
            row.get(0)
        })
        .expect("count");
    let post_assoc_count: i64 = conn
        .query_row("SELECT COUNT(*) FROM variant_attribute_values", [], |row| {
            row.get(0)
        })
        .expect("count");

    assert_eq!(
        pre_variant_count, post_variant_count,
        "No product_variants row should remain after failure"
    );
    assert_eq!(
        pre_assoc_count, post_assoc_count,
        "No variant_attribute_values row should remain after failure"
    );
}

#[test]
fn test_create_variant_concurrent_combination_uniqueness_multi_connection() {
    use std::sync::{Arc, Barrier};
    use std::thread;

    // Use a unique file-backed database so multiple independent connections can access it concurrently
    let temp_dir = std::env::temp_dir();
    let db_path = temp_dir.join(format!("test_concurrency_{}.sqlite", uuid::Uuid::new_v4()));

    // 1. Initialize schema and base data
    {
        let conn = rusqlite::Connection::open(&db_path).expect("open db");
        crate::db::init_database(&conn).expect("init db");
        conn.execute_batch("PRAGMA journal_mode = WAL;")
            .expect("set WAL");
    }

    let conn_setup = rusqlite::Connection::open(&db_path).expect("open setup conn");
    let product_id = create_sample_product(&conn_setup, "Concurrency Polo");

    let def_size = create_attribute_definition(
        &conn_setup,
        CreateAttributeDefinitionInput {
            name: "Size".to_string(),
            sort_order: Some(1),
        },
    )
    .expect("create def");

    let val_m = create_attribute_value(
        &conn_setup,
        CreateAttributeValueInput {
            attribute_definition_id: def_size.id.clone(),
            value: "Medium".to_string(),
            sort_order: Some(1),
        },
    )
    .expect("create val");

    drop(conn_setup);

    // 2. Set up two independent threads with separate SQLite connections synchronized with a barrier
    let barrier = Arc::new(Barrier::new(2));
    let mut handles = Vec::new();

    for thread_idx in 0..2 {
        let b = Arc::clone(&barrier);
        let p_id = product_id.clone();
        let val_id = val_m.id.clone();
        let path = db_path.clone();

        let handle = thread::spawn(move || {
            let conn = rusqlite::Connection::open(&path).expect("open thread conn");
            conn.busy_timeout(std::time::Duration::from_secs(5))
                .expect("set busy timeout");

            // Wait for both threads to reach the execution point
            b.wait();

            create_variant(
                &conn,
                CreateVariantInput {
                    product_id: p_id,
                    sku: Some(format!("CONC-M-{}", thread_idx)),
                    barcode: Some(format!("11112222333{}", thread_idx)),
                    price_override_minor: Some(5000),
                    cost_price_minor: Some(2500),
                    attribute_value_ids: vec![val_id],
                },
            )
        });
        handles.push(handle);
    }

    // 3. Collect results from both competing operations
    let results: Vec<Result<crate::variant::VariantWithAttributes, VariantError>> =
        handles.into_iter().map(|h| h.join().unwrap()).collect();

    let success_count = results.iter().filter(|r| r.is_ok()).count();
    let error_count = results.iter().filter(|r| r.is_err()).count();

    // 4. Verify invariant: Exactly one create succeeds and the other fails closed
    assert_eq!(
        success_count, 1,
        "Expected exactly one successful variant creation under concurrent race, got: {success_count}"
    );
    assert_eq!(
        error_count, 1,
        "Expected exactly one rejected operation under concurrent race"
    );

    for res in &results {
        if let Err(err) = res {
            assert!(
                matches!(
                    err,
                    VariantError::DuplicateCombination(_) | VariantError::Database(_)
                ),
                "Losing operation returned unexpected error variant: {:?}",
                err
            );
        }
    }

    // 5. Verify database integrity
    let conn_verify = rusqlite::Connection::open(&db_path).expect("open verify conn");
    let active_variant_count: i64 = conn_verify
        .query_row(
            "SELECT COUNT(*) FROM product_variants WHERE product_id = ?1 AND is_active = 1",
            rusqlite::params![product_id],
            |row| row.get(0),
        )
        .expect("count active variants");
    assert_eq!(
        active_variant_count, 1,
        "Database must contain at most one active variant for the combination"
    );

    let assoc_count: i64 = conn_verify
        .query_row("SELECT COUNT(*) FROM variant_attribute_values", [], |row| {
            row.get(0)
        })
        .expect("count assoc rows");
    assert_eq!(
        assoc_count, 1,
        "Database must contain exactly 1 association row (no partial or orphaned joins)"
    );

    drop(conn_verify);
    let _ = std::fs::remove_file(&db_path);
}
