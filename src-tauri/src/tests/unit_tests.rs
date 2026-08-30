use crate::tests::test_helpers::{
    create_test_org_and_branch, create_test_user_with_creds, setup_test_db,
};
use crate::unit::{
    convert_quantity, create_unit, create_unit_conversion, delete_unit, delete_unit_conversion,
    find_conversion_factor, get_unit, get_unit_by_code, list_unit_conversions, list_units,
    update_unit, validate_multiplier, validate_unit_code, validate_unit_name,
    validate_unit_precision, ConvertQuantityInput, CreateUnitConversionInput, CreateUnitInput,
    UnitDimension, UnitError, UnitFilter, UpdateUnitInput,
};
use crate::user::session::create_local_session;

// =========================================================================
// 1. VALIDATION & DIMENSION PARSING TESTS
// =========================================================================

#[test]
fn test_unit_code_validation_valid() {
    assert_eq!(validate_unit_code("kg").unwrap(), "kg");
    assert_eq!(validate_unit_code("  piece  ").unwrap(), "piece");
    assert_eq!(validate_unit_code("box-12").unwrap(), "box-12");
    assert_eq!(validate_unit_code("pack_6").unwrap(), "pack_6");
    assert_eq!(validate_unit_code("fl_oz").unwrap(), "fl_oz");
    assert_eq!(validate_unit_code("m2").unwrap(), "m2");
    assert_eq!(validate_unit_code("m/s").unwrap(), "m/s");
    assert_eq!(validate_unit_code("100%").unwrap(), "100%");
}

#[test]
fn test_unit_code_validation_invalid() {
    assert!(validate_unit_code("").is_err());
    assert!(validate_unit_code("   ").is_err());
    assert!(validate_unit_code("a".repeat(33).as_str()).is_err());
    assert!(validate_unit_code("kg with space").is_err());
    assert!(validate_unit_code("kg@item").is_err());
    assert!(validate_unit_code("kg#1").is_err());
}

#[test]
fn test_unit_name_validation() {
    assert_eq!(validate_unit_name("Kilogram").unwrap(), "Kilogram");
    assert_eq!(validate_unit_name("  Box of 12  ").unwrap(), "Box of 12");
    assert!(validate_unit_name("").is_err());
    assert!(validate_unit_name("   ").is_err());
    assert!(validate_unit_name("a".repeat(129).as_str()).is_err());
}

#[test]
fn test_unit_precision_validation() {
    assert_eq!(validate_unit_precision(0).unwrap(), 0);
    assert_eq!(validate_unit_precision(3).unwrap(), 3);
    assert_eq!(validate_unit_precision(6).unwrap(), 6);
    assert!(validate_unit_precision(7).is_err());
    assert!(validate_unit_precision(100).is_err());
}

fn assert_approx(actual: f64, expected: f64) {
    assert!(
        (actual - expected).abs() < 1e-6,
        "Expected {expected}, got {actual}"
    );
}

#[test]
fn test_multiplier_validation() {
    assert_approx(validate_multiplier(1.0).unwrap(), 1.0);
    assert_approx(validate_multiplier(1000.0).unwrap(), 1000.0);
    assert_approx(validate_multiplier(0.001).unwrap(), 0.001);
    assert!(validate_multiplier(0.0).is_err());
    assert!(validate_multiplier(-5.0).is_err());
    assert!(validate_multiplier(f64::NAN).is_err());
    assert!(validate_multiplier(f64::INFINITY).is_err());
    assert!(validate_multiplier(f64::NEG_INFINITY).is_err());
}

#[test]
fn test_unit_dimension_parsing() {
    assert_eq!(UnitDimension::parse("count").unwrap(), UnitDimension::Count);
    assert_eq!(UnitDimension::parse("MASS").unwrap(), UnitDimension::Mass);
    assert_eq!(
        UnitDimension::parse("Volume").unwrap(),
        UnitDimension::Volume
    );
    assert_eq!(
        UnitDimension::parse("length").unwrap(),
        UnitDimension::Length
    );
    assert_eq!(UnitDimension::parse("area").unwrap(), UnitDimension::Area);
    assert_eq!(
        UnitDimension::parse("custom").unwrap(),
        UnitDimension::Custom
    );
    assert!(UnitDimension::parse("invalid_dimension").is_err());
}

// =========================================================================
// 2. REPOSITORY CRUD TESTS — UNITS
// =========================================================================

#[test]
fn test_create_and_get_unit() {
    let conn = setup_test_db();

    let input = CreateUnitInput {
        code: "tray".to_string(),
        name: "Tray of Eggs".to_string(),
        dimension: "count".to_string(),
        precision: Some(0),
        is_base: Some(false),
    };

    let unit = create_unit(&conn, input).expect("unit created");
    assert_eq!(unit.code, "tray");
    assert_eq!(unit.name, "Tray of Eggs");
    assert_eq!(unit.dimension, UnitDimension::Count);
    assert_eq!(unit.precision, 0);
    assert!(!unit.is_base);

    let fetched = get_unit(&conn, &unit.id)
        .unwrap()
        .expect("unit fetched by id");
    assert_eq!(fetched.id, unit.id);
    assert_eq!(fetched.code, "tray");

    let fetched_by_code = get_unit_by_code(&conn, "TRAY")
        .unwrap()
        .expect("unit fetched case-insensitively");
    assert_eq!(fetched_by_code.id, unit.id);
}

#[test]
fn test_duplicate_unit_code_rejected() {
    let conn = setup_test_db();

    let input1 = CreateUnitInput {
        code: "barrel".to_string(),
        name: "Standard Oil Barrel".to_string(),
        dimension: "volume".to_string(),
        precision: Some(2),
        is_base: Some(false),
    };
    create_unit(&conn, input1).expect("unit created");

    let input2 = CreateUnitInput {
        code: "BARREL".to_string(),
        name: "Duplicate Barrel".to_string(),
        dimension: "volume".to_string(),
        precision: Some(2),
        is_base: Some(false),
    };
    let err = create_unit(&conn, input2).unwrap_err();
    assert!(matches!(err, UnitError::DuplicateCode(_)));
}

#[test]
fn test_list_units_with_filters() {
    let conn = setup_test_db();

    // Baseline seeded units include piece, box, pack, set, kg, g, L, ml, m, cm, m2
    let all_units = list_units(&conn, UnitFilter::default()).expect("all units");
    assert!(all_units.len() >= 11);

    // Filter by dimension
    let mass_units = list_units(
        &conn,
        UnitFilter {
            dimension: Some("mass".to_string()),
            is_base: None,
            query: None,
        },
    )
    .expect("mass units");
    assert_eq!(mass_units.len(), 2); // kg, g

    // Filter by search query
    let searched = list_units(
        &conn,
        UnitFilter {
            dimension: None,
            is_base: None,
            query: Some("Liter".to_string()),
        },
    )
    .expect("searched units");
    assert_eq!(searched.len(), 2); // Liter, Milliliter
}

#[test]
fn test_update_unit() {
    let conn = setup_test_db();

    let input = CreateUnitInput {
        code: "doz".to_string(),
        name: "Dozen Items".to_string(),
        dimension: "count".to_string(),
        precision: Some(0),
        is_base: Some(false),
    };
    let unit = create_unit(&conn, input).expect("created");

    let update = UpdateUnitInput {
        id: unit.id.clone(),
        code: "dozen".to_string(),
        name: "Baker's Dozen".to_string(),
        dimension: "count".to_string(),
        precision: 0,
        is_base: false,
    };
    let updated = update_unit(&conn, update).expect("updated");
    assert_eq!(updated.code, "dozen");
    assert_eq!(updated.name, "Baker's Dozen");

    let fetched = get_unit(&conn, &unit.id).unwrap().expect("fetched");
    assert_eq!(fetched.code, "dozen");
}

#[test]
fn test_is_base_demotes_other_units_in_dimension() {
    let conn = setup_test_db();

    let u1 = create_unit(
        &conn,
        CreateUnitInput {
            code: "mass_ref1".to_string(),
            name: "Mass Reference 1".to_string(),
            dimension: "mass".to_string(),
            precision: Some(3),
            is_base: Some(true),
        },
    )
    .expect("u1 created as base");
    assert!(u1.is_base);

    let u2 = create_unit(
        &conn,
        CreateUnitInput {
            code: "mass_ref2".to_string(),
            name: "Mass Reference 2".to_string(),
            dimension: "mass".to_string(),
            precision: Some(3),
            is_base: Some(true),
        },
    )
    .expect("u2 created as new base");
    assert!(u2.is_base);

    // Verify u1 was demoted to non-base
    let u1_reloaded = get_unit(&conn, &u1.id).unwrap().expect("u1 reloaded");
    assert!(!u1_reloaded.is_base);
}

#[test]
fn test_is_base_demotes_when_dimension_changes() {
    let conn = setup_test_db();

    // Create base unit in volume
    let v_base = create_unit(
        &conn,
        CreateUnitInput {
            code: "vol_base".to_string(),
            name: "Volume Base".to_string(),
            dimension: "volume".to_string(),
            precision: Some(3),
            is_base: Some(true),
        },
    )
    .expect("vol_base");

    // Create base unit in custom
    let c_base = create_unit(
        &conn,
        CreateUnitInput {
            code: "custom_base".to_string(),
            name: "Custom Base".to_string(),
            dimension: "custom".to_string(),
            precision: Some(2),
            is_base: Some(true),
        },
    )
    .expect("custom_base");

    // Move vol_base to custom dimension with is_base = true
    let updated = update_unit(
        &conn,
        UpdateUnitInput {
            id: v_base.id.clone(),
            code: "vol_base".to_string(),
            name: "Volume Base Moved".to_string(),
            dimension: "custom".to_string(),
            precision: 3,
            is_base: true,
        },
    )
    .expect("moved");
    assert!(updated.is_base);
    assert_eq!(updated.dimension, UnitDimension::Custom);

    // Verify c_base was demoted in custom dimension
    let c_base_reloaded = get_unit(&conn, &c_base.id).unwrap().expect("c_base");
    assert!(!c_base_reloaded.is_base);

    // Verify exactly one base unit exists in custom dimension
    let custom_units = list_units(
        &conn,
        UnitFilter {
            dimension: Some("custom".to_string()),
            is_base: Some(true),
            query: None,
        },
    )
    .expect("custom base units");
    assert_eq!(custom_units.len(), 1);
    assert_eq!(custom_units[0].id, v_base.id);
}

#[test]
fn test_delete_unit_and_associated_conversions() {
    let conn = setup_test_db();

    let u1 = create_unit(
        &conn,
        CreateUnitInput {
            code: "crate".to_string(),
            name: "Wooden Crate".to_string(),
            dimension: "count".to_string(),
            precision: Some(0),
            is_base: Some(false),
        },
    )
    .expect("crate");

    let piece = get_unit_by_code(&conn, "piece").unwrap().expect("piece");

    create_unit_conversion(
        &conn,
        CreateUnitConversionInput {
            from_unit_id: u1.id.clone(),
            to_unit_id: piece.id.clone(),
            multiplier: 24.0,
        },
    )
    .expect("conversion created");

    let convs_before = list_unit_conversions(&conn, Some(&u1.id)).expect("convs");
    assert_eq!(convs_before.len(), 1);

    delete_unit(&conn, &u1.id).expect("unit deleted");
    assert!(get_unit(&conn, &u1.id).unwrap().is_none());

    let convs_after = list_unit_conversions(&conn, Some(&u1.id)).expect("convs");
    assert_eq!(convs_after.len(), 0);
}

// =========================================================================
// 3. REPOSITORY CRUD TESTS — UNIT CONVERSIONS
// =========================================================================

#[test]
fn test_create_and_list_conversion_rules() {
    let conn = setup_test_db();

    let box_unit = get_unit_by_code(&conn, "box").unwrap().expect("box");
    let piece_unit = get_unit_by_code(&conn, "piece").unwrap().expect("piece");

    let conv = create_unit_conversion(
        &conn,
        CreateUnitConversionInput {
            from_unit_id: box_unit.id.clone(),
            to_unit_id: piece_unit.id.clone(),
            multiplier: 12.0,
        },
    )
    .expect("conversion created");

    assert_eq!(conv.from_unit_id, box_unit.id);
    assert_eq!(conv.to_unit_id, piece_unit.id);
    assert_approx(conv.multiplier, 12.0);
    assert!(!conv.created_at.is_empty());
    assert_ne!(conv.created_at, "now");

    let list = list_unit_conversions(&conn, None).expect("list");
    assert!(!list.is_empty());
    let found = list
        .iter()
        .find(|c| c.from_unit_id == box_unit.id && c.to_unit_id == piece_unit.id);
    assert!(found.is_some());
    assert_approx(found.unwrap().multiplier, 12.0);
}

#[test]
fn test_self_conversion_rule_rejected() {
    let conn = setup_test_db();
    let kg = get_unit_by_code(&conn, "kg").unwrap().expect("kg");

    let err = create_unit_conversion(
        &conn,
        CreateUnitConversionInput {
            from_unit_id: kg.id.clone(),
            to_unit_id: kg.id.clone(),
            multiplier: 1.0,
        },
    )
    .unwrap_err();

    assert!(matches!(err, UnitError::Validation(_)));
}

#[test]
fn test_delete_conversion_rule() {
    let conn = setup_test_db();
    let pack = get_unit_by_code(&conn, "pack").unwrap().expect("pack");
    let piece = get_unit_by_code(&conn, "piece").unwrap().expect("piece");

    create_unit_conversion(
        &conn,
        CreateUnitConversionInput {
            from_unit_id: pack.id.clone(),
            to_unit_id: piece.id.clone(),
            multiplier: 6.0,
        },
    )
    .expect("created");

    delete_unit_conversion(&conn, &pack.id, &piece.id).expect("deleted");

    let list = list_unit_conversions(&conn, Some(&pack.id)).expect("list");
    assert!(list.is_empty());
}

// =========================================================================
// 4. CONVERSION EVALUATION ENGINE TESTS
// =========================================================================

#[test]
fn test_identity_conversion() {
    let conn = setup_test_db();
    let kg = get_unit_by_code(&conn, "kg").unwrap().expect("kg");

    let factor = find_conversion_factor(&conn, &kg, &kg).expect("identity factor");
    assert_approx(factor, 1.0);

    let result = convert_quantity(
        &conn,
        ConvertQuantityInput {
            from_unit_id: kg.id.clone(),
            to_unit_id: kg.id.clone(),
            quantity: 42.5,
        },
    )
    .expect("converted");

    assert_approx(result.converted_quantity, 42.5);
    assert_approx(result.effective_multiplier, 1.0);
}

#[test]
fn test_direct_conversion() {
    let conn = setup_test_db();
    let box_u = get_unit_by_code(&conn, "box").unwrap().expect("box");
    let piece_u = get_unit_by_code(&conn, "piece").unwrap().expect("piece");

    create_unit_conversion(
        &conn,
        CreateUnitConversionInput {
            from_unit_id: box_u.id.clone(),
            to_unit_id: piece_u.id.clone(),
            multiplier: 12.0,
        },
    )
    .expect("conversion created");

    let factor = find_conversion_factor(&conn, &box_u, &piece_u).expect("direct factor");
    assert_approx(factor, 12.0);

    let result = convert_quantity(
        &conn,
        ConvertQuantityInput {
            from_unit_id: box_u.id.clone(),
            to_unit_id: piece_u.id.clone(),
            quantity: 3.0,
        },
    )
    .expect("converted");

    assert_approx(result.converted_quantity, 36.0);
    assert_approx(result.effective_multiplier, 12.0);
}

#[test]
fn test_inverse_conversion() {
    let conn = setup_test_db();
    let kg = get_unit_by_code(&conn, "kg").unwrap().expect("kg");
    let g = get_unit_by_code(&conn, "g").unwrap().expect("g");

    // Explicit direct: 1 kg = 1000 g
    create_unit_conversion(
        &conn,
        CreateUnitConversionInput {
            from_unit_id: kg.id.clone(),
            to_unit_id: g.id.clone(),
            multiplier: 1000.0,
        },
    )
    .expect("conversion created");

    // Inverse conversion: 2500 g -> kg should be 2.5 kg (multiplier 0.001)
    let factor = find_conversion_factor(&conn, &g, &kg).expect("inverse factor");
    assert_approx(factor, 0.001);

    let result = convert_quantity(
        &conn,
        ConvertQuantityInput {
            from_unit_id: g.id.clone(),
            to_unit_id: kg.id.clone(),
            quantity: 2500.0,
        },
    )
    .expect("converted");

    assert_approx(result.converted_quantity, 2.5);
}

#[test]
fn test_transitive_conversion_chain() {
    let conn = setup_test_db();

    // Pallet -> Box -> Pack -> Piece
    let pallet = create_unit(
        &conn,
        CreateUnitInput {
            code: "pallet".to_string(),
            name: "Standard Pallet".to_string(),
            dimension: "count".to_string(),
            precision: Some(0),
            is_base: Some(false),
        },
    )
    .expect("pallet");

    let box_u = get_unit_by_code(&conn, "box").unwrap().expect("box");
    let pack_u = get_unit_by_code(&conn, "pack").unwrap().expect("pack");
    let piece_u = get_unit_by_code(&conn, "piece").unwrap().expect("piece");

    // 1 Pallet = 10 Boxes
    create_unit_conversion(
        &conn,
        CreateUnitConversionInput {
            from_unit_id: pallet.id.clone(),
            to_unit_id: box_u.id.clone(),
            multiplier: 10.0,
        },
    )
    .expect("pallet -> box");

    // 1 Box = 5 Packs
    create_unit_conversion(
        &conn,
        CreateUnitConversionInput {
            from_unit_id: box_u.id.clone(),
            to_unit_id: pack_u.id.clone(),
            multiplier: 5.0,
        },
    )
    .expect("box -> pack");

    // 1 Pack = 6 Pieces
    create_unit_conversion(
        &conn,
        CreateUnitConversionInput {
            from_unit_id: pack_u.id.clone(),
            to_unit_id: piece_u.id.clone(),
            multiplier: 6.0,
        },
    )
    .expect("pack -> piece");

    // 1 Pallet -> Pieces should be 10 * 5 * 6 = 300 pieces
    let result = convert_quantity(
        &conn,
        ConvertQuantityInput {
            from_unit_id: pallet.id.clone(),
            to_unit_id: piece_u.id.clone(),
            quantity: 2.0,
        },
    )
    .expect("pallet to piece conversion");

    assert_approx(result.effective_multiplier, 300.0);
    assert_approx(result.converted_quantity, 600.0);

    // Inverse Transitive: 300 Pieces -> Pallet should be 1 Pallet
    let inv_result = convert_quantity(
        &conn,
        ConvertQuantityInput {
            from_unit_id: piece_u.id.clone(),
            to_unit_id: pallet.id.clone(),
            quantity: 600.0,
        },
    )
    .expect("piece to pallet conversion");

    assert_approx(inv_result.converted_quantity, 2.0);
}

#[test]
fn test_conversion_cycle_handling() {
    let conn = setup_test_db();

    // Create a cycle: U1 -> U2 -> U3 -> U1
    let u1 = create_unit(
        &conn,
        CreateUnitInput {
            code: "cycle_u1".to_string(),
            name: "Cycle 1".to_string(),
            dimension: "custom".to_string(),
            precision: Some(2),
            is_base: Some(false),
        },
    )
    .expect("u1");

    let u2 = create_unit(
        &conn,
        CreateUnitInput {
            code: "cycle_u2".to_string(),
            name: "Cycle 2".to_string(),
            dimension: "custom".to_string(),
            precision: Some(2),
            is_base: Some(false),
        },
    )
    .expect("u2");

    let u3 = create_unit(
        &conn,
        CreateUnitInput {
            code: "cycle_u3".to_string(),
            name: "Cycle 3".to_string(),
            dimension: "custom".to_string(),
            precision: Some(2),
            is_base: Some(false),
        },
    )
    .expect("u3");

    let target_unreachable = create_unit(
        &conn,
        CreateUnitInput {
            code: "isolated_u4".to_string(),
            name: "Isolated 4".to_string(),
            dimension: "custom".to_string(),
            precision: Some(2),
            is_base: Some(false),
        },
    )
    .expect("u4");

    create_unit_conversion(
        &conn,
        CreateUnitConversionInput {
            from_unit_id: u1.id.clone(),
            to_unit_id: u2.id.clone(),
            multiplier: 2.0,
        },
    )
    .expect("u1->u2");

    create_unit_conversion(
        &conn,
        CreateUnitConversionInput {
            from_unit_id: u2.id.clone(),
            to_unit_id: u3.id.clone(),
            multiplier: 3.0,
        },
    )
    .expect("u2->u3");

    create_unit_conversion(
        &conn,
        CreateUnitConversionInput {
            from_unit_id: u3.id.clone(),
            to_unit_id: u1.id.clone(),
            multiplier: 0.1666667,
        },
    )
    .expect("u3->u1");

    // Traversal terminates deterministically without infinite loop when target is unreachable
    let err = find_conversion_factor(&conn, &u1, &target_unreachable).unwrap_err();
    assert!(matches!(err, UnitError::ConversionPathNotFound { .. }));
}

#[test]
fn test_cross_dimension_mismatch_rejected() {
    let conn = setup_test_db();
    let kg = get_unit_by_code(&conn, "kg").unwrap().expect("kg");
    let liter = get_unit_by_code(&conn, "L").unwrap().expect("L");

    let err = convert_quantity(
        &conn,
        ConvertQuantityInput {
            from_unit_id: kg.id.clone(),
            to_unit_id: liter.id.clone(),
            quantity: 5.0,
        },
    )
    .unwrap_err();

    assert!(matches!(err, UnitError::IncompatibleDimensions { .. }));
}

#[test]
fn test_cross_dimension_with_explicit_bridge_rule() {
    let conn = setup_test_db();
    let kg = get_unit_by_code(&conn, "kg").unwrap().expect("kg");
    let liter = get_unit_by_code(&conn, "L").unwrap().expect("L");

    // Explicit density bridge: 1 L of Olive Oil = 0.92 kg (multiplier from L to kg is 0.92)
    create_unit_conversion(
        &conn,
        CreateUnitConversionInput {
            from_unit_id: liter.id.clone(),
            to_unit_id: kg.id.clone(),
            multiplier: 0.92,
        },
    )
    .expect("bridge created");

    let result = convert_quantity(
        &conn,
        ConvertQuantityInput {
            from_unit_id: liter.id.clone(),
            to_unit_id: kg.id.clone(),
            quantity: 10.0,
        },
    )
    .expect("converted");

    assert_approx(result.converted_quantity, 9.2);
    assert_approx(result.effective_multiplier, 0.92);
}

#[test]
fn test_explicit_reverse_rule_takes_precedence_over_inferred_inverse() {
    let conn = setup_test_db();

    let u1 = create_unit(
        &conn,
        CreateUnitInput {
            code: "asym_u1".to_string(),
            name: "Asymmetric 1".to_string(),
            dimension: "custom".to_string(),
            precision: Some(4),
            is_base: Some(false),
        },
    )
    .expect("u1");

    let u2 = create_unit(
        &conn,
        CreateUnitInput {
            code: "asym_u2".to_string(),
            name: "Asymmetric 2".to_string(),
            dimension: "custom".to_string(),
            precision: Some(4),
            is_base: Some(false),
        },
    )
    .expect("u2");

    // Explicit forward: 1 U1 = 2.0 U2
    create_unit_conversion(
        &conn,
        CreateUnitConversionInput {
            from_unit_id: u1.id.clone(),
            to_unit_id: u2.id.clone(),
            multiplier: 2.0,
        },
    )
    .expect("u1 -> u2");

    // Explicit reverse with non-symmetric multiplier: 1 U2 = 0.6 U1 (not 0.5)
    create_unit_conversion(
        &conn,
        CreateUnitConversionInput {
            from_unit_id: u2.id.clone(),
            to_unit_id: u1.id.clone(),
            multiplier: 0.6,
        },
    )
    .expect("u2 -> u1");

    let fwd_factor = find_conversion_factor(&conn, &u1, &u2).expect("fwd factor");
    assert_approx(fwd_factor, 2.0);

    let rev_factor = find_conversion_factor(&conn, &u2, &u1).expect("rev factor");
    assert_approx(rev_factor, 0.6);

    let fwd_conv = convert_quantity(
        &conn,
        ConvertQuantityInput {
            from_unit_id: u1.id.clone(),
            to_unit_id: u2.id.clone(),
            quantity: 10.0,
        },
    )
    .expect("fwd conv");
    assert_approx(fwd_conv.converted_quantity, 20.0);

    let rev_conv = convert_quantity(
        &conn,
        ConvertQuantityInput {
            from_unit_id: u2.id.clone(),
            to_unit_id: u1.id.clone(),
            quantity: 10.0,
        },
    )
    .expect("rev conv");
    assert_approx(rev_conv.converted_quantity, 6.0);
}

// =========================================================================
// 5. NUMERIC SAFETY & PRECISION TESTS
// =========================================================================

#[test]
fn test_precision_rounding_to_target_unit() {
    let conn = setup_test_db();

    // Piece has precision 0
    let piece = get_unit_by_code(&conn, "piece").unwrap().expect("piece");
    let box_u = get_unit_by_code(&conn, "box").unwrap().expect("box");

    create_unit_conversion(
        &conn,
        CreateUnitConversionInput {
            from_unit_id: box_u.id.clone(),
            to_unit_id: piece.id.clone(),
            multiplier: 7.0,
        },
    )
    .expect("conv");

    // 1 piece -> box (multiplier 1/7 = 0.142857...)
    // box has precision 0
    let res = convert_quantity(
        &conn,
        ConvertQuantityInput {
            from_unit_id: piece.id.clone(),
            to_unit_id: box_u.id.clone(),
            quantity: 1.0,
        },
    )
    .expect("converted");

    assert_approx(res.converted_quantity, 0.0); // 0.1428 rounded to 0 decimals is 0.0

    // kg has precision 3
    let kg = get_unit_by_code(&conn, "kg").unwrap().expect("kg");
    let g = get_unit_by_code(&conn, "g").unwrap().expect("g");

    create_unit_conversion(
        &conn,
        CreateUnitConversionInput {
            from_unit_id: kg.id.clone(),
            to_unit_id: g.id.clone(),
            multiplier: 1000.0,
        },
    )
    .expect("kg->g");

    // 1234.5678 grams to kg
    let res_kg = convert_quantity(
        &conn,
        ConvertQuantityInput {
            from_unit_id: g.id.clone(),
            to_unit_id: kg.id.clone(),
            quantity: 1234.5678,
        },
    )
    .expect("g to kg");

    assert_approx(res_kg.converted_quantity, 1.235); // rounded to 3 decimal places
}

#[test]
fn test_zero_quantity_conversion() {
    let conn = setup_test_db();
    let kg = get_unit_by_code(&conn, "kg").unwrap().expect("kg");
    let g = get_unit_by_code(&conn, "g").unwrap().expect("g");

    create_unit_conversion(
        &conn,
        CreateUnitConversionInput {
            from_unit_id: kg.id.clone(),
            to_unit_id: g.id.clone(),
            multiplier: 1000.0,
        },
    )
    .expect("conv");

    let res = convert_quantity(
        &conn,
        ConvertQuantityInput {
            from_unit_id: kg.id.clone(),
            to_unit_id: g.id.clone(),
            quantity: 0.0,
        },
    )
    .expect("zero quantity");

    assert_approx(res.converted_quantity, 0.0);
}

// =========================================================================
// 6. AUTHORIZATION & COMMAND CONTRACT TESTS
// =========================================================================

#[test]
fn test_unit_authorization_read_and_mutation() {
    let conn = setup_test_db();
    let (_, branch_id) = create_test_org_and_branch(&conn);

    let user_manager = create_test_user_with_creds(
        &conn,
        &branch_id,
        "Unit Manager",
        Some("unit_mgr"),
        None,
        None,
        "manager",
    )
    .expect("manager");
    let session_mgr = create_local_session(&conn, &user_manager.id, &branch_id, "pin", None)
        .expect("mgr session");

    // Manager can mutate units (has ProductsManage permission)
    assert!(crate::commands::authorize_catalog_mutation(&conn, &session_mgr.id).is_ok());

    // Manager can read units
    assert!(crate::commands::authorize_catalog_read(&conn, &session_mgr.id).is_ok());

    let created = create_unit(
        &conn,
        CreateUnitInput {
            code: "meter_sq".to_string(),
            name: "Square Meters".to_string(),
            dimension: "area".to_string(),
            precision: Some(2),
            is_base: Some(false),
        },
    )
    .expect("manager created unit");
    assert_eq!(created.code, "meter_sq");

    let fetched = get_unit(&conn, &created.id)
        .expect("read unit")
        .expect("found");
    assert_eq!(fetched.code, "meter_sq");

    let fetched_code = get_unit_by_code(&conn, "METER_SQ")
        .expect("read by code")
        .expect("found");
    assert_eq!(fetched_code.id, created.id);

    let all = list_units(&conn, UnitFilter::default()).expect("list units");
    assert!(!all.is_empty());

    let updated = update_unit(
        &conn,
        UpdateUnitInput {
            id: created.id.clone(),
            code: "sq_meter".to_string(),
            name: "Square Meter (Updated)".to_string(),
            dimension: "area".to_string(),
            precision: 3,
            is_base: false,
        },
    )
    .expect("updated unit");
    assert_eq!(updated.code, "sq_meter");

    let m2 = get_unit_by_code(&conn, "m2")
        .expect("get m2")
        .expect("found m2");

    let conv = create_unit_conversion(
        &conn,
        CreateUnitConversionInput {
            from_unit_id: created.id.clone(),
            to_unit_id: m2.id.clone(),
            multiplier: 1.0,
        },
    )
    .expect("created conversion");
    assert_approx(conv.multiplier, 1.0);

    let convs = list_unit_conversions(&conn, Some(&created.id)).expect("list convs");
    assert_eq!(convs.len(), 1);

    let converted = convert_quantity(
        &conn,
        ConvertQuantityInput {
            from_unit_id: created.id.clone(),
            to_unit_id: m2.id.clone(),
            quantity: 50.0,
        },
    )
    .expect("converted qty");
    assert_approx(converted.converted_quantity, 50.0);

    delete_unit_conversion(&conn, &created.id, &m2.id).expect("deleted conversion");
    delete_unit(&conn, &created.id).expect("deleted unit");
}

#[test]
fn test_unit_unauthorized_mutation_rejected() {
    let conn = setup_test_db();
    let (_, branch_id) = create_test_org_and_branch(&conn);

    let user_cashier = create_test_user_with_creds(
        &conn,
        &branch_id,
        "Unit Cashier",
        Some("unit_cashier"),
        None,
        None,
        "cashier",
    )
    .expect("cashier");
    let session_cashier = create_local_session(&conn, &user_cashier.id, &branch_id, "pin", None)
        .expect("cashier session");

    // Cashier cannot mutate units (lacks ProductsManage)
    let err = crate::commands::authorize_catalog_mutation(&conn, &session_cashier.id).unwrap_err();
    assert!(
        err.contains("permission")
            || err.contains("denied")
            || err.contains("unauthorized")
            || err.contains("missing permission")
    );

    // Cashier CAN read units with active session
    assert!(crate::commands::authorize_catalog_read(&conn, &session_cashier.id).is_ok());

    // Unauthenticated request fails closed
    let err_unauth =
        crate::commands::authorize_catalog_mutation(&conn, "invalid_session_token_12345")
            .unwrap_err();
    assert!(!err_unauth.is_empty());
}
