// GS1 Modulo-10 Check Digit Calculation and Verification.
// F2.03 — SKU / Barcode

use super::BarcodeError;

/// Calculates the GS1 standard Modulo-10 check digit for a string of numeric digits.
///
/// Algorithm:
/// 1. Process digits right-to-left.
/// 2. Alternate weight multipliers: 3 for the first digit to the left of the check digit position, then 1, 3, 1...
/// 3. Sum weighted digits.
/// 4. Check digit is the smallest number that, when added to the sum, results in a multiple of 10.
///    Formula: `(10 - (sum % 10)) % 10`.
pub fn calculate_gs1_check_digit(digits_without_check: &str) -> Result<u8, BarcodeError> {
    let trimmed = digits_without_check.trim();
    if trimmed.is_empty() {
        return Err(BarcodeError::Validation(
            "Cannot calculate check digit for empty string".into(),
        ));
    }

    let mut sum: u32 = 0;
    let mut weight = 3;

    for ch in trimmed.chars().rev() {
        let digit = ch.to_digit(10).ok_or_else(|| {
            BarcodeError::Validation(format!(
                "Non-numeric character '{ch}' in barcode check digit input"
            ))
        })?;
        sum += digit * weight;
        weight = if weight == 3 { 1 } else { 3 };
    }

    let check_digit = ((10 - (sum % 10)) % 10) as u8;
    Ok(check_digit)
}

/// Verifies that a full barcode string (including check digit) satisfies the GS1 Modulo-10 check.
pub fn verify_gs1_check_digit(full_code: &str) -> Result<(), BarcodeError> {
    let trimmed = full_code.trim();
    if trimmed.len() < 2 {
        return Err(BarcodeError::Validation(
            "Barcode must be at least 2 characters to verify check digit".into(),
        ));
    }

    let body = &trimmed[..trimmed.len() - 1];
    let expected_check = calculate_gs1_check_digit(body)?;

    let actual_char = trimmed.chars().last().unwrap();
    let actual_check = actual_char.to_digit(10).ok_or_else(|| {
        BarcodeError::Validation(format!(
            "Check digit character '{actual_char}' is not numeric"
        ))
    })? as u8;

    if actual_check != expected_check {
        return Err(BarcodeError::InvalidCheckDigit {
            expected: expected_check,
            actual: actual_check,
        });
    }

    Ok(())
}
