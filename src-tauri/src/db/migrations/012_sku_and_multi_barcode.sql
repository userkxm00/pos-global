-- 012_sku_and_multi_barcode.sql
-- F2.03 — SKU and Multi-Barcode Management

-- 1. Add SKU column and partial unique index to products table
ALTER TABLE products ADD COLUMN sku TEXT;
CREATE UNIQUE INDEX IF NOT EXISTS idx_products_sku_active 
    ON products(sku COLLATE NOCASE) 
    WHERE sku IS NOT NULL AND is_active = 1;

-- 2. Concurrency-safe atomic SKU sequence table
CREATE TABLE IF NOT EXISTS sku_sequences (
    prefix TEXT PRIMARY KEY,
    last_sequence INTEGER NOT NULL DEFAULT 0,
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);

-- 3. Multi-barcode canonical registry table with strict state integrity CHECK constraint
CREATE TABLE IF NOT EXISTS product_barcodes (
    id TEXT PRIMARY KEY DEFAULT (lower(hex(randomblob(16)))),
    product_id TEXT NOT NULL REFERENCES products(id) ON DELETE CASCADE,
    barcode TEXT NOT NULL,
    symbology TEXT NOT NULL DEFAULT 'CODE128',
    is_primary INTEGER NOT NULL DEFAULT 0,
    is_active INTEGER NOT NULL DEFAULT 1,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now')),
    CHECK (is_active IN (0, 1) AND is_primary IN (0, 1) AND (is_primary = 0 OR is_active = 1))
);

-- 4. Indexes for product foreign key, lookup performance, and active uniqueness
CREATE INDEX IF NOT EXISTS idx_product_barcodes_product ON product_barcodes(product_id);
CREATE INDEX IF NOT EXISTS idx_product_barcodes_lookup ON product_barcodes(barcode COLLATE NOCASE);

-- Strict database invariant: Global uniqueness for all active barcodes across products
CREATE UNIQUE INDEX IF NOT EXISTS idx_product_barcodes_unique_active 
    ON product_barcodes(barcode COLLATE NOCASE) 
    WHERE is_active = 1;

-- Strict database invariant: At most ONE active primary barcode per product
CREATE UNIQUE INDEX IF NOT EXISTS idx_product_barcodes_one_active_primary 
    ON product_barcodes(product_id) 
    WHERE is_active = 1 AND is_primary = 1;

-- 5. Deterministic Data Backfill: Active products
-- If case-insensitive collisions existed in legacy data, earliest created product gets the canonical entry.
INSERT INTO product_barcodes (id, product_id, barcode, symbology, is_primary, is_active, created_at, updated_at)
SELECT lower(hex(randomblob(16))), id, trim(barcode), 'UNKNOWN', 1, 1, created_at, updated_at
FROM (
    SELECT id, barcode, created_at, updated_at,
           ROW_NUMBER() OVER (PARTITION BY trim(barcode) COLLATE NOCASE ORDER BY created_at ASC, id ASC) as rn
    FROM products
    WHERE is_active = 1 
      AND barcode IS NOT NULL 
      AND length(trim(barcode)) > 0
)
WHERE rn = 1;

-- Clear legacy mirror for any active product whose colliding barcode was not backfilled
UPDATE products
SET barcode = NULL
WHERE is_active = 1
  AND barcode IS NOT NULL
  AND id NOT IN (
      SELECT product_id FROM product_barcodes WHERE is_active = 1 AND is_primary = 1
  );

-- Synchronize retained active products' legacy mirror with canonical trimmed barcode
UPDATE products
SET barcode = (
    SELECT pb.barcode
    FROM product_barcodes pb
    WHERE pb.product_id = products.id AND pb.is_active = 1 AND pb.is_primary = 1
)
WHERE is_active = 1
  AND id IN (
      SELECT product_id FROM product_barcodes WHERE is_active = 1 AND is_primary = 1
  );

-- 6. Deterministic Data Backfill: Inactive (soft-deleted) products
INSERT INTO product_barcodes (id, product_id, barcode, symbology, is_primary, is_active, created_at, updated_at)
SELECT lower(hex(randomblob(16))), id, trim(barcode), 'UNKNOWN', 0, 0, created_at, updated_at
FROM products
WHERE is_active = 0 
  AND barcode IS NOT NULL 
  AND length(trim(barcode)) > 0;

-- 7. Enforce Canonical Mirror Invariant: Clear products.barcode on inactive or blank rows
UPDATE products 
SET barcode = NULL 
WHERE is_active = 0 
   OR (barcode IS NOT NULL AND length(trim(barcode)) = 0);
