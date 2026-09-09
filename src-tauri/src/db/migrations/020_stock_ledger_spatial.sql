-- 020_stock_ledger_spatial.sql
-- F2.11 — Stock Ledger & Spatial Balances Architecture
-- Append-only migration. Never modify applied migrations.

-- 1. Composite uniqueness on bins(id, location_id) required for foreign key reference
CREATE UNIQUE INDEX IF NOT EXISTS idx_bins_id_location_id
    ON bins(id, location_id);

-- 2. Location Inventory: distributed physical stock balances
CREATE TABLE location_inventory (
    id TEXT PRIMARY KEY DEFAULT (lower(hex(randomblob(16)))),
    branch_id TEXT NOT NULL REFERENCES branches(id) ON DELETE RESTRICT,
    location_id TEXT NOT NULL REFERENCES locations(id) ON DELETE RESTRICT,
    product_id TEXT NOT NULL REFERENCES products(id) ON DELETE RESTRICT,
    bin_id TEXT REFERENCES bins(id) ON DELETE RESTRICT,
    variant_id TEXT REFERENCES product_variants(id) ON DELETE RESTRICT,
    batch_id TEXT REFERENCES product_batches(id) ON DELETE RESTRICT,
    quantity_milli INTEGER NOT NULL DEFAULT 0 CHECK (quantity_milli >= 0),
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now')),
    FOREIGN KEY (location_id, branch_id) REFERENCES locations(id, branch_id) ON DELETE RESTRICT,
    FOREIGN KEY (bin_id, location_id) REFERENCES bins(id, location_id) ON DELETE RESTRICT
);

-- 3. Mutually exclusive partial unique indexes covering all 8 combinations of (bin_id, variant_id, batch_id) nullability
CREATE UNIQUE INDEX uq_location_inventory_no_bin_no_var_no_batch
    ON location_inventory(branch_id, location_id, product_id)
    WHERE bin_id IS NULL AND variant_id IS NULL AND batch_id IS NULL;

CREATE UNIQUE INDEX uq_location_inventory_bin_no_var_no_batch
    ON location_inventory(branch_id, location_id, product_id, bin_id)
    WHERE bin_id IS NOT NULL AND variant_id IS NULL AND batch_id IS NULL;

CREATE UNIQUE INDEX uq_location_inventory_no_bin_var_no_batch
    ON location_inventory(branch_id, location_id, product_id, variant_id)
    WHERE bin_id IS NULL AND variant_id IS NOT NULL AND batch_id IS NULL;

CREATE UNIQUE INDEX uq_location_inventory_bin_var_no_batch
    ON location_inventory(branch_id, location_id, product_id, bin_id, variant_id)
    WHERE bin_id IS NOT NULL AND variant_id IS NOT NULL AND batch_id IS NULL;

CREATE UNIQUE INDEX uq_location_inventory_no_bin_no_var_batch
    ON location_inventory(branch_id, location_id, product_id, batch_id)
    WHERE bin_id IS NULL AND variant_id IS NULL AND batch_id IS NOT NULL;

CREATE UNIQUE INDEX uq_location_inventory_bin_no_var_batch
    ON location_inventory(branch_id, location_id, product_id, bin_id, batch_id)
    WHERE bin_id IS NOT NULL AND variant_id IS NULL AND batch_id IS NOT NULL;

CREATE UNIQUE INDEX uq_location_inventory_no_bin_var_batch
    ON location_inventory(branch_id, location_id, product_id, variant_id, batch_id)
    WHERE bin_id IS NULL AND variant_id IS NOT NULL AND batch_id IS NOT NULL;

CREATE UNIQUE INDEX uq_location_inventory_bin_var_batch
    ON location_inventory(branch_id, location_id, product_id, bin_id, variant_id, batch_id)
    WHERE bin_id IS NOT NULL AND variant_id IS NOT NULL AND batch_id IS NOT NULL;

-- 4. Lookup performance indexes on location_inventory
CREATE INDEX idx_location_inventory_location
    ON location_inventory(location_id);

CREATE INDEX idx_location_inventory_bin
    ON location_inventory(bin_id);

CREATE INDEX idx_location_inventory_branch_product
    ON location_inventory(branch_id, product_id);

CREATE INDEX idx_location_inventory_batch
    ON location_inventory(batch_id);

-- 5. Spatial and entity attribution columns on stock_movements
ALTER TABLE stock_movements ADD COLUMN location_id TEXT REFERENCES locations(id);
ALTER TABLE stock_movements ADD COLUMN bin_id TEXT REFERENCES bins(id);
ALTER TABLE stock_movements ADD COLUMN batch_id TEXT REFERENCES product_batches(id);
ALTER TABLE stock_movements ADD COLUMN serial_id TEXT REFERENCES serial_numbers(id);

CREATE INDEX idx_stock_movements_location ON stock_movements(location_id);
CREATE INDEX idx_stock_movements_bin ON stock_movements(bin_id);
CREATE INDEX idx_stock_movements_batch ON stock_movements(batch_id);
CREATE INDEX idx_stock_movements_serial ON stock_movements(serial_id);

-- 6. Spatial attribution columns on serial_numbers
ALTER TABLE serial_numbers ADD COLUMN location_id TEXT REFERENCES locations(id);
ALTER TABLE serial_numbers ADD COLUMN bin_id TEXT REFERENCES bins(id);

CREATE INDEX idx_serial_numbers_location ON serial_numbers(location_id);
CREATE INDEX idx_serial_numbers_bin ON serial_numbers(bin_id);

-- 7. Request hash column on idempotency_keys
ALTER TABLE idempotency_keys ADD COLUMN request_hash TEXT;

-- 8. Immutability enforcement: stock_movements rows cannot be updated or deleted
CREATE TRIGGER trg_stock_movements_immutable_update
BEFORE UPDATE ON stock_movements
BEGIN
    SELECT RAISE(ABORT, 'stock_movements records are immutable');
END;

CREATE TRIGGER trg_stock_movements_immutable_delete
BEFORE DELETE ON stock_movements
BEGIN
    SELECT RAISE(ABORT, 'stock_movements records are immutable');
END;
