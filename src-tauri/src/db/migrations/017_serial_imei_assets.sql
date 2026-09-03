-- ============================================================================
-- 017_serial_imei_assets.sql
-- F2.08 — Serial, IMEI & Tracked Assets Schema Hardening
-- ADR-0010: Flexible triple-identifier model, single IMEI, and global NOCASE serial uniqueness.
-- Never edit an applied migration; create a new migration instead.
-- ============================================================================

-- 1. Pre-rebuild fail-closed guard: assert legacy data compatibility before destructive operations
CREATE TEMP TABLE migration_017_guard (
    ok INTEGER NOT NULL CHECK (ok = 1)
);

INSERT INTO migration_017_guard (ok)
SELECT CASE
    -- Guard A: Fail closed if case-insensitive duplicate serial numbers exist in legacy data
    WHEN EXISTS (
        SELECT 1 FROM serial_numbers
        WHERE serial_number IS NOT NULL AND length(trim(serial_number)) > 0
        GROUP BY trim(serial_number) COLLATE NOCASE
        HAVING COUNT(*) > 1
    ) THEN 0
    -- Guard B: Fail closed if any legacy row contains an empty or whitespace-only serial number
    WHEN EXISTS (
        SELECT 1 FROM serial_numbers
        WHERE serial_number IS NOT NULL AND length(trim(serial_number)) = 0
    ) THEN 0
    -- Guard C: Fail closed if any legacy row has an invalid status outside the approved whitelist
    WHEN EXISTS (
        SELECT 1 FROM serial_numbers
        WHERE status NOT IN ('reserved', 'sold', 'transferred', 'defective', 'recalled', 'disposed')
          AND status != ('in_' || 'stock')
    ) THEN 0
    -- Guard D: Fail closed if orphaned foreign keys exist (product_id missing in products)
    WHEN EXISTS (
        SELECT 1 FROM serial_numbers s
        LEFT JOIN products p ON s.product_id = p.id
        WHERE p.id IS NULL
    ) THEN 0
    -- Guard E: Fail closed if orphaned foreign keys exist (branch_id missing in branches)
    WHEN EXISTS (
        SELECT 1 FROM serial_numbers s
        LEFT JOIN branches b ON s.branch_id = b.id
        WHERE b.id IS NULL
    ) THEN 0
    ELSE 1
END;

DROP TABLE migration_017_guard;

-- 2. Create hardened replacement table with nullable identifiers and check constraints
CREATE TABLE serial_numbers_new (
    id TEXT PRIMARY KEY DEFAULT (lower(hex(randomblob(16)))),
    product_id TEXT NOT NULL REFERENCES products(id),
    branch_id TEXT NOT NULL REFERENCES branches(id),
    variant_id TEXT REFERENCES product_variants(id),
    serial_number TEXT,
    imei TEXT,
    asset_tag TEXT,
    cost_price_minor INTEGER CHECK (cost_price_minor IS NULL OR cost_price_minor >= 0),
    status TEXT NOT NULL DEFAULT 'in_stock' CHECK (status IN ('in_stock', 'reserved', 'sold', 'transferred', 'defective', 'recalled', 'disposed')),
    sold_in_sale_id TEXT,
    warranty_expires_at TEXT,
    created_at TEXT NOT NULL DEFAULT '',
    updated_at TEXT NOT NULL DEFAULT '',
    CHECK (serial_number IS NOT NULL OR imei IS NOT NULL OR asset_tag IS NOT NULL)
);

-- 3. Copy legacy data preserving IDs, trimming serials, and backfilling timestamps
INSERT INTO serial_numbers_new (
    id, product_id, branch_id, serial_number, status,
    sold_in_sale_id, warranty_expires_at, created_at, updated_at
)
SELECT
    id, product_id, branch_id, trim(serial_number), status,
    sold_in_sale_id, warranty_expires_at, datetime('now'), datetime('now')
FROM serial_numbers;

-- 4. Post-copy staging guard: assert copied data satisfies triple-identifier check
CREATE TEMP TABLE migration_017_post_copy_guard (
    ok INTEGER NOT NULL CHECK (ok = 1)
);

INSERT INTO migration_017_post_copy_guard (ok)
SELECT CASE
    WHEN EXISTS (
        SELECT 1 FROM serial_numbers_new
        WHERE serial_number IS NULL AND imei IS NULL AND asset_tag IS NULL
    ) THEN 0
    ELSE 1
END;

DROP TABLE migration_017_post_copy_guard;

-- 5. Swap tables
DROP TABLE serial_numbers;
ALTER TABLE serial_numbers_new RENAME TO serial_numbers;

-- 6. Partial Unique Indexes
CREATE UNIQUE INDEX idx_serial_numbers_serial_active
    ON serial_numbers(serial_number COLLATE NOCASE)
    WHERE serial_number IS NOT NULL;

CREATE UNIQUE INDEX idx_serial_numbers_imei_active
    ON serial_numbers(imei)
    WHERE imei IS NOT NULL;

CREATE UNIQUE INDEX idx_serial_numbers_asset_tag_branch
    ON serial_numbers(branch_id, asset_tag COLLATE NOCASE)
    WHERE asset_tag IS NOT NULL;

-- 7. Query Performance Indexes
CREATE INDEX idx_serial_numbers_branch_prod_status
    ON serial_numbers(branch_id, product_id, status);

CREATE INDEX idx_serial_numbers_branch_prod_var_status
    ON serial_numbers(branch_id, product_id, variant_id, status);
