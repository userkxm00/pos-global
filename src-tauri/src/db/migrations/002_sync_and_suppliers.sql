-- 002_sync_and_suppliers.sql
-- Suppliers, purchasing foundation and legacy sync queue.

CREATE TABLE suppliers (
    id TEXT PRIMARY KEY DEFAULT (lower(hex(randomblob(16)))),
    name TEXT NOT NULL,
    phone TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE TABLE purchase_orders (
    id TEXT PRIMARY KEY DEFAULT (lower(hex(randomblob(16)))),
    supplier_id TEXT NOT NULL REFERENCES suppliers(id),
    branch_id TEXT NOT NULL REFERENCES branches(id),
    status TEXT NOT NULL DEFAULT 'received',
    total_cost REAL NOT NULL DEFAULT 0,
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE TABLE purchase_order_items (
    id TEXT PRIMARY KEY DEFAULT (lower(hex(randomblob(16)))),
    purchase_order_id TEXT NOT NULL REFERENCES purchase_orders(id) ON DELETE CASCADE,
    product_id TEXT NOT NULL REFERENCES products(id),
    variant_id TEXT REFERENCES product_variants(id),
    quantity REAL NOT NULL,
    unit_cost REAL NOT NULL
);

CREATE TABLE sync_queue (
    id TEXT PRIMARY KEY DEFAULT (lower(hex(randomblob(16)))),
    entity_type TEXT NOT NULL,
    entity_id TEXT NOT NULL,
    operation TEXT NOT NULL,
    payload_json TEXT NOT NULL,
    branch_id TEXT NOT NULL REFERENCES branches(id),
    status TEXT NOT NULL DEFAULT 'pending',
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    synced_at TEXT
);
CREATE INDEX idx_sync_status ON sync_queue(status);

-- New sync code should prefer the transactional outbox in migration 003.
-- Keep this table for compatibility while the migration to outbox is completed.