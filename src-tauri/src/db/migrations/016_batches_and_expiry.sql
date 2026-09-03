-- 016_batches_and_expiry.sql
-- F2.07 — Batches, Expiry Dates & FEFO Schema Hardening
-- Append-only migration. Never modify applied migrations.

-- 1. Pre-validation assertion: fail closed on invalid historical legacy quantities
SELECT CASE
    WHEN EXISTS (
        SELECT 1 FROM product_batches
        WHERE quantity < 0
           OR quantity IS NULL
           OR CAST(ROUND(quantity * 1000.0) AS INTEGER) / 1000.0 != quantity
    ) THEN RAISE(ABORT, 'Migration 016 aborted: legacy product_batches contains negative, NULL, or non-exact fractional quantities')
END;

-- 2. Rebuild product_batches with exact columns, nullable expiry_date, and integer milli precision
CREATE TABLE product_batches_new (
    id TEXT PRIMARY KEY DEFAULT (lower(hex(randomblob(16)))),
    product_id TEXT NOT NULL REFERENCES products(id),
    branch_id TEXT NOT NULL REFERENCES branches(id),
    variant_id TEXT REFERENCES product_variants(id),
    batch_number TEXT NOT NULL,
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

-- 4. Swap tables
DROP TABLE product_batches;
ALTER TABLE product_batches_new RENAME TO product_batches;

-- 5. Indexes
CREATE INDEX idx_batches_expiry ON product_batches(expiry_date);
CREATE INDEX idx_product_batches_fefo ON product_batches(branch_id, product_id, status, expiry_date);
CREATE INDEX idx_product_batches_variant_fefo ON product_batches(branch_id, product_id, variant_id, status, expiry_date);

CREATE UNIQUE INDEX idx_product_batches_unique_prod
    ON product_batches(branch_id, product_id, batch_number COLLATE NOCASE)
    WHERE variant_id IS NULL;

CREATE UNIQUE INDEX idx_product_batches_unique_var
    ON product_batches(branch_id, product_id, variant_id, batch_number COLLATE NOCASE)
    WHERE variant_id IS NOT NULL;
