// Integration and unit tests for F2.06 — Weighted Products (ADR-0008).

use crate::commands::weighted::{
    calculate_weighted_item_impl, delete_product_weight_config_impl,
    get_product_weight_config_impl, set_product_weight_config_impl,
};
use crate::product::{create_product, CreateProductInput};
use crate::tests::test_helpers::{
    apply_migrations_up_to, create_test_org_and_branch, create_test_user_with_creds, setup_test_db,
    setup_test_db_up_to,
};
use crate::user::session::create_local_session;
use crate::weighted::{
    calculate_weighted_item, calculate_weighted_price, deduct_tare, delete_product_weight_config,
    get_product_weight_config, is_product_weighted, normalize_metric_mass_quantity_milli,
    upsert_product_weight_config, validate_weight_bounds, validate_weighted_product_unit,
    UpsertWeightConfigInput, WeightedError,
};
use rusqlite::{params, Connection};

fn make_test_product(
    conn: &Connection,
    name: &str,
    product_type: &str,
    unit_type: Option<&str>,
    price_minor: i64,
) -> String {
    let p = create_product(
        conn,
        CreateProductInput {
            name: name.to_string(),
            description: None,
            category_id: None,
            sku: None,
            barcode: None,
            product_type: Some(product_type.to_string()),
            base_price_minor: price_minor,
            cost_price_minor: None,
            unit_type: unit_type.map(ToString::to_string),
            requires_expiry: None,
            requires_serial: None,
            warranty_months: None,
            custom_attributes: None,
        },
    )
    .expect("create test product");
    p.id
}

// =========================================================================
// 1. PURE ARITHMETIC & INVARIANT UNIT TESTS
// =========================================================================

#[test]
fn test_normal_tare_subtraction() {
    let net = deduct_tare(1250, 50).expect("deduct tare");
    assert_eq!(net, 1200);
}

#[test]
fn test_zero_tare_subtraction() {
    let net = deduct_tare(1250, 0).expect("zero tare");
    assert_eq!(net, 1250);
}

#[test]
fn test_gross_equals_tare() {
    let net = deduct_tare(50, 50).expect("gross equals tare");
    assert_eq!(net, 0);
}

#[test]
fn test_gross_less_than_tare_rejection() {
    let err = deduct_tare(40, 50).unwrap_err();
    assert!(matches!(err, WeightedError::NegativeWeight(_)));
}

#[test]
fn test_negative_tare_rejection() {
    let err = deduct_tare(100, -10).unwrap_err();
    assert!(matches!(err, WeightedError::Validation(_)));
}

#[test]
fn test_negative_gross_rejection() {
    let err = deduct_tare(-50, 10).unwrap_err();
    assert!(matches!(err, WeightedError::Validation(_)));
}

#[test]
fn test_negative_net_weight_in_price_calc_rejection() {
    let err = calculate_weighted_price(-10, 1000).unwrap_err();
    assert!(matches!(err, WeightedError::NegativeWeight(_)));
}

#[test]
fn test_negative_unit_price_in_price_calc_rejection() {
    let err = calculate_weighted_price(1000, -50).unwrap_err();
    assert!(matches!(err, WeightedError::Validation(_)));
}

#[test]
fn test_kg_pricing_calculation() {
    // 1.250 kg at 10.00 minor/kg (1000 minor) = 1250 minor
    let p1 = calculate_weighted_price(1250, 1000).expect("calc price");
    assert_eq!(p1, 1250);

    // 1.400 kg at 1.99 minor/kg (199 minor) = 278.6 -> 279 minor ($2.79)
    let p2 = calculate_weighted_price(1400, 199).expect("calc price");
    assert_eq!(p2, 279);
}

#[test]
fn test_g_pricing_calculation() {
    // 2.500 g at 8.00 minor/g (800 minor) = 20.00 minor (2000 minor)
    let p1 = calculate_weighted_price(2500, 800).expect("calc price");
    assert_eq!(p1, 2000);

    // 3.000 g at 8.50 minor/g (850 minor) = 25.50 minor (2550 minor)
    let p2 = calculate_weighted_price(3000, 850).expect("calc price");
    assert_eq!(p2, 2550);
}

#[test]
fn test_exact_half_up_rounding_boundaries() {
    // Exact tie at .5: 125 * 100 = 12500 + 500 = 13000 / 1000 = 13
    assert_eq!(calculate_weighted_price(125, 100).expect("tie up"), 13);

    // Just below tie: 124 * 100 = 12400 + 500 = 12900 / 1000 = 12
    assert_eq!(calculate_weighted_price(124, 100).expect("below tie"), 12);

    // 335 * 100 = 33500 + 500 = 34000 / 1000 = 34
    assert_eq!(calculate_weighted_price(335, 100).expect("round up"), 34);

    // 334 * 100 = 33400 + 500 = 33900 / 1000 = 33
    assert_eq!(calculate_weighted_price(334, 100).expect("round down"), 33);

    // Smallest increment to round up: 1 * 500 = 500 + 500 = 1000 / 1000 = 1
    assert_eq!(calculate_weighted_price(1, 500).expect("min round up"), 1);

    // Smallest increment below half: 1 * 499 = 499 + 500 = 999 / 1000 = 0
    assert_eq!(calculate_weighted_price(1, 499).expect("min round down"), 0);
}

#[test]
fn test_arithmetic_overflow_rejection() {
    // Multiplication overflow with i64::MAX
    let err_mul = calculate_weighted_price(i64::MAX, 2).unwrap_err();
    assert!(matches!(err_mul, WeightedError::Overflow(_)));

    // Addition overflow with near-MAX product
    let err_add = calculate_weighted_price(i64::MAX - 100, 1).unwrap_err();
    assert!(matches!(err_add, WeightedError::Overflow(_)));

    // Tare deduction overflow
    let err_sub = deduct_tare(i64::MIN, 1).unwrap_err();
    assert!(matches!(err_sub, WeightedError::Validation(_)));
}

#[test]
fn test_exact_cross_unit_normalization() {
    // Identity kg -> kg: 1.250 kg = 1,250 milli-kg
    assert_eq!(
        normalize_metric_mass_quantity_milli(1250, "kg", "kg").expect("kg-kg"),
        1250
    );

    // Identity g -> g: 2.500 g = 2,500 milli-g
    assert_eq!(
        normalize_metric_mass_quantity_milli(2500, "g", "g").expect("g-g"),
        2500
    );

    // Whole grams in milli-g to kg pricing milli-units: 1,250 g = 1,250,000 milli-g = 1,250 milli-kg
    assert_eq!(
        normalize_metric_mass_quantity_milli(1_250_000, "g", "kg").expect("g-kg"),
        1250
    );

    // Sub-gram fractional remainder in milli-g to kg pricing rejected to prevent loss of exactness
    let err_subgram = normalize_metric_mass_quantity_milli(1250, "g", "kg").unwrap_err();
    assert!(matches!(err_subgram, WeightedError::Validation(_)));

    // Kg milli-units to g pricing milli-units: 2 milli-kg = 2 grams = 2,000 milli-g
    assert_eq!(
        normalize_metric_mass_quantity_milli(2, "kg", "g").expect("kg-g"),
        2000
    );

    // Negative measured quantity rejected
    let err_neg = normalize_metric_mass_quantity_milli(-50, "kg", "kg").unwrap_err();
    assert!(matches!(err_neg, WeightedError::NegativeWeight(_)));

    // Unsupported unit conversion rejected
    let err = normalize_metric_mass_quantity_milli(100, "lb", "kg").unwrap_err();
    assert!(matches!(err, WeightedError::Validation(_)));
}

#[test]
fn test_weight_bounds_validation() {
    assert!(validate_weight_bounds(500, Some(100), Some(1000)).is_ok());
    assert!(validate_weight_bounds(100, Some(100), Some(1000)).is_ok());
    assert!(validate_weight_bounds(1000, Some(100), Some(1000)).is_ok());

    let err_low = validate_weight_bounds(50, Some(100), Some(1000)).unwrap_err();
    assert!(matches!(err_low, WeightedError::WeightOutOfBounds { .. }));

    let err_high = validate_weight_bounds(1500, Some(100), Some(1000)).unwrap_err();
    assert!(matches!(err_high, WeightedError::WeightOutOfBounds { .. }));
}

// =========================================================================
// 2. UNIT DIMENSION & CAPABILITY INVARIANT TESTS
// =========================================================================

#[test]
fn test_mass_dimension_acceptance() {
    let conn = setup_test_db();
    let p_kg = make_test_product(&conn, "Bulk Apples", "weighted", Some("kg"), 250);
    let p_g = make_test_product(&conn, "Bulk Tea", "weighted", Some("g"), 1500);

    assert_eq!(
        validate_weighted_product_unit(&conn, &p_kg).expect("valid kg unit"),
        "kg"
    );
    assert_eq!(
        validate_weighted_product_unit(&conn, &p_g).expect("valid g unit"),
        "g"
    );
}

#[test]
fn test_count_dimension_rejection() {
    let conn = setup_test_db();
    let p_piece = make_test_product(&conn, "Discrete Mug", "weighted", Some("piece"), 500);

    let err = validate_weighted_product_unit(&conn, &p_piece).unwrap_err();
    assert!(matches!(
        err,
        WeightedError::InvalidUnitDimension { dimension, .. } if dimension == "count"
    ));
}

#[test]
fn test_volume_dimension_rejection() {
    let conn = setup_test_db();
    let p_vol = make_test_product(&conn, "Bulk Milk", "weighted", Some("L"), 300);

    let err = validate_weighted_product_unit(&conn, &p_vol).unwrap_err();
    assert!(matches!(
        err,
        WeightedError::InvalidUnitDimension { dimension, .. } if dimension == "volume"
    ));
}

#[test]
fn test_unsupported_mass_unit_rejection() {
    let conn = setup_test_db();
    // Non-canonical mass units (e.g. "oz", "lb") exist in units catalog with dimension 'mass',
    // but F2.06 exact integer pricing requires canonical metric mass units ('kg' or 'g').
    let p_oz = make_test_product(&conn, "Bulk Ounces", "weighted", Some("oz"), 150);
    let err_oz = validate_weighted_product_unit(&conn, &p_oz).unwrap_err();
    assert!(
        matches!(err_oz, WeightedError::Validation(msg) if msg.contains("Unsupported mass unit"))
    );

    let p_lb = make_test_product(&conn, "Bulk Pounds", "weighted", Some("lb"), 450);
    let err_lb = validate_weighted_product_unit(&conn, &p_lb).unwrap_err();
    assert!(
        matches!(err_lb, WeightedError::Validation(msg) if msg.contains("Unsupported mass unit"))
    );
}

#[test]
fn test_missing_unit_rejection() {
    let conn = setup_test_db();
    let p_none = make_test_product(&conn, "Mystery Weighted", "weighted", None, 100);

    let err = validate_weighted_product_unit(&conn, &p_none).unwrap_err();
    assert!(matches!(err, WeightedError::MissingUnit(_)));
}

#[test]
fn test_weight_capability_invariant() {
    let conn = setup_test_db();
    // Create product with product_type = 'simple'
    let p_id = make_test_product(&conn, "Capability Produce", "simple", Some("kg"), 400);

    assert!(
        !is_product_weighted(&conn, &p_id).expect("check weighted"),
        "product is not weighted yet"
    );

    // Link capability 'WEIGHT' in product_capabilities
    let cap_id: String = conn
        .query_row(
            "SELECT id FROM capabilities WHERE code = 'WEIGHT'",
            [],
            |row| row.get(0),
        )
        .expect("weight capability exists");

    conn.execute(
        "INSERT INTO product_capabilities (product_id, capability_id, enabled) VALUES (?1, ?2, 1)",
        params![p_id, cap_id],
    )
    .expect("link weight capability");

    assert!(
        is_product_weighted(&conn, &p_id).expect("check weighted"),
        "product is now weighted via capability"
    );
    assert_eq!(
        validate_weighted_product_unit(&conn, &p_id).expect("unit valid"),
        "kg"
    );
}

// =========================================================================
// 3. DATABASE MIGRATION & REPOSITORY TESTS
// =========================================================================

#[test]
fn test_migration_015_fresh_application() {
    let conn = setup_test_db();

    // Verify table exists
    let exists: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM sqlite_master WHERE type = 'table' AND name = 'product_weight_configs'",
            [],
            |row| row.get(0),
        )
        .expect("query table");
    assert_eq!(exists, 1);

    // Verify columns
    for col in [
        "product_id",
        "default_tare_milli",
        "min_weight_milli",
        "max_weight_milli",
        "created_at",
        "updated_at",
    ] {
        let count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM pragma_table_info('product_weight_configs') WHERE name = ?1",
                params![col],
                |row| row.get(0),
            )
            .expect("pragma query");
        assert_eq!(count, 1, "column {col} must exist");
    }
}

#[test]
fn test_migration_upgrade_014_to_015() {
    let conn = setup_test_db_up_to("014_product_variants_hardening");

    // Seed existing product on 014
    let pid = make_test_product(&conn, "Legacy Product", "weighted", Some("kg"), 200);

    // Apply migration 015
    apply_migrations_up_to(&conn, "015_weighted_products");

    // Verify table is queryable and product foreign key is intact
    let config = upsert_product_weight_config(
        &conn,
        &UpsertWeightConfigInput {
            product_id: pid.clone(),
            default_tare_milli: Some(25),
            min_weight_milli: Some(50),
            max_weight_milli: Some(5000),
        },
    )
    .expect("upsert on upgraded db");

    assert_eq!(config.product_id, pid);
    assert_eq!(config.default_tare_milli, 25);
}

#[test]
fn test_configuration_persistence_and_retrieval() {
    let conn = setup_test_db();
    let pid = make_test_product(&conn, "Oranges", "weighted", Some("kg"), 300);

    let created = upsert_product_weight_config(
        &conn,
        &UpsertWeightConfigInput {
            product_id: pid.clone(),
            default_tare_milli: Some(40),
            min_weight_milli: Some(100),
            max_weight_milli: Some(10000),
        },
    )
    .expect("upsert config");

    assert_eq!(created.product_id, pid);
    assert_eq!(created.default_tare_milli, 40);
    assert_eq!(created.min_weight_milli, Some(100));
    assert_eq!(created.max_weight_milli, Some(10000));

    let fetched = get_product_weight_config(&conn, &pid)
        .expect("get config")
        .expect("found");
    assert_eq!(fetched, created);
}

#[test]
fn test_configuration_upsert_updates_existing() {
    let conn = setup_test_db();
    let pid = make_test_product(&conn, "Grapes", "weighted", Some("kg"), 450);

    upsert_product_weight_config(
        &conn,
        &UpsertWeightConfigInput {
            product_id: pid.clone(),
            default_tare_milli: Some(20),
            min_weight_milli: None,
            max_weight_milli: None,
        },
    )
    .expect("initial insert");

    let updated = upsert_product_weight_config(
        &conn,
        &UpsertWeightConfigInput {
            product_id: pid.clone(),
            default_tare_milli: Some(35),
            min_weight_milli: Some(50),
            max_weight_milli: Some(5000),
        },
    )
    .expect("update existing");

    assert_eq!(updated.default_tare_milli, 35);
    assert_eq!(updated.min_weight_milli, Some(50));
    assert_eq!(updated.max_weight_milli, Some(5000));

    let count: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM product_weight_configs WHERE product_id = ?1",
            params![pid],
            |row| row.get(0),
        )
        .expect("count rows");
    assert_eq!(count, 1, "upsert must not create duplicate rows");
}

#[test]
fn test_weight_bounds_and_tare_constraints() {
    let conn = setup_test_db();
    let pid = make_test_product(&conn, "Potatoes", "weighted", Some("kg"), 120);

    // Negative tare rejected by validation
    let err_tare = upsert_product_weight_config(
        &conn,
        &UpsertWeightConfigInput {
            product_id: pid.clone(),
            default_tare_milli: Some(-10),
            min_weight_milli: None,
            max_weight_milli: None,
        },
    )
    .unwrap_err();
    assert!(matches!(err_tare, WeightedError::Validation(_)));

    // min > max rejected by validation
    let err_bounds = upsert_product_weight_config(
        &conn,
        &UpsertWeightConfigInput {
            product_id: pid.clone(),
            default_tare_milli: Some(0),
            min_weight_milli: Some(500),
            max_weight_milli: Some(200),
        },
    )
    .unwrap_err();
    assert!(matches!(err_bounds, WeightedError::Validation(_)));

    // SQLite check constraint also enforces min <= max on direct insert
    let sql_err = conn.execute(
        "INSERT INTO product_weight_configs (product_id, default_tare_milli, min_weight_milli, max_weight_milli)
         VALUES (?1, 0, 500, 200)",
        params![pid],
    );
    assert!(
        sql_err.is_err(),
        "SQLite check constraint must reject min > max"
    );
}

#[test]
fn test_cascade_delete_removes_weight_config() {
    let conn = setup_test_db();
    let pid = make_test_product(&conn, "Watermelon", "weighted", Some("kg"), 80);

    upsert_product_weight_config(
        &conn,
        &UpsertWeightConfigInput {
            product_id: pid.clone(),
            default_tare_milli: Some(50),
            min_weight_milli: None,
            max_weight_milli: None,
        },
    )
    .expect("upsert config");

    // Delete product
    conn.execute("DELETE FROM products WHERE id = ?1", params![pid])
        .expect("delete product");

    let cfg = get_product_weight_config(&conn, &pid).expect("query config");
    assert!(cfg.is_none(), "weight config must be cascade-deleted");
}

#[test]
fn test_explicit_delete_product_weight_config() {
    let conn = setup_test_db();
    let pid = make_test_product(&conn, "Cherries", "weighted", Some("kg"), 600);

    upsert_product_weight_config(
        &conn,
        &UpsertWeightConfigInput {
            product_id: pid.clone(),
            default_tare_milli: Some(10),
            min_weight_milli: None,
            max_weight_milli: None,
        },
    )
    .expect("upsert config");

    delete_product_weight_config(&conn, &pid).expect("delete config");

    let cfg = get_product_weight_config(&conn, &pid).expect("query config");
    assert!(cfg.is_none());

    // Deleting non-existent config returns NotFound
    let err = delete_product_weight_config(&conn, &pid).unwrap_err();
    assert!(matches!(err, WeightedError::NotFound(_)));
}

// =========================================================================
// 4. ITEM CALCULATION INTEGRATION TESTS
// =========================================================================

#[test]
fn test_calculate_weighted_item_uses_default_tare() {
    let conn = setup_test_db();
    let pid = make_test_product(&conn, "Pears", "weighted", Some("kg"), 350); // $3.50/kg

    upsert_product_weight_config(
        &conn,
        &UpsertWeightConfigInput {
            product_id: pid.clone(),
            default_tare_milli: Some(50), // 50g container
            min_weight_milli: None,
            max_weight_milli: None,
        },
    )
    .expect("upsert config");

    // Gross: 1250g, Default tare: 50g -> Net: 1200g. Price: 1200 * 350 / 1000 = 420 minor ($4.20)
    let res = calculate_weighted_item(&conn, &pid, 1250, None, None).expect("calc item");
    assert_eq!(res.gross_weight_milli, 1250);
    assert_eq!(res.tare_weight_milli, 50);
    assert_eq!(res.net_weight_milli, 1200);
    assert_eq!(res.unit_price_minor, 350);
    assert_eq!(res.total_price_minor, 420);
}

#[test]
fn test_calculate_weighted_item_custom_tare_overrides_default() {
    let conn = setup_test_db();
    let pid = make_test_product(&conn, "Peaches", "weighted", Some("kg"), 300);

    upsert_product_weight_config(
        &conn,
        &UpsertWeightConfigInput {
            product_id: pid.clone(),
            default_tare_milli: Some(50),
            min_weight_milli: None,
            max_weight_milli: None,
        },
    )
    .expect("upsert config");

    // Custom tare 100g overrides default 50g: Gross 1250g, Tare 100g -> Net 1150g. Price: 1150 * 300 / 1000 = 345 minor
    let res = calculate_weighted_item(&conn, &pid, 1250, Some(100), None).expect("calc item");
    assert_eq!(res.tare_weight_milli, 100);
    assert_eq!(res.net_weight_milli, 1150);
    assert_eq!(res.total_price_minor, 345);
}

#[test]
fn test_calculate_weighted_item_enforces_bounds() {
    let conn = setup_test_db();
    let pid = make_test_product(&conn, "Heavy Watermelon", "weighted", Some("kg"), 100);

    upsert_product_weight_config(
        &conn,
        &UpsertWeightConfigInput {
            product_id: pid.clone(),
            default_tare_milli: Some(0),
            min_weight_milli: Some(500),
            max_weight_milli: Some(2000),
        },
    )
    .expect("upsert config");

    // Gross 300g < min 500g
    let err_min = calculate_weighted_item(&conn, &pid, 300, None, None).unwrap_err();
    assert!(matches!(err_min, WeightedError::WeightOutOfBounds { .. }));

    // Gross 2500g > max 2000g
    let err_max = calculate_weighted_item(&conn, &pid, 2500, None, None).unwrap_err();
    assert!(matches!(err_max, WeightedError::WeightOutOfBounds { .. }));

    // Gross 1500g is valid
    assert!(calculate_weighted_item(&conn, &pid, 1500, None, None).is_ok());
}

#[test]
fn test_calculate_weighted_item_cross_unit_normalized() {
    let conn = setup_test_db();
    let pid = make_test_product(&conn, "Bulk Coffee", "weighted", Some("kg"), 2000); // $20.00/kg

    // Gross 500,000 milli-g (500g = 0.500 kg), Tare None -> Net 500 milli-kg. Price: 500 * 2000 / 1000 = 1000 minor ($10.00)
    let res =
        calculate_weighted_item(&conn, &pid, 500_000, None, Some("g")).expect("calc cross unit");
    assert_eq!(res.gross_weight_milli, 500_000);
    assert_eq!(res.net_weight_milli, 500);
    assert_eq!(res.unit_price_minor, 2000);
    assert_eq!(res.total_price_minor, 1000);
}

#[test]
fn test_product_id_whitespace_trimming() {
    let conn = setup_test_db();
    let pid = make_test_product(&conn, "Trimmed Produce", "weighted", Some("kg"), 200);
    let padded_pid = format!("  {pid}  ");

    // Config upsert with padded product_id succeeds and normalizes key
    let cfg = upsert_product_weight_config(
        &conn,
        &UpsertWeightConfigInput {
            product_id: padded_pid.clone(),
            default_tare_milli: Some(25),
            min_weight_milli: None,
            max_weight_milli: None,
        },
    )
    .expect("upsert with padded id");
    assert_eq!(cfg.product_id, pid);

    // Query with padded product_id succeeds
    let fetched = get_product_weight_config(&conn, &padded_pid)
        .expect("fetch with padded id")
        .expect("found");
    assert_eq!(fetched.default_tare_milli, 25);

    // Calculation with padded product_id succeeds
    let calc =
        calculate_weighted_item(&conn, &padded_pid, 1025, None, None).expect("calc with padded id");
    assert_eq!(calc.net_weight_milli, 1000);
    assert_eq!(calc.total_price_minor, 200);

    // Delete with padded product_id succeeds
    delete_product_weight_config(&conn, &padded_pid).expect("delete with padded id");
}

// =========================================================================
// 5. TAURI IPC COMMAND & AUTHORIZATION TESTS
// =========================================================================

#[test]
fn test_weighted_commands_authorization_manager_allowed() {
    let conn = setup_test_db();
    let (_, branch_id) = create_test_org_and_branch(&conn);

    let user_manager = create_test_user_with_creds(
        &conn,
        &branch_id,
        "Produce Manager",
        Some("produce_mgr"),
        None,
        None,
        "manager",
    )
    .expect("manager created");

    let session_mgr = create_local_session(&conn, &user_manager.id, &branch_id, "pin", None)
        .expect("mgr session");

    let user_cashier = create_test_user_with_creds(
        &conn,
        &branch_id,
        "Produce Cashier",
        Some("produce_cashier"),
        None,
        None,
        "cashier",
    )
    .expect("cashier created");

    let session_cashier = create_local_session(&conn, &user_cashier.id, &branch_id, "pin", None)
        .expect("cashier session");

    let pid = make_test_product(&conn, "Command Bananas", "weighted", Some("kg"), 200);

    let input = UpsertWeightConfigInput {
        product_id: pid.clone(),
        default_tare_milli: Some(30),
        min_weight_milli: Some(100),
        max_weight_milli: Some(5000),
    };

    // 1. Cashier cannot mutate weight config (fails authorization)
    let unauth_err =
        set_product_weight_config_impl(&conn, &session_cashier.id, &input).unwrap_err();
    assert!(unauth_err.contains("permission") || unauth_err.contains("unauthorized"));

    // 2. Manager can set weight config
    let config = set_product_weight_config_impl(&conn, &session_mgr.id, &input)
        .expect("manager can set config");
    assert_eq!(config.default_tare_milli, 30);

    // 3. Manager and Cashier can read weight config (both have catalog read)
    let fetched = get_product_weight_config_impl(&conn, &session_mgr.id, &pid)
        .expect("manager can read config")
        .expect("found");
    assert_eq!(fetched.default_tare_milli, 30);

    let fetched_cashier = get_product_weight_config_impl(&conn, &session_cashier.id, &pid)
        .expect("cashier can read config")
        .expect("found");
    assert_eq!(fetched_cashier.default_tare_milli, 30);

    // 4. Calculate weighted item under session
    let calc = calculate_weighted_item_impl(&conn, &session_mgr.id, &pid, 1030, None, None)
        .expect("manager can calculate");
    assert_eq!(calc.net_weight_milli, 1000);
    assert_eq!(calc.total_price_minor, 200);

    // 5. Manager can delete weight config
    delete_product_weight_config_impl(&conn, &session_mgr.id, &pid)
        .expect("manager can delete config");
}

#[test]
fn test_weighted_commands_unauthenticated_session_denied() {
    let conn = setup_test_db();
    let (_, _branch_id) = create_test_org_and_branch(&conn);
    let pid = make_test_product(&conn, "Secured Apples", "weighted", Some("kg"), 250);

    let input = UpsertWeightConfigInput {
        product_id: pid.clone(),
        default_tare_milli: Some(10),
        min_weight_milli: None,
        max_weight_milli: None,
    };

    let err =
        set_product_weight_config_impl(&conn, "invalid_session_token_12345", &input).unwrap_err();

    assert!(err.contains("session") || err.contains("unauthorized") || err.contains("permission"));
}
