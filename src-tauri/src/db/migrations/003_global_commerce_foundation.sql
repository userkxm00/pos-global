-- 003_global_commerce_foundation.sql
-- Additive foundation for industry presets, capabilities, modules and offline events.
-- Never edit an applied migration; create a new migration instead.

CREATE TABLE industry_presets (
    id TEXT PRIMARY KEY DEFAULT (lower(hex(randomblob(16)))),
    code TEXT NOT NULL UNIQUE,
    name TEXT NOT NULL,
    description TEXT,
    is_active INTEGER NOT NULL DEFAULT 1,
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE TABLE capabilities (
    id TEXT PRIMARY KEY DEFAULT (lower(hex(randomblob(16)))),
    code TEXT NOT NULL UNIQUE,
    name TEXT NOT NULL,
    scope TEXT NOT NULL,
    description TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE TABLE business_capabilities (
    business_id TEXT NOT NULL REFERENCES business_settings(id) ON DELETE CASCADE,
    capability_id TEXT NOT NULL REFERENCES capabilities(id) ON DELETE CASCADE,
    enabled INTEGER NOT NULL DEFAULT 1,
    source TEXT NOT NULL DEFAULT 'manual',
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    PRIMARY KEY (business_id, capability_id)
);

CREATE TABLE product_capabilities (
    product_id TEXT NOT NULL REFERENCES products(id) ON DELETE CASCADE,
    capability_id TEXT NOT NULL REFERENCES capabilities(id) ON DELETE CASCADE,
    enabled INTEGER NOT NULL DEFAULT 1,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    PRIMARY KEY (product_id, capability_id)
);

CREATE TABLE modules (
    id TEXT PRIMARY KEY DEFAULT (lower(hex(randomblob(16)))),
    code TEXT NOT NULL UNIQUE,
    name TEXT NOT NULL,
    description TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE TABLE business_modules (
    business_id TEXT NOT NULL REFERENCES business_settings(id) ON DELETE CASCADE,
    module_id TEXT NOT NULL REFERENCES modules(id) ON DELETE CASCADE,
    enabled INTEGER NOT NULL DEFAULT 1,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    PRIMARY KEY (business_id, module_id)
);

CREATE TABLE units (
    id TEXT PRIMARY KEY DEFAULT (lower(hex(randomblob(16)))),
    code TEXT NOT NULL UNIQUE,
    name TEXT NOT NULL,
    dimension TEXT NOT NULL,
    precision INTEGER NOT NULL DEFAULT 3,
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE TABLE unit_conversions (
    from_unit_id TEXT NOT NULL REFERENCES units(id),
    to_unit_id TEXT NOT NULL REFERENCES units(id),
    multiplier REAL NOT NULL,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    PRIMARY KEY (from_unit_id, to_unit_id)
);

CREATE TABLE stock_movements (
    id TEXT PRIMARY KEY DEFAULT (lower(hex(randomblob(16)))),
    branch_id TEXT NOT NULL REFERENCES branches(id),
    product_id TEXT NOT NULL REFERENCES products(id),
    variant_id TEXT REFERENCES product_variants(id),
    quantity_delta REAL NOT NULL,
    quantity_before REAL,
    quantity_after REAL,
    reason TEXT NOT NULL,
    source_type TEXT,
    source_id TEXT,
    user_id TEXT REFERENCES users(id),
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);
CREATE INDEX idx_stock_movements_product ON stock_movements(branch_id, product_id, created_at);

CREATE TABLE idempotency_keys (
    key TEXT PRIMARY KEY,
    operation TEXT NOT NULL,
    result_json TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE TABLE outbox_events (
    event_id TEXT PRIMARY KEY,
    aggregate_type TEXT NOT NULL,
    aggregate_id TEXT NOT NULL,
    event_type TEXT NOT NULL,
    schema_version INTEGER NOT NULL DEFAULT 1,
    branch_id TEXT REFERENCES branches(id),
    payload_json TEXT NOT NULL,
    status TEXT NOT NULL DEFAULT 'pending',
    retry_count INTEGER NOT NULL DEFAULT 0,
    last_error TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    synced_at TEXT
);
CREATE INDEX idx_outbox_status ON outbox_events(status, created_at);

INSERT INTO capabilities (code,name,scope,description) VALUES
('SKU','SKU','product','Stock keeping unit'),
('BARCODE','Barcode','product','Barcode identification'),
('MATRIX','Matrix','product','Variant combinations'),
('COLOR','Color','product','Color attribute'),
('SIZE','Size','product','Size attribute'),
('MATERIAL','Material','product','Material attribute'),
('WEIGHT','Weight','product','Quantity by mass'),
('VOLUME','Volume','product','Quantity by volume'),
('LENGTH','Length','product','Quantity by length'),
('BATCH','Batch/Lot','product','Batch tracking'),
('EXPIRY','Expiry','product','Expiry tracking'),
('FEFO','FEFO','product','First-expire-first-out'),
('SERIAL','Serial Number','product','Unique serial tracking'),
('IMEI','IMEI','product','IMEI tracking'),
('WARRANTY','Warranty','product','Warranty tracking'),
('DIMENSIONS','Dimensions','product','Physical dimensions'),
('MULTI_PRICE','Multiple Prices','product','Multiple price lists'),
('WHOLESALE_PRICE','Wholesale Pricing','business','Wholesale pricing'),
('CUSTOMER_PRICING','Customer Pricing','business','Customer group pricing'),
('LOYALTY','Loyalty','business','Customer loyalty'),
('DELIVERY','Delivery','transaction','Delivery workflow'),
('MODIFIERS','Modifiers','transaction','Restaurant modifiers'),
('TABLES','Tables','transaction','Restaurant table workflow'),
('KITCHEN','Kitchen','transaction','Kitchen workflow'),
('APPOINTMENTS','Appointments','transaction','Appointment workflow'),
('RENTAL','Rental','transaction','Rental workflow');

INSERT INTO modules (code,name,description) VALUES
('RETAIL','Retail','Core retail POS'),
('RESTAURANT','Restaurant','Restaurant ordering and kitchen'),
('SERVICE','Service','Service tickets and appointments'),
('RENTAL','Rental','Asset rental lifecycle'),
('WHOLESALE','Wholesale','B2B ordering and pricing'),
('HOSPITALITY','Hospitality','Rooms and folios'),
('EVENTS','Events','Tickets and capacity');

INSERT INTO units (code,name,dimension,precision) VALUES
('piece','Piece','count',0),('box','Box','count',0),('pack','Pack','count',0),('set','Set','count',0),
('kg','Kilogram','mass',3),('g','Gram','mass',3),('L','Liter','volume',3),('ml','Milliliter','volume',3),
('m','Meter','length',3),('cm','Centimeter','length',3),('m2','Square meter','area',3);