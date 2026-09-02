-- 014_product_variants_hardening.sql
-- F2.05 — Product Variants & Matrix Engine Schema Hardening
-- Append-only migration. Never modify applied migrations.

-- 1. Extend product_variants with exact integer money, cost, audit timestamps, and soft-deletion
-- Using SQLite-compatible constant defaults for table-state independence
ALTER TABLE product_variants ADD COLUMN price_override_minor INTEGER;
ALTER TABLE product_variants ADD COLUMN cost_price_minor INTEGER;
ALTER TABLE product_variants ADD COLUMN created_at TEXT NOT NULL DEFAULT '1970-01-01 00:00:00';
ALTER TABLE product_variants ADD COLUMN updated_at TEXT NOT NULL DEFAULT '1970-01-01 00:00:00';
ALTER TABLE product_variants ADD COLUMN deleted_at TEXT;

-- 2. Backfill existing product_variants rows with current timestamps and integer minor money
UPDATE product_variants
SET created_at = datetime('now')
WHERE created_at = '1970-01-01 00:00:00';

UPDATE product_variants
SET updated_at = datetime('now')
WHERE updated_at = '1970-01-01 00:00:00';

UPDATE product_variants
SET price_override_minor = CAST(ROUND(price_override * 100.0) AS INTEGER)
WHERE price_override IS NOT NULL AND price_override_minor IS NULL;

-- 3. Extend attribute_definitions with sort_order and created_at
ALTER TABLE attribute_definitions ADD COLUMN sort_order INTEGER NOT NULL DEFAULT 0;
ALTER TABLE attribute_definitions ADD COLUMN created_at TEXT NOT NULL DEFAULT '1970-01-01 00:00:00';

UPDATE attribute_definitions
SET created_at = datetime('now')
WHERE created_at = '1970-01-01 00:00:00';

-- 4. Extend attribute_values with sort_order and created_at
ALTER TABLE attribute_values ADD COLUMN sort_order INTEGER NOT NULL DEFAULT 0;
ALTER TABLE attribute_values ADD COLUMN created_at TEXT NOT NULL DEFAULT '1970-01-01 00:00:00';

UPDATE attribute_values
SET created_at = datetime('now')
WHERE created_at = '1970-01-01 00:00:00';

-- 5. Case-insensitive unique indexes for attribute definitions and values
CREATE UNIQUE INDEX IF NOT EXISTS idx_attribute_definitions_name_nocase
    ON attribute_definitions(name COLLATE NOCASE);

CREATE UNIQUE INDEX IF NOT EXISTS idx_attribute_values_def_val_nocase
    ON attribute_values(attribute_definition_id, value COLLATE NOCASE);

-- 6. Partial unique indexes for active product variants (SKU and Barcode)
CREATE UNIQUE INDEX IF NOT EXISTS idx_product_variants_sku_active
    ON product_variants(sku COLLATE NOCASE)
    WHERE sku IS NOT NULL AND is_active = 1;

CREATE UNIQUE INDEX IF NOT EXISTS idx_product_variants_barcode_active
    ON product_variants(barcode COLLATE NOCASE)
    WHERE barcode IS NOT NULL AND is_active = 1;

-- 7. Query and foreign-key performance indexes
CREATE INDEX IF NOT EXISTS idx_product_variants_product
    ON product_variants(product_id);

CREATE INDEX IF NOT EXISTS idx_variant_attribute_values_variant
    ON variant_attribute_values(variant_id);

CREATE INDEX IF NOT EXISTS idx_variant_attribute_values_value
    ON variant_attribute_values(attribute_value_id);
