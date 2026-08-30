// Barcode symbologies and format validators.
// F2.03 — SKU / Barcode

use super::check_digit::verify_gs1_check_digit;
use super::BarcodeError;
use serde::{Deserialize, Serialize};

/// Supported barcode symbologies.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BarcodeSymbology {
    #[serde(rename = "EAN13")]
    Ean13,
    #[serde(rename = "EAN8")]
    Ean8,
    #[serde(rename = "UPCA")]
    UpcA,
    #[serde(rename = "UPCE")]
    UpcE,
    #[serde(rename = "CODE128")]
    Code128,
    #[serde(rename = "CODE39")]
    Code39,
    #[serde(rename = "CUSTOM")]
    Custom,
    #[serde(rename = "UNKNOWN")]
    Unknown,
}

impl BarcodeSymbology {
    pub fn as_str(&self) -> &'static str {
        match self {
            BarcodeSymbology::Ean13 => "EAN13",
            BarcodeSymbology::Ean8 => "EAN8",
            BarcodeSymbology::UpcA => "UPCA",
            BarcodeSymbology::UpcE => "UPCE",
            BarcodeSymbology::Code128 => "CODE128",
            BarcodeSymbology::Code39 => "CODE39",
            BarcodeSymbology::Custom => "CUSTOM",
            BarcodeSymbology::Unknown => "UNKNOWN",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        match s.trim().to_uppercase().as_str() {
            "EAN13" | "EAN-13" => Some(BarcodeSymbology::Ean13),
            "EAN8" | "EAN-8" => Some(BarcodeSymbology::Ean8),
            "UPCA" | "UPC-A" => Some(BarcodeSymbology::UpcA),
            "UPCE" | "UPC-E" => Some(BarcodeSymbology::UpcE),
            "CODE128" | "CODE-128" => Some(BarcodeSymbology::Code128),
            "CODE39" | "CODE-39" => Some(BarcodeSymbology::Code39),
            "CUSTOM" => Some(BarcodeSymbology::Custom),
            "UNKNOWN" => Some(BarcodeSymbology::Unknown),
            _ => None,
        }
    }
}

impl std::fmt::Display for BarcodeSymbology {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

fn validate_numeric_length(s: &str, exact_len: usize, name: &str) -> Result<(), BarcodeError> {
    if s.len() != exact_len {
        return Err(BarcodeError::Validation(format!(
            "{name} barcode must be exactly {exact_len} numeric digits, got {}",
            s.len()
        )));
    }
    if !s.chars().all(|c| c.is_ascii_digit()) {
        return Err(BarcodeError::Validation(format!(
            "{name} barcode must contain only numeric digits (0-9)"
        )));
    }
    Ok(())
}

/// Validates an EAN-13 barcode (13 numeric digits + GS1 Modulo-10 check digit).
pub fn validate_ean13(barcode: &str) -> Result<(), BarcodeError> {
    validate_numeric_length(barcode, 13, "EAN-13")?;
    verify_gs1_check_digit(barcode)?;
    Ok(())
}

/// Validates an EAN-8 barcode (8 numeric digits + GS1 Modulo-10 check digit).
pub fn validate_ean8(barcode: &str) -> Result<(), BarcodeError> {
    validate_numeric_length(barcode, 8, "EAN-8")?;
    verify_gs1_check_digit(barcode)?;
    Ok(())
}

/// Validates a UPC-A barcode (12 numeric digits + GS1 Modulo-10 check digit).
pub fn validate_upc_a(barcode: &str) -> Result<(), BarcodeError> {
    validate_numeric_length(barcode, 12, "UPC-A")?;
    verify_gs1_check_digit(barcode)?;
    Ok(())
}

/// Validates a UPC-E barcode (8 numeric digits).
pub fn validate_upc_e(barcode: &str) -> Result<(), BarcodeError> {
    validate_numeric_length(barcode, 8, "UPC-E")?;
    Ok(())
}

/// Validates a Code 128 barcode (1 to 128 printable ASCII characters).
pub fn validate_code128(barcode: &str) -> Result<(), BarcodeError> {
    if barcode.is_empty() || barcode.len() > 128 {
        return Err(BarcodeError::Validation(format!(
            "Code 128 barcode length must be between 1 and 128 characters, got {}",
            barcode.len()
        )));
    }
    if !barcode
        .chars()
        .all(|c| c.is_ascii() && !c.is_ascii_control())
    {
        return Err(BarcodeError::Validation(
            "Code 128 barcode contains invalid non-printable or non-ASCII characters".into(),
        ));
    }
    Ok(())
}

/// Validates a Code 39 barcode (1 to 64 alphanumeric / standard symbols: A-Z, 0-9, -, ., $, /, +, %, space).
pub fn validate_code39(barcode: &str) -> Result<(), BarcodeError> {
    if barcode.is_empty() || barcode.len() > 64 {
        return Err(BarcodeError::Validation(format!(
            "Code 39 barcode length must be between 1 and 64 characters, got {}",
            barcode.len()
        )));
    }
    let valid_code39_char = |c: char| {
        c.is_ascii_uppercase()
            || c.is_ascii_digit()
            || matches!(c, '-' | '.' | ' ' | '$' | '/' | '+' | '%')
    };
    if !barcode.chars().all(valid_code39_char) {
        return Err(BarcodeError::Validation(
            "Code 39 barcode contains invalid characters. Allowed: A-Z, 0-9, -, ., $, /, +, %, space".into(),
        ));
    }
    Ok(())
}

/// Validates a custom or arbitrary barcode string.
pub fn validate_custom_barcode(barcode: &str) -> Result<(), BarcodeError> {
    if barcode.is_empty() || barcode.len() > 128 {
        return Err(BarcodeError::Validation(format!(
            "Custom barcode length must be between 1 and 128 characters, got {}",
            barcode.len()
        )));
    }
    if !barcode
        .chars()
        .all(|c| c.is_ascii() && !c.is_ascii_control())
    {
        return Err(BarcodeError::Validation(
            "Custom barcode contains invalid non-printable characters".into(),
        ));
    }
    Ok(())
}

/// Validates a barcode against its explicit symbology specification.
pub fn validate_barcode_symbology(
    barcode: &str,
    symbology: BarcodeSymbology,
) -> Result<String, BarcodeError> {
    let trimmed = barcode.trim();
    if trimmed.is_empty() {
        return Err(BarcodeError::Validation("Barcode cannot be empty".into()));
    }

    match symbology {
        BarcodeSymbology::Ean13 => validate_ean13(trimmed)?,
        BarcodeSymbology::Ean8 => validate_ean8(trimmed)?,
        BarcodeSymbology::UpcA => validate_upc_a(trimmed)?,
        BarcodeSymbology::UpcE => validate_upc_e(trimmed)?,
        BarcodeSymbology::Code128 => validate_code128(trimmed)?,
        BarcodeSymbology::Code39 => validate_code39(trimmed)?,
        BarcodeSymbology::Custom | BarcodeSymbology::Unknown => {
            validate_custom_barcode(trimmed)?;
        }
    }

    Ok(trimmed.to_string())
}

/// Auto-detects the most specific valid symbology for a barcode string.
pub fn detect_symbology(barcode: &str) -> BarcodeSymbology {
    let trimmed = barcode.trim();

    if trimmed.len() == 13
        && trimmed.chars().all(|c| c.is_ascii_digit())
        && validate_ean13(trimmed).is_ok()
    {
        return BarcodeSymbology::Ean13;
    }
    if trimmed.len() == 12
        && trimmed.chars().all(|c| c.is_ascii_digit())
        && validate_upc_a(trimmed).is_ok()
    {
        return BarcodeSymbology::UpcA;
    }
    if trimmed.len() == 8
        && trimmed.chars().all(|c| c.is_ascii_digit())
        && validate_ean8(trimmed).is_ok()
    {
        return BarcodeSymbology::Ean8;
    }
    if trimmed.len() == 8 && trimmed.chars().all(|c| c.is_ascii_digit()) {
        return BarcodeSymbology::UpcE;
    }
    if validate_code39(trimmed).is_ok() {
        return BarcodeSymbology::Code39;
    }
    if validate_code128(trimmed).is_ok() {
        return BarcodeSymbology::Code128;
    }

    BarcodeSymbology::Custom
}
