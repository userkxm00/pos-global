-- 016_batches_and_expiry.sql
-- F2.07 — Batches, Expiry Dates & FEFO Schema Hardening
-- Append-only migration. Never modify applied migrations.

-- 1. Pre-validation assertion: fail closed on invalid historical legacy quantities and duplicate batch numbers
CREATE TEMP TABLE migration_016_guard (
    ok INTEGER NOT NULL CHECK (ok = 1)
);

INSERT INTO migration_016_guard (ok)
SELECT CASE
    -- Quantity integrity: fail closed on negative, NULL, or fractional quantities beyond 3 decimal places
    WHEN EXISTS (
        SELECT 1 FROM product_batches
        WHERE quantity < 0
           OR quantity IS NULL
           OR CAST(ROUND(quantity * 1000.0) AS INTEGER) / 1000.0 != quantity
    ) THEN 0
    -- Non-variant duplicate batch numbers (case-insensitive collision within same branch and product)
    WHEN EXISTS (
        SELECT 1 FROM product_batches
        WHERE batch_number IS NOT NULL
        GROUP BY branch_id, product_id, batch_number COLLATE NOCASE
        HAVING COUNT(*) > 1
    ) THEN 0
    ELSE 1
END;

DROP TABLE migration_016_guard;

-- 2. Rebuild product_batches with exact columns, nullable expiry_date, nullable batch_number for historical data, and integer milli precision
CREATE TABLE product_batches_new (
    id TEXT PRIMARY KEY DEFAULT (lower(hex(randomblob(16)))),
    product_id TEXT NOT NULL REFERENCES products(id),
    branch_id TEXT NOT NULL REFERENCES branches(id),
    variant_id TEXT REFERENCES product_variants(id),
    batch_number TEXT,
    quantity_milli INTEGER NOT NULL DEFAULT 0 CHECK (quantity_milli >= 0),
    cost_price_minor INTEGER CHECK (cost_price_minor IS NULL OR cost_price_minor >= 0),
    status TEXT NOT NULL DEFAULT 'active' CHECK (status IN ('active', 'quarantined', 'recalled', 'depleted')),
    manufactured_date TEXT,
    expiry_date TEXT,
    received_at TEXT NOT NULL DEFAULT (datetime('now')),
    created_at TEXT NOT NULL DEFAULT '',
    updated_at TEXT NOT NULL DEFAULT ''
);

-- 3. Copy legacy data with exact integer conversion (dropping legacy quantity REAL)
INSERT INTO product_batches_new (
    id, product_id, branch_id, batch_number, quantity_milli, expiry_date, received_at, created_at, updated_at
)
SELECT
    id, product_id, branch_id, batch_number,
    CAST(ROUND(quantity * 1000.0) AS INTEGER),
    expiry_date, received_at, received_at, received_at
FROM product_batches;

-- 4. Pre-drop validation: verify staged data satisfies partial unique index constraints before destructive drop
CREATE TEMP TABLE migration_016_post_copy_guard (
    ok INTEGER NOT NULL CHECK (ok = 1)
);

INSERT INTO migration_016_post_copy_guard (ok)
SELECT CASE
    -- Check non-variant uniqueness in staged product_batches_new
    WHEN EXISTS (
        SELECT 1 FROM product_batches_new
        WHERE variant_id IS NULL AND batch_number IS NOT NULL
        GROUP BY branch_id, product_id, batch_number COLLATE NOCASE
        HAVING COUNT(*) > 1
    ) THEN 0
    -- Check variant uniqueness in staged product_batches_new
    WHEN EXISTS (
        SELECT 1 FROM product_batches_new
        WHERE variant_id IS NOT NULL AND batch_number IS NOT NULL
        GROUP BY branch_id, product_id, variant_id, batch_number COLLATE NOCASE
        HAVING COUNT(*) > 1
    ) THEN 0
    ELSE 1
END;

DROP TABLE migration_016_post_copy_guard;

-- 5. Swap tables
DROP TABLE product_batches;
ALTER TABLE product_batches_new RENAME TO product_batches;

-- 6. Indexes
CREATE INDEX idx_batches_expiry ON product_batches(expiry_date);
CREATE INDEX idx_product_batches_fefo ON product_batches(branch_id, product_id, status, expiry_date);
CREATE INDEX idx_product_batches_variant_fefo ON product_batches(branch_id, product_id, variant_id, status, expiry_date);

CREATE UNIQUE INDEX idx_product_batches_unique_prod
    ON product_batches(branch_id, product_id, batch_number COLLATE NOCASE)
    WHERE variant_id IS NULL AND batch_number IS NOT NULL;

CREATE UNIQUE INDEX idx_product_batches_unique_var
    ON product_batches(branch_id, product_id, variant_id, batch_number COLLATE NOCASE)
    WHERE variant_id IS NOT NULL AND batch_number IS NOT NULL;
