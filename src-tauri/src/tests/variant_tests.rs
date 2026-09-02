// Comprehensive unit, integration, migration, and contract tests for F2.05 Variants / Matrix.

use crate::auth::middleware::{require_scoped_permission, AuthorizeRequest};
use crate::commands::{authorize_catalog_mutation, authorize_catalog_read};
use crate::permission::Permission;
use crate::product::{create_product, CreateProductInput};
use crate::tests::test_helpers::{
    apply_migrations_up_to, create_test_org_and_branch, create_test_user_with_creds, setup_test_db,
    setup_test_db_up_to,
};
use crate::user::session::create_local_session;
use crate::variant::*;
use rusqlite::params;

fn create_sample_variable_product(conn: &rusqlite::Connection, name: &str) -> String {
    let input = CreateProductInput {
        name: name.to_string(),
        description: None,
        category_id: None,
        sku: None,
        barcode: None,
        product_type: Some("variable".to_string()),
        base_price_minor: 5000,
        cost_price_minor: Some(2500),
        unit_type: None,
        requires_expiry: None,
        requires_serial: None,
        warranty_months: None,
        custom_attributes: None,
    };
    create_product(conn, input)
        .expect("sample variable product created")
        .id
}

// =========================================================================
// 1. MIGRATION TESTS
// =========================================================================

#[test]
fn test_migration_014_applies_cleanly_to_fresh_database() {
    let conn = setup_test_db();

    // Verify product_variants extended columns exist
    let mut stmt = conn
        .prepare("PRAGMA table_info(product_variants)")
        .expect("table info");
    let columns: Vec<String> = stmt
        .query_map([], |row| row.get(1))
        .expect("query map")
        .filter_map(Result::ok)
        .collect();

    assert!(columns.contains(&"price_override_minor".to_string()));
    assert!(columns.contains(&"cost_price_minor".to_string()));
    assert!(columns.contains(&"created_at".to_string()));
    assert!(columns.contains(&"updated_at".to_string()));
    assert!(columns.contains(&"deleted_at".to_string()));

    // Verify attribute_definitions extended columns
    let mut stmt_def = conn
        .prepare("PRAGMA table_info(attribute_definitions)")
        .expect("table info");
    let def_cols: Vec<String> = stmt_def
        .query_map([], |row| row.get(1))
        .expect("query map")
        .filter_map(Result::ok)
        .collect();
    assert!(def_cols.contains(&"sort_order".to_string()));
    assert!(def_cols.contains(&"created_at".to_string()));
}

#[test]
fn test_migration_014_upgrades_from_013_with_representative_data() {
    let conn = setup_test_db_up_to("013_units_conversions_hardening");

    // Seed pre-existing legacy product and variant before 014
    let prod_id = create_sample_variable_product(&conn, "Legacy Product");
    let var_id = "test-legacy-var-1";

    conn.execute(
        "INSERT INTO product_variants (id, product_id, sku, barcode, price_override, is_active)
         VALUES (?1, ?2, 'LEGACY-SKU', '1234567890128', 19.99, 1)",
        params![var_id, prod_id],
    )
    .expect("insert legacy variant");

    // Apply migration 014
    apply_migrations_up_to(&conn, "014_product_variants_hardening");

    // Verify backfilled price_override_minor (19.99 * 100 = 1999 minor units)
    let (price_minor, cost_minor, is_active, created_at): (Option<i64>, Option<i64>, i64, String) =
        conn.query_row(
            "SELECT price_override_minor, cost_price_minor, is_active, created_at FROM product_variants WHERE id = ?1",
            params![var_id],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
        )
        .expect("query backfilled variant");

    assert_eq!(price_minor, Some(1999));
    assert_eq!(cost_minor, None);
    assert_eq!(is_active, 1);
    assert!(!created_at.is_empty());
}

#[test]
fn test_migration_014_repeatability() {
    let conn = setup_test_db();
    // Re-applying unapplied migrations must be a clean no-op
    apply_migrations_up_to(&conn, "014_product_variants_hardening");
}

// =========================================================================
// 2. ATTRIBUTE DEFINITION & VALUE TESTS
// =========================================================================

#[test]
fn test_attribute_definition_validation_and_uniqueness() {
    let conn = setup_test_db();

    // Reject empty
    let err_empty = create_attribute_definition(
        &conn,
        CreateAttributeDefinitionInput {
            name: "   ".into(),
            sort_order: None,
        },
    );
    assert!(matches!(err_empty, Err(VariantError::Validation(_))));

    // Create valid
    let def1 = create_attribute_definition(
        &conn,
        CreateAttributeDefinitionInput {
            name: "Size".into(),
            sort_order: Some(10),
        },
    )
    .expect("created Size");
    assert_eq!(def1.name, "Size");
    assert_eq!(def1.sort_order, 10);

    // Reject case-insensitive duplicate
    let err_dup = create_attribute_definition(
        &conn,
        CreateAttributeDefinitionInput {
            name: "size".into(),
            sort_order: None,
        },
    );
    assert!(matches!(err_dup, Err(VariantError::DuplicateName(_))));
}

#[test]
fn test_attribute_value_validation_and_scoping() {
    let conn = setup_test_db();

    let def_size = create_attribute_definition(
        &conn,
        CreateAttributeDefinitionInput {
            name: "Size".into(),
            sort_order: Some(1),
        },
    )
    .expect("def Size");

    let def_color = create_attribute_definition(
        &conn,
        CreateAttributeDefinitionInput {
            name: "Color".into(),
            sort_order: Some(2),
        },
    )
    .expect("def Color");

    // Value for non-existent definition fails
    let err_not_found = create_attribute_value(
        &conn,
        CreateAttributeValueInput {
            attribute_definition_id: "non-existent-def".into(),
            value: "Large".into(),
            sort_order: None,
        },
    );
    assert!(matches!(err_not_found, Err(VariantError::NotFound(_))));

    // Create valid values
    let val_m = create_attribute_value(
        &conn,
        CreateAttributeValueInput {
            attribute_definition_id: def_size.id.clone(),
            value: "Medium".into(),
            sort_order: Some(2),
        },
    )
    .expect("val Medium");
    assert_eq!(val_m.value, "Medium");

    // Reject duplicate value within same definition
    let err_dup = create_attribute_value(
        &conn,
        CreateAttributeValueInput {
            attribute_definition_id: def_size.id.clone(),
            value: "medium".into(),
            sort_order: None,
        },
    );
    assert!(matches!(err_dup, Err(VariantError::DuplicateValue(_))));

    // Same value name in different definition is allowed
    let val_color_m = create_attribute_value(
        &conn,
        CreateAttributeValueInput {
            attribute_definition_id: def_color.id.clone(),
            value: "Medium".into(),
            sort_order: None,
        },
    )
    .expect("val Medium in Color allowed");
    assert_eq!(val_color_m.attribute_definition_id, def_color.id);
}

// =========================================================================
// 3. VARIANT LIFECYCLE & EXACT MONEY TESTS
// =========================================================================

#[test]
fn test_product_variant_lifecycle_and_exact_money() {
    let conn = setup_test_db();
    let product_id = create_sample_variable_product(&conn, "Cotton Shirt");

    let def_size = create_attribute_definition(
        &conn,
        CreateAttributeDefinitionInput {
            name: "Size".into(),
            sort_order: None,
        },
    )
    .expect("def Size");

    let val_s = create_attribute_value(
        &conn,
        CreateAttributeValueInput {
            attribute_definition_id: def_size.id,
            value: "Small".into(),
            sort_order: None,
        },
    )
    .expect("val Small");

    // 1. Create variant with exact minor units
    let created = create_variant(
        &conn,
        CreateVariantInput {
            product_id: product_id.clone(),
            sku: Some("SHIRT-S".into()),
            barcode: Some("9780201379624".into()),
            price_override_minor: Some(4500),
            cost_price_minor: Some(2000),
            attribute_value_ids: vec![val_s.id.clone()],
        },
    )
    .expect("variant created");

    assert_eq!(created.variant.sku.as_deref(), Some("SHIRT-S"));
    assert_eq!(created.variant.barcode.as_deref(), Some("9780201379624"));
    assert_eq!(created.variant.price_override_minor, Some(4500));
    assert_eq!(created.variant.cost_price_minor, Some(2000));
    assert!(created.variant.is_active);
    assert_eq!(created.variant.deleted_at, None);

    // 2. Active list returns the variant
    let active_list =
        list_variants_by_product(&conn, &product_id, Some(true)).expect("list active");
    assert_eq!(active_list.len(), 1);

    // 3. Update variant
    let updated = update_variant(
        &conn,
        UpdateVariantInput {
            id: created.variant.id.clone(),
            sku: Some("SHIRT-S-V2".into()),
            barcode: Some("9780201379624".into()),
            price_override_minor: Some(4800),
            cost_price_minor: Some(2100),
            is_active: true,
        },
    )
    .expect("updated variant");
    assert_eq!(updated.sku.as_deref(), Some("SHIRT-S-V2"));
    assert_eq!(updated.price_override_minor, Some(4800));

    // 4. Soft-delete variant
    soft_delete_variant(&conn, &created.variant.id).expect("soft deleted");

    // 5. Active list now returns 0
    let active_after_delete =
        list_variants_by_product(&conn, &product_id, Some(true)).expect("list active");
    assert_eq!(active_after_delete.len(), 0);

    // 6. Inactive list returns the soft-deleted variant
    let inactive_list =
        list_variants_by_product(&conn, &product_id, Some(false)).expect("list inactive");
    assert_eq!(inactive_list.len(), 1);
    assert!(!inactive_list[0].is_active);
    assert!(inactive_list[0].deleted_at.is_some());
}

#[test]
fn test_exact_money_validation_and_overflow_rejection() {
    let conn = setup_test_db();
    let product_id = create_sample_variable_product(&conn, "Test Overflow");

    // Negative price rejected
    let err_neg = create_variant(
        &conn,
        CreateVariantInput {
            product_id: product_id.clone(),
            sku: None,
            barcode: None,
            price_override_minor: Some(-100),
            cost_price_minor: None,
            attribute_value_ids: vec![],
        },
    );
    assert!(matches!(err_neg, Err(VariantError::Validation(_))));

    // Exceeding MAX_SAFE_MINOR_UNITS rejected
    let err_overflow = create_variant(
        &conn,
        CreateVariantInput {
            product_id,
            sku: None,
            barcode: None,
            price_override_minor: Some(MAX_SAFE_MINOR_UNITS + 1),
            cost_price_minor: None,
            attribute_value_ids: vec![],
        },
    );
    assert!(matches!(err_overflow, Err(VariantError::Validation(_))));
}

#[test]
fn test_product_variant_multiple_values_for_same_definition_rejected() {
    let conn = setup_test_db();
    let product_id = create_sample_variable_product(&conn, "Dress");

    let def_size = create_attribute_definition(
        &conn,
        CreateAttributeDefinitionInput {
            name: "Size".into(),
            sort_order: None,
        },
    )
    .expect("def Size");

    let val_s = create_attribute_value(
        &conn,
        CreateAttributeValueInput {
            attribute_definition_id: def_size.id.clone(),
            value: "S".into(),
            sort_order: None,
        },
    )
    .expect("val S");

    let val_m = create_attribute_value(
        &conn,
        CreateAttributeValueInput {
            attribute_definition_id: def_size.id,
            value: "M".into(),
            sort_order: None,
        },
    )
    .expect("val M");

    let err = create_variant(
        &conn,
        CreateVariantInput {
            product_id,
            sku: None,
            barcode: None,
            price_override_minor: None,
            cost_price_minor: None,
            attribute_value_ids: vec![val_s.id, val_m.id],
        },
    );

    assert!(matches!(err, Err(VariantError::Validation(_))));
}

// =========================================================================
// 4. ACTIVE COMBINATION UNIQUENESS TESTS
// =========================================================================

#[test]
fn test_active_combination_uniqueness_enforced() {
    let conn = setup_test_db();
    let product_id = create_sample_variable_product(&conn, "Polo");

    let def_size = create_attribute_definition(
        &conn,
        CreateAttributeDefinitionInput {
            name: "Size".into(),
            sort_order: None,
        },
    )
    .expect("def Size");

    let val_l = create_attribute_value(
        &conn,
        CreateAttributeValueInput {
            attribute_definition_id: def_size.id,
            value: "L".into(),
            sort_order: None,
        },
    )
    .expect("val L");

    // First active variant with combination [Size: L] succeeds
    create_variant(
        &conn,
        CreateVariantInput {
            product_id: product_id.clone(),
            sku: Some("POLO-L-1".into()),
            barcode: None,
            price_override_minor: None,
            cost_price_minor: None,
            attribute_value_ids: vec![val_l.id.clone()],
        },
    )
    .expect("first variant created");

    // Second active variant with duplicate combination [Size: L] must be rejected
    let err_dup = create_variant(
        &conn,
        CreateVariantInput {
            product_id,
            sku: Some("POLO-L-2".into()),
            barcode: None,
            price_override_minor: None,
            cost_price_minor: None,
            attribute_value_ids: vec![val_l.id],
        },
    );

    assert!(matches!(
        err_dup,
        Err(VariantError::DuplicateCombination(_))
    ));
}

// =========================================================================
// 5. CARTESIAN PRODUCT & MATRIX GENERATION TESTS
// =========================================================================

#[test]
fn test_cartesian_product_1d_2d_and_3d_generation() {
    // 1D: [S, M, L] -> 3 combinations
    let d1 = vec![vec!["S".to_string(), "M".to_string(), "L".to_string()]];
    let res1 = compute_cartesian_product(&d1);
    assert_eq!(res1.len(), 3);

    // 2D: [S, M] x [Red, Blue] -> 4 combinations
    let d2 = vec![
        vec!["S".to_string(), "M".to_string()],
        vec!["Red".to_string(), "Blue".to_string()],
    ];
    let res2 = compute_cartesian_product(&d2);
    assert_eq!(res2.len(), 4);
    assert_eq!(res2[0], vec!["S", "Red"]);
    assert_eq!(res2[1], vec!["S", "Blue"]);
    assert_eq!(res2[2], vec!["M", "Red"]);
    assert_eq!(res2[3], vec!["M", "Blue"]);

    // 3D: [S, M] x [Red, Blue] x [Cotton, Linen] -> 8 combinations
    let d3 = vec![
        vec!["S".to_string(), "M".to_string()],
        vec!["Red".to_string(), "Blue".to_string()],
        vec!["Cotton".to_string(), "Linen".to_string()],
    ];
    let res3 = compute_cartesian_product(&d3);
    assert_eq!(res3.len(), 8);
}

#[test]
fn test_matrix_preview_side_effect_freedom_adr0007() {
    let conn = setup_test_db();
    let product_id = create_sample_variable_product(&conn, "Sneakers");

    let def_size = create_attribute_definition(
        &conn,
        CreateAttributeDefinitionInput {
            name: "Size".into(),
            sort_order: None,
        },
    )
    .expect("def Size");

    let val_41 = create_attribute_value(
        &conn,
        CreateAttributeValueInput {
            attribute_definition_id: def_size.id.clone(),
            value: "41".into(),
            sort_order: None,
        },
    )
    .expect("val 41");

    let val_42 = create_attribute_value(
        &conn,
        CreateAttributeValueInput {
            attribute_definition_id: def_size.id.clone(),
            value: "42".into(),
            sort_order: None,
        },
    )
    .expect("val 42");

    let input = PreviewMatrixInput {
        product_id: product_id.clone(),
        dimensions: vec![MatrixDimensionInput {
            attribute_definition_id: def_size.id,
            attribute_value_ids: vec![val_41.id, val_42.id],
        }],
    };

    let preview = preview_variant_matrix(&conn, input).expect("preview succeeds");
    assert_eq!(preview.total_combinations, 2);
    assert_eq!(preview.new_combinations_count, 2);
    assert_eq!(preview.existing_combinations_count, 0);

    // Verify ZERO side effects in the database
    let count: i64 = conn
        .query_row("SELECT COUNT(*) FROM product_variants", [], |r| r.get(0))
        .expect("count");
    assert_eq!(
        count, 0,
        "Preview must not write any variants to the database"
    );

    let seq_count: i64 = conn
        .query_row("SELECT COUNT(*) FROM sku_sequences", [], |r| r.get(0))
        .expect("seq count");
    assert_eq!(seq_count, 0, "Preview must not mutate sku_sequences");
}

#[test]
fn test_matrix_generation_preserves_existing_variants_and_is_idempotent() {
    let conn = setup_test_db();
    let product_id = create_sample_variable_product(&conn, "Jacket");

    let def_size = create_attribute_definition(
        &conn,
        CreateAttributeDefinitionInput {
            name: "Size".into(),
            sort_order: None,
        },
    )
    .expect("def Size");

    let val_s = create_attribute_value(
        &conn,
        CreateAttributeValueInput {
            attribute_definition_id: def_size.id.clone(),
            value: "S".into(),
            sort_order: None,
        },
    )
    .expect("val S");

    let val_m = create_attribute_value(
        &conn,
        CreateAttributeValueInput {
            attribute_definition_id: def_size.id.clone(),
            value: "M".into(),
            sort_order: None,
        },
    )
    .expect("val M");

    let dim_input = vec![MatrixDimensionInput {
        attribute_definition_id: def_size.id,
        attribute_value_ids: vec![val_s.id, val_m.id],
    }];

    // First generation: creates 2 new variants
    let res1 = generate_variant_matrix(
        &conn,
        GenerateMatrixInput {
            product_id: product_id.clone(),
            dimensions: dim_input.clone(),
            default_price_override_minor: Some(9900),
            default_cost_price_minor: Some(5000),
            sku_prefix: Some("JKT".into()),
        },
    )
    .expect("matrix gen 1");

    assert_eq!(res1.total_combinations, 2);
    assert_eq!(res1.created_count, 2);
    assert_eq!(res1.existing_count, 0);

    let created_ids: Vec<String> = res1
        .created_variants
        .iter()
        .map(|v| v.variant.id.clone())
        .collect();

    // Second generation with same input: 0 created, 2 existing, exact same IDs preserved (idempotent)
    let res2 = generate_variant_matrix(
        &conn,
        GenerateMatrixInput {
            product_id,
            dimensions: dim_input,
            default_price_override_minor: Some(9900),
            default_cost_price_minor: Some(5000),
            sku_prefix: Some("JKT".into()),
        },
    )
    .expect("matrix gen 2");

    assert_eq!(res2.total_combinations, 2);
    assert_eq!(res2.created_count, 0);
    assert_eq!(res2.existing_count, 2);

    let existing_ids: Vec<String> = res2
        .existing_variants
        .iter()
        .map(|v| v.variant.id.clone())
        .collect();
    assert_eq!(
        created_ids, existing_ids,
        "Existing variant IDs must be preserved"
    );
}

#[test]
fn test_matrix_5000_safety_boundary_and_overflow_rejection() {
    let conn = setup_test_db();
    let product_id = create_sample_variable_product(&conn, "Combinatorial Stress");

    let def1 = create_attribute_definition(
        &conn,
        CreateAttributeDefinitionInput {
            name: "D1".into(),
            sort_order: None,
        },
    )
    .expect("def 1");
    let def2 = create_attribute_definition(
        &conn,
        CreateAttributeDefinitionInput {
            name: "D2".into(),
            sort_order: None,
        },
    )
    .expect("def 2");

    // 100 x 51 = 5,100 combinations (exceeds 5,000 threshold)
    let mut val_ids_1 = Vec::new();
    for i in 0..100 {
        let v = create_attribute_value(
            &conn,
            CreateAttributeValueInput {
                attribute_definition_id: def1.id.clone(),
                value: format!("V1_{i}"),
                sort_order: None,
            },
        )
        .expect("val");
        val_ids_1.push(v.id);
    }

    let mut val_ids_2 = Vec::new();
    for i in 0..51 {
        let v = create_attribute_value(
            &conn,
            CreateAttributeValueInput {
                attribute_definition_id: def2.id.clone(),
                value: format!("V2_{i}"),
                sort_order: None,
            },
        )
        .expect("val");
        val_ids_2.push(v.id);
    }

    let err = preview_variant_matrix(
        &conn,
        PreviewMatrixInput {
            product_id,
            dimensions: vec![
                MatrixDimensionInput {
                    attribute_definition_id: def1.id,
                    attribute_value_ids: val_ids_1,
                },
                MatrixDimensionInput {
                    attribute_definition_id: def2.id,
                    attribute_value_ids: val_ids_2,
                },
            ],
        },
    );

    assert!(matches!(err, Err(VariantError::Validation(msg)) if msg.contains("5000")));
}

// =========================================================================
// 6. SKU DECISION TESTS (ADR-0007 DECISIONS A, B, C)
// =========================================================================

#[test]
fn test_table_local_sku_namespace_equality_adr0007_decision_a() {
    let conn = setup_test_db();

    // 1. Create a product with SKU "ELEC-000001"
    let p_input = CreateProductInput {
        name: "Shared SKU Product".to_string(),
        description: None,
        category_id: None,
        sku: Some("ELEC-000001".to_string()),
        barcode: None,
        product_type: Some("variable".to_string()),
        base_price_minor: 1000,
        cost_price_minor: None,
        unit_type: None,
        requires_expiry: None,
        requires_serial: None,
        warranty_months: None,
        custom_attributes: None,
    };
    let prod = create_product(&conn, p_input).expect("product created with ELEC-000001");
    assert_eq!(prod.sku.as_deref(), Some("ELEC-000001"));

    // 2. Under Decision A (table-local namespace), a Variant MAY also have SKU "ELEC-000001"
    let v = create_variant(
        &conn,
        CreateVariantInput {
            product_id: prod.id,
            sku: Some("ELEC-000001".to_string()),
            barcode: None,
            price_override_minor: None,
            cost_price_minor: None,
            attribute_value_ids: vec![],
        },
    )
    .expect("variant created with same SKU as product under table-local namespace");

    assert_eq!(v.variant.sku.as_deref(), Some("ELEC-000001"));
}

#[test]
fn test_matrix_sku_collision_causes_sequence_advancement_decision_b() {
    let conn = setup_test_db();
    let product_id = create_sample_variable_product(&conn, "Advancement Test");

    let def_size = create_attribute_definition(
        &conn,
        CreateAttributeDefinitionInput {
            name: "Size".into(),
            sort_order: None,
        },
    )
    .expect("def Size");

    let val_s = create_attribute_value(
        &conn,
        CreateAttributeValueInput {
            attribute_definition_id: def_size.id.clone(),
            value: "S".into(),
            sort_order: None,
        },
    )
    .expect("val S");

    // Pre-occupy SKU "ELEC-000001" in product_variants directly
    create_variant(
        &conn,
        CreateVariantInput {
            product_id: product_id.clone(),
            sku: Some("ELEC-000001".to_string()),
            barcode: None,
            price_override_minor: None,
            cost_price_minor: None,
            attribute_value_ids: vec![],
        },
    )
    .expect("pre-occupied variant created");

    // Matrix generation with prefix "ELEC" must detect the variant collision,
    // advance sequence, and allocate "ELEC-000002" without error
    let res = generate_variant_matrix(
        &conn,
        GenerateMatrixInput {
            product_id,
            dimensions: vec![MatrixDimensionInput {
                attribute_definition_id: def_size.id,
                attribute_value_ids: vec![val_s.id],
            }],
            default_price_override_minor: None,
            default_cost_price_minor: None,
            sku_prefix: Some("ELEC".into()),
        },
    )
    .expect("matrix generation advances sequence past occupied variant SKU");

    assert_eq!(res.created_variants.len(), 1);
    assert_eq!(
        res.created_variants[0].variant.sku.as_deref(),
        Some("ELEC-000002"),
        "Generator must advance past occupied variant sequence"
    );
}

#[test]
fn test_archived_variant_sku_remains_reserved_decision_c() {
    let conn = setup_test_db();
    let product_id = create_sample_variable_product(&conn, "Reserved SKU Test");

    // 1. Create variant with SKU "RES-000001"
    let v = create_variant(
        &conn,
        CreateVariantInput {
            product_id: product_id.clone(),
            sku: Some("RES-000001".to_string()),
            barcode: None,
            price_override_minor: None,
            cost_price_minor: None,
            attribute_value_ids: vec![],
        },
    )
    .expect("variant created");

    // 2. Soft-delete the variant
    soft_delete_variant(&conn, &v.variant.id).expect("soft deleted");

    // 3. Attempting to assign "RES-000001" to a new variant must fail (Decision C: remains reserved)
    let err_reuse = create_variant(
        &conn,
        CreateVariantInput {
            product_id,
            sku: Some("RES-000001".to_string()),
            barcode: None,
            price_override_minor: None,
            cost_price_minor: None,
            attribute_value_ids: vec![],
        },
    );

    assert!(matches!(err_reuse, Err(VariantError::DuplicateSku(_))));
}

#[test]
fn test_matrix_sku_allocation_transaction_rollback_on_failure() {
    let conn = setup_test_db();
    let product_id = create_sample_variable_product(&conn, "Rollback Test");

    let def = create_attribute_definition(
        &conn,
        CreateAttributeDefinitionInput {
            name: "Size".into(),
            sort_order: None,
        },
    )
    .expect("def");

    let val = create_attribute_value(
        &conn,
        CreateAttributeValueInput {
            attribute_definition_id: def.id.clone(),
            value: "S".into(),
            sort_order: None,
        },
    )
    .expect("val");

    // Pre-allocate sequence 1 for prefix "ROLL"
    let _ = crate::barcode::generate_next_sku(&conn, Some("ROLL")).expect("allocated 1");

    // Pre-seed a conflicting SKU in product_variants that will fail the bounded retry (Decision B)
    for i in 2..=22 {
        let occupied_sku = format!("ROLL-{i:06}");
        conn.execute(
            "INSERT INTO product_variants (id, product_id, sku, is_active) VALUES (?1, ?2, ?3, 1)",
            params![format!("var-{i}"), product_id, occupied_sku],
        )
        .expect("seed occupied");
    }

    // Matrix generation will exhaust the 20-retry bounded loop and fail
    let err = generate_variant_matrix(
        &conn,
        GenerateMatrixInput {
            product_id,
            dimensions: vec![MatrixDimensionInput {
                attribute_definition_id: def.id,
                attribute_value_ids: vec![val.id],
            }],
            default_price_override_minor: None,
            default_cost_price_minor: None,
            sku_prefix: Some("ROLL".into()),
        },
    );

    assert!(err.is_err(), "Exceeded bounded retries must fail");

    // Verify that because the transaction rolled back, the candidate variant was not created
    let count: i64 = conn
        .query_row("SELECT COUNT(*) FROM variant_attribute_values", [], |r| {
            r.get(0)
        })
        .expect("count");
    assert_eq!(
        count, 0,
        "All matrix variant inserts must roll back on failure"
    );
}

// =========================================================================
// 7. BULK OPERATIONS & SEARCH TESTS
// =========================================================================

#[test]
fn test_bulk_update_variant_status_and_prices_atomicity() {
    let conn = setup_test_db();
    let product_id = create_sample_variable_product(&conn, "Bulk Test");

    let v1 = create_variant(
        &conn,
        CreateVariantInput {
            product_id: product_id.clone(),
            sku: None,
            barcode: None,
            price_override_minor: Some(1000),
            cost_price_minor: Some(500),
            attribute_value_ids: vec![],
        },
    )
    .expect("v1");

    let v2 = create_variant(
        &conn,
        CreateVariantInput {
            product_id,
            sku: None,
            barcode: None,
            price_override_minor: Some(2000),
            cost_price_minor: Some(1000),
            attribute_value_ids: vec![],
        },
    )
    .expect("v2");

    // 1. Bulk update prices
    let price_res = bulk_update_variant_prices(
        &conn,
        BulkUpdateVariantPricesInput {
            variant_ids: vec![v1.variant.id.clone(), v2.variant.id.clone()],
            price_override_minor: Some(3500),
            cost_price_minor: Some(1800),
        },
    )
    .expect("bulk prices updated");
    assert_eq!(price_res.updated_count, 2);

    let v1_updated = get_variant(&conn, &v1.variant.id).unwrap().unwrap();
    assert_eq!(v1_updated.price_override_minor, Some(3500));
    assert_eq!(v1_updated.cost_price_minor, Some(1800));

    // 2. Bulk update status to inactive
    let status_res = bulk_update_variant_status(
        &conn,
        BulkUpdateVariantStatusInput {
            variant_ids: vec![v1.variant.id.clone(), v2.variant.id.clone()],
            is_active: false,
        },
    )
    .expect("bulk status updated");
    assert_eq!(status_res.updated_count, 2);

    let v1_inactive = get_variant(&conn, &v1.variant.id).unwrap().unwrap();
    assert!(!v1_inactive.is_active);
    assert!(v1_inactive.deleted_at.is_some());

    // 3. Atomicity: if any ID in list is invalid, whole bulk update rolls back
    let err_bulk = bulk_update_variant_prices(
        &conn,
        BulkUpdateVariantPricesInput {
            variant_ids: vec![v1.variant.id.clone(), "non-existent-id".into()],
            price_override_minor: Some(9999),
            cost_price_minor: None,
        },
    );
    assert!(matches!(err_bulk, Err(VariantError::NotFound(_))));

    // Price of v1 must NOT have changed to 9999
    let v1_check = get_variant(&conn, &v1.variant.id).unwrap().unwrap();
    assert_eq!(v1_check.price_override_minor, Some(3500));
}

#[test]
fn test_variant_barcode_and_sku_lookup_and_search() {
    let conn = setup_test_db();
    let product_id = create_sample_variable_product(&conn, "Search Product");

    let def_color = create_attribute_definition(
        &conn,
        CreateAttributeDefinitionInput {
            name: "Color".into(),
            sort_order: None,
        },
    )
    .expect("def Color");

    let val_crimson = create_attribute_value(
        &conn,
        CreateAttributeValueInput {
            attribute_definition_id: def_color.id,
            value: "Crimson".into(),
            sort_order: None,
        },
    )
    .expect("val Crimson");

    let v = create_variant(
        &conn,
        CreateVariantInput {
            product_id: product_id.clone(),
            sku: Some("CRIMSON-01".into()),
            barcode: Some("9780201379624".into()),
            price_override_minor: None,
            cost_price_minor: None,
            attribute_value_ids: vec![val_crimson.id],
        },
    )
    .expect("variant");

    // Lookup by barcode
    let by_bc = get_variant_by_barcode(&conn, "9780201379624").expect("lookup barcode");
    assert!(by_bc.is_some());
    assert_eq!(by_bc.unwrap().variant.id, v.variant.id);

    // Lookup by SKU
    let by_sku = get_variant_by_sku(&conn, "CRIMSON-01").expect("lookup sku");
    assert!(by_sku.is_some());
    assert_eq!(by_sku.unwrap().variant.id, v.variant.id);

    // Search by attribute value "Crimson"
    let search_res = search_variants(&conn, Some(&product_id), "Crimson").expect("search");
    assert_eq!(search_res.len(), 1);
    assert_eq!(search_res[0].variant.id, v.variant.id);
}

// =========================================================================
// 8. AUTHORIZATION TESTS
// =========================================================================

#[test]
fn test_variant_catalog_authorization() {
    let conn = setup_test_db();
    let (org_id, branch_id) = create_test_org_and_branch(&conn);

    // User with ProductsManage permission
    let user_mgr = create_test_user_with_creds(
        &conn,
        &branch_id,
        "manager",
        "Manager User",
        "mgr_pass_123",
        "manager",
    );
    let session_mgr =
        create_local_session(&conn, &user_mgr.id, &branch_id, None, None, None).expect("session");

    // User without ProductsManage permission (Cashier)
    let user_cashier = create_test_user_with_creds(
        &conn,
        &branch_id,
        "cashier",
        "Cashier User",
        "cashier_pass_123",
        "cashier",
    );
    let session_cashier =
        create_local_session(&conn, &user_cashier.id, &branch_id, None, None, None)
            .expect("session");

    // Manager can mutate catalog
    assert!(authorize_catalog_mutation(&conn, &session_mgr.id).is_ok());

    // Cashier cannot mutate catalog
    assert!(authorize_catalog_mutation(&conn, &session_cashier.id).is_err());

    // Both can read catalog
    assert!(authorize_catalog_read(&conn, &session_mgr.id).is_ok());
    assert!(authorize_catalog_read(&conn, &session_cashier.id).is_ok());
}
