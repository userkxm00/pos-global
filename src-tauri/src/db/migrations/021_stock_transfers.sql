-- 021_stock_transfers.sql
-- F2.12 — Transfers Architecture
-- Append-only migration. Never modify applied migrations.

CREATE TABLE stock_transfers (
    id TEXT PRIMARY KEY DEFAULT (lower(hex(randomblob(16)))),
    transfer_number TEXT NOT NULL UNIQUE,
    transfer_type TEXT NOT NULL CHECK (transfer_type IN ('intra_branch', 'inter_branch')),
    source_branch_id TEXT NOT NULL REFERENCES branches(id) ON DELETE RESTRICT,
    destination_branch_id TEXT NOT NULL REFERENCES branches(id) ON DELETE RESTRICT,
    source_location_id TEXT NOT NULL REFERENCES locations(id) ON DELETE RESTRICT,
    destination_location_id TEXT NOT NULL REFERENCES locations(id) ON DELETE RESTRICT,
    source_bin_id TEXT,
    destination_bin_id TEXT,
    status TEXT NOT NULL DEFAULT 'draft' CHECK (status IN ('draft', 'in_transit', 'completed', 'cancelled')),
    dispatched_at TEXT,
    dispatched_by TEXT REFERENCES users(id) ON DELETE SET NULL,
    received_at TEXT,
    received_by TEXT REFERENCES users(id) ON DELETE SET NULL,
    notes TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now')),
    FOREIGN KEY (source_location_id, source_branch_id) REFERENCES locations(id, branch_id) ON DELETE RESTRICT,
    FOREIGN KEY (destination_location_id, destination_branch_id) REFERENCES locations(id, branch_id) ON DELETE RESTRICT,
    FOREIGN KEY (source_bin_id, source_location_id) REFERENCES bins(id, location_id) ON DELETE RESTRICT,
    FOREIGN KEY (destination_bin_id, destination_location_id) REFERENCES bins(id, location_id) ON DELETE RESTRICT,
    CHECK (transfer_type != 'intra_branch' OR status IN ('draft', 'completed', 'cancelled')),
    CHECK ((transfer_type = 'intra_branch' AND source_branch_id = destination_branch_id) OR (transfer_type = 'inter_branch' AND source_branch_id != destination_branch_id)),
    CHECK (transfer_type != 'intra_branch' OR NOT (source_location_id = destination_location_id AND (source_bin_id = destination_bin_id OR (source_bin_id IS NULL AND destination_bin_id IS NULL))))
);

CREATE INDEX idx_stock_transfers_source_branch ON stock_transfers(source_branch_id, status);
CREATE INDEX idx_stock_transfers_destination_branch ON stock_transfers(destination_branch_id, status);
CREATE INDEX idx_stock_transfers_status ON stock_transfers(status);

CREATE TABLE stock_transfer_items (
    id TEXT PRIMARY KEY DEFAULT (lower(hex(randomblob(16)))),
    transfer_id TEXT NOT NULL REFERENCES stock_transfers(id) ON DELETE CASCADE,
    product_id TEXT NOT NULL REFERENCES products(id) ON DELETE RESTRICT,
    variant_id TEXT REFERENCES product_variants(id) ON DELETE RESTRICT,
    batch_id TEXT REFERENCES product_batches(id) ON DELETE RESTRICT,
    serial_id TEXT REFERENCES serial_numbers(id) ON DELETE RESTRICT,
    quantity_milli INTEGER NOT NULL CHECK (quantity_milli > 0),
    received_quantity_milli INTEGER,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    CHECK (NOT (batch_id IS NOT NULL AND serial_id IS NOT NULL)),
    CHECK (serial_id IS NULL OR quantity_milli = 1000),
    CHECK (received_quantity_milli IS NULL OR received_quantity_milli = quantity_milli)
);

CREATE INDEX idx_stock_transfer_items_transfer ON stock_transfer_items(transfer_id);
CREATE INDEX idx_stock_transfer_items_product ON stock_transfer_items(product_id);

CREATE TRIGGER trg_stock_transfers_completed_receipt
BEFORE UPDATE OF status ON stock_transfers
FOR EACH ROW
WHEN NEW.status = 'completed' AND EXISTS (
    SELECT 1 FROM stock_transfer_items
    WHERE transfer_id = NEW.id AND received_quantity_milli IS NULL
)
BEGIN
    SELECT RAISE(ABORT, 'Cannot complete transfer with unreceived items');
END;
