-- 020_stock_ledger_and_spatial_balances.sql
-- F2.11 — Stock Movement Ledger and Spatial Inventory Architecture
-- ADR-0013: Spatial balance tracking, immutable movements, single write authority, fail-closed negative stock prevention.
-- Append-only migration. Never modify applied migrations.

-- 1. Idempotency request hash extension
ALTER TABLE idempotency_keys ADD COLUMN request_hash TEXT;

-- 2. Stock movements spatial, batch, and serial attribution
ALTER TABLE stock_movements ADD COLUMN location_id TEXT REFERENCES locations(id);
ALTER TABLE stock_movements ADD COLUMN bin_id TEXT REFERENCES bins(id);
ALTER TABLE stock_movements ADD COLUMN batch_id TEXT REFERENCES product_batches(id);
ALTER TABLE stock_movements ADD COLUMN serial_id TEXT REFERENCES serial_numbers(id);

-- 3. Serial numbers spatial attribution
ALTER TABLE serial_numbers ADD COLUMN location_id TEXT REFERENCES locations(id);
ALTER TABLE serial_numbers ADD COLUMN bin_id TEXT REFERENCES bins(id);

-- 4. Unique composite indexes on referenced tables for SQLite composite foreign keys
CREATE UNIQUE INDEX IF NOT EXISTS idx_bins_id_location_id
    ON bins(id, location_id);

CREATE UNIQUE INDEX IF NOT EXISTS idx_product_batches_id_product_branch
    ON product_batches(id, product_id, branch_id);

-- 5. Spatial inventory balances table
CREATE TABLE location_inventory (
    id TEXT PRIMARY KEY DEFAULT (lower(hex(randomblob(16)))),
    branch_id TEXT NOT NULL REFERENCES branches(id) ON DELETE RESTRICT,
    location_id TEXT NOT NULL REFERENCES locations(id) ON DELETE RESTRICT,
    bin_id TEXT REFERENCES bins(id) ON DELETE RESTRICT,
    product_id TEXT NOT NULL REFERENCES products(id) ON DELETE RESTRICT,
    variant_id TEXT REFERENCES product_variants(id) ON DELETE RESTRICT,
    batch_id TEXT REFERENCES product_batches(id) ON DELETE RESTRICT,
    quantity_milli INTEGER NOT NULL DEFAULT 0 CHECK (quantity_milli >= 0),
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now')),
    FOREIGN KEY (location_id, branch_id) REFERENCES locations(id, branch_id),
    FOREIGN KEY (bin_id, location_id) REFERENCES bins(id, location_id),
    FOREIGN KEY (batch_id, product_id, branch_id) REFERENCES product_batches(id, product_id, branch_id)
);

-- 6. Exactly 8 mutually exclusive partial unique indexes on location_inventory
CREATE UNIQUE INDEX idx_loc_inv_u1 ON location_inventory(branch_id, location_id, product_id)
    WHERE bin_id IS NULL AND variant_id IS NULL AND batch_id IS NULL;

CREATE UNIQUE INDEX idx_loc_inv_u2 ON location_inventory(branch_id, location_id, bin_id, product_id)
    WHERE bin_id IS NOT NULL AND variant_id IS NULL AND batch_id IS NULL;

CREATE UNIQUE INDEX idx_loc_inv_u3 ON location_inventory(branch_id, location_id, product_id, variant_id)
    WHERE bin_id IS NULL AND variant_id IS NOT NULL AND batch_id IS NULL;

CREATE UNIQUE INDEX idx_loc_inv_u4 ON location_inventory(branch_id, location_id, product_id, batch_id)
    WHERE bin_id IS NULL AND variant_id IS NULL AND batch_id IS NOT NULL;

CREATE UNIQUE INDEX idx_loc_inv_u5 ON location_inventory(branch_id, location_id, bin_id, product_id, variant_id)
    WHERE bin_id IS NOT NULL AND variant_id IS NOT NULL AND batch_id IS NULL;

CREATE UNIQUE INDEX idx_loc_inv_u6 ON location_inventory(branch_id, location_id, bin_id, product_id, batch_id)
    WHERE bin_id IS NOT NULL AND variant_id IS NULL AND batch_id IS NOT NULL;

CREATE UNIQUE INDEX idx_loc_inv_u7 ON location_inventory(branch_id, location_id, product_id, variant_id, batch_id)
    WHERE bin_id IS NULL AND variant_id IS NOT NULL AND batch_id IS NOT NULL;

CREATE UNIQUE INDEX idx_loc_inv_u8 ON location_inventory(branch_id, location_id, bin_id, product_id, variant_id, batch_id)
    WHERE bin_id IS NOT NULL AND variant_id IS NOT NULL AND batch_id IS NOT NULL;

-- 7. Supporting performance indexes
CREATE INDEX idx_location_inventory_lookup ON location_inventory(branch_id, product_id, location_id);
CREATE INDEX idx_location_inventory_batch ON location_inventory(batch_id);
CREATE INDEX idx_serial_numbers_location_bin ON serial_numbers(location_id, bin_id);
CREATE INDEX idx_stock_movements_location_bin ON stock_movements(location_id, bin_id);
CREATE INDEX idx_stock_movements_batch ON stock_movements(batch_id);
CREATE INDEX idx_stock_movements_serial ON stock_movements(serial_id);

-- 8. Immutability triggers on stock_movements
CREATE TRIGGER trg_stock_movements_no_update
BEFORE UPDATE ON stock_movements
BEGIN
    SELECT RAISE(ABORT, 'Historical stock_movements rows are immutable and cannot be updated');
END;

CREATE TRIGGER trg_stock_movements_no_delete
BEFORE DELETE ON stock_movements
BEGIN
    SELECT RAISE(ABORT, 'Historical stock_movements rows are immutable and cannot be deleted');
END;

-- 9. Referential and spatial consistency triggers on stock_movements
CREATE TRIGGER trg_stock_movements_spatial_guard
BEFORE INSERT ON stock_movements
BEGIN
    -- Cannot specify bin without location
    SELECT RAISE(ABORT, 'Movement bin cannot be specified without a location')
    WHERE NEW.bin_id IS NOT NULL AND NEW.location_id IS NULL;

    -- Location must belong to branch
    SELECT RAISE(ABORT, 'Movement location branch does not match movement branch')
    WHERE NEW.location_id IS NOT NULL AND NOT EXISTS (
        SELECT 1 FROM locations
        WHERE id = NEW.location_id AND branch_id = NEW.branch_id
    );

    -- Bin must belong to location
    SELECT RAISE(ABORT, 'Movement bin does not belong to movement location')
    WHERE NEW.bin_id IS NOT NULL AND NOT EXISTS (
        SELECT 1 FROM bins
        WHERE id = NEW.bin_id AND location_id = NEW.location_id
    );

    -- Batch must belong to product and branch
    SELECT RAISE(ABORT, 'Movement batch does not match product or branch')
    WHERE NEW.batch_id IS NOT NULL AND NOT EXISTS (
        SELECT 1 FROM product_batches
        WHERE id = NEW.batch_id AND product_id = NEW.product_id AND branch_id = NEW.branch_id
    );

    -- Serial must belong to product and branch
    SELECT RAISE(ABORT, 'Movement serial does not match product or branch')
    WHERE NEW.serial_id IS NOT NULL AND NOT EXISTS (
        SELECT 1 FROM serial_numbers
        WHERE id = NEW.serial_id AND product_id = NEW.product_id AND branch_id = NEW.branch_id
    );

    -- Stock movement delta cannot be zero
    SELECT RAISE(ABORT, 'Stock movement quantity delta cannot be zero')
    WHERE NEW.quantity_delta_milli = 0;

    -- If before and after quantities are provided, after must equal before plus delta
    SELECT RAISE(ABORT, 'Stock movement after quantity must equal before quantity plus delta')
    WHERE NEW.quantity_before_milli IS NOT NULL
      AND NEW.quantity_after_milli IS NOT NULL
      AND NEW.quantity_after_milli != (NEW.quantity_before_milli + NEW.quantity_delta_milli);
END;
