PRAGMA foreign_keys = ON;

CREATE TABLE business_settings (
    id TEXT PRIMARY KEY DEFAULT (lower(hex(randomblob(16)))),
    business_name TEXT NOT NULL,
    business_type TEXT NOT NULL DEFAULT 'general',
    default_currency TEXT NOT NULL DEFAULT 'DZD',
    default_language TEXT NOT NULL DEFAULT 'en',
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE TABLE branches (
    id TEXT PRIMARY KEY DEFAULT (lower(hex(randomblob(16)))),
    name TEXT NOT NULL,
    address TEXT,
    currency TEXT NOT NULL,
    is_active INTEGER NOT NULL DEFAULT 1,
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE TABLE users (
    id TEXT PRIMARY KEY DEFAULT (lower(hex(randomblob(16)))),
    branch_id TEXT NOT NULL REFERENCES branches(id),
    full_name TEXT NOT NULL,
    username TEXT UNIQUE,
    password_hash TEXT,
    pin_hash TEXT,
    role TEXT NOT NULL,
    is_active INTEGER NOT NULL DEFAULT 1,
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE TABLE shifts (
    id TEXT PRIMARY KEY DEFAULT (lower(hex(randomblob(16)))),
    branch_id TEXT NOT NULL REFERENCES branches(id),
    user_id TEXT NOT NULL REFERENCES users(id),
    opening_balance REAL NOT NULL DEFAULT 0,
    closing_balance REAL,
    opened_at TEXT NOT NULL DEFAULT (datetime('now')),
    closed_at TEXT,
    status TEXT NOT NULL DEFAULT 'open'
);

CREATE TABLE categories (
    id TEXT PRIMARY KEY DEFAULT (lower(hex(randomblob(16)))),
    name TEXT NOT NULL,
    parent_id TEXT REFERENCES categories(id),
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE TABLE products (
    id TEXT PRIMARY KEY DEFAULT (lower(hex(randomblob(16)))),
    category_id TEXT REFERENCES categories(id),
    name TEXT NOT NULL,
    description TEXT,
    barcode TEXT UNIQUE,
    product_type TEXT NOT NULL DEFAULT 'simple',
    base_price REAL NOT NULL DEFAULT 0,
    cost_price REAL DEFAULT 0,
    unit_type TEXT,
    requires_expiry INTEGER NOT NULL DEFAULT 0,
    requires_serial INTEGER NOT NULL DEFAULT 0,
    warranty_months INTEGER,
    custom_attributes TEXT,
    is_active INTEGER NOT NULL DEFAULT 1,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);
CREATE INDEX idx_products_category ON products(category_id);
CREATE INDEX idx_products_barcode ON products(barcode);

CREATE TABLE attribute_definitions (
    id TEXT PRIMARY KEY DEFAULT (lower(hex(randomblob(16)))),
    name TEXT NOT NULL UNIQUE
);
CREATE TABLE attribute_values (
    id TEXT PRIMARY KEY DEFAULT (lower(hex(randomblob(16)))),
    attribute_definition_id TEXT NOT NULL REFERENCES attribute_definitions(id),
    value TEXT NOT NULL,
    UNIQUE(attribute_definition_id, value)
);
CREATE TABLE product_variants (
    id TEXT PRIMARY KEY DEFAULT (lower(hex(randomblob(16)))),
    product_id TEXT NOT NULL REFERENCES products(id) ON DELETE CASCADE,
    sku TEXT UNIQUE,
    barcode TEXT UNIQUE,
    price_override REAL,
    is_active INTEGER NOT NULL DEFAULT 1
);
CREATE TABLE variant_attribute_values (
    variant_id TEXT NOT NULL REFERENCES product_variants(id) ON DELETE CASCADE,
    attribute_value_id TEXT NOT NULL REFERENCES attribute_values(id),
    PRIMARY KEY (variant_id, attribute_value_id)
);

CREATE TABLE inventory (
    id TEXT PRIMARY KEY DEFAULT (lower(hex(randomblob(16)))),
    branch_id TEXT NOT NULL REFERENCES branches(id),
    product_id TEXT NOT NULL REFERENCES products(id),
    variant_id TEXT REFERENCES product_variants(id),
    quantity REAL NOT NULL DEFAULT 0,
    low_stock_threshold REAL DEFAULT 0,
    updated_at TEXT NOT NULL DEFAULT (datetime('now')),
    UNIQUE(branch_id, product_id, variant_id)
);
CREATE TABLE product_batches (
    id TEXT PRIMARY KEY DEFAULT (lower(hex(randomblob(16)))),
    product_id TEXT NOT NULL REFERENCES products(id),
    branch_id TEXT NOT NULL REFERENCES branches(id),
    batch_number TEXT,
    quantity REAL NOT NULL DEFAULT 0,
    expiry_date TEXT NOT NULL,
    received_at TEXT NOT NULL DEFAULT (datetime('now'))
);
CREATE INDEX idx_batches_expiry ON product_batches(expiry_date);
CREATE TABLE serial_numbers (
    id TEXT PRIMARY KEY DEFAULT (lower(hex(randomblob(16)))),
    product_id TEXT NOT NULL REFERENCES products(id),
    branch_id TEXT NOT NULL REFERENCES branches(id),
    serial_number TEXT NOT NULL UNIQUE,
    status TEXT NOT NULL DEFAULT 'in_stock',
    sold_in_sale_id TEXT,
    warranty_expires_at TEXT
);

CREATE TABLE customers (
    id TEXT PRIMARY KEY DEFAULT (lower(hex(randomblob(16)))),
    full_name TEXT NOT NULL,
    phone TEXT,
    loyalty_points INTEGER NOT NULL DEFAULT 0,
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);
CREATE TABLE customer_debts (
    id TEXT PRIMARY KEY DEFAULT (lower(hex(randomblob(16)))),
    customer_id TEXT NOT NULL REFERENCES customers(id),
    branch_id TEXT NOT NULL REFERENCES branches(id),
    shift_id TEXT NOT NULL REFERENCES shifts(id),
    sale_id TEXT,
    amount REAL NOT NULL,
    remaining_amount REAL NOT NULL,
    status TEXT NOT NULL DEFAULT 'unpaid',
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);
CREATE TABLE debt_payments (
    id TEXT PRIMARY KEY DEFAULT (lower(hex(randomblob(16)))),
    debt_id TEXT NOT NULL REFERENCES customer_debts(id),
    shift_id TEXT NOT NULL REFERENCES shifts(id),
    amount_paid REAL NOT NULL,
    paid_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE TABLE sales (
    id TEXT PRIMARY KEY DEFAULT (lower(hex(randomblob(16)))),
    branch_id TEXT NOT NULL REFERENCES branches(id),
    shift_id TEXT NOT NULL REFERENCES shifts(id),
    user_id TEXT NOT NULL REFERENCES users(id),
    customer_id TEXT REFERENCES customers(id),
    currency TEXT NOT NULL,
    subtotal REAL NOT NULL,
    discount_amount REAL NOT NULL DEFAULT 0,
    tax_amount REAL NOT NULL DEFAULT 0,
    total REAL NOT NULL,
    status TEXT NOT NULL DEFAULT 'completed',
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);
CREATE INDEX idx_sales_shift ON sales(shift_id);
CREATE INDEX idx_sales_branch_date ON sales(branch_id, created_at);
CREATE TABLE sale_items (
    id TEXT PRIMARY KEY DEFAULT (lower(hex(randomblob(16)))),
    sale_id TEXT NOT NULL REFERENCES sales(id) ON DELETE CASCADE,
    product_id TEXT NOT NULL REFERENCES products(id),
    variant_id TEXT REFERENCES product_variants(id),
    quantity REAL NOT NULL,
    unit_price REAL NOT NULL,
    line_total REAL NOT NULL
);
CREATE TABLE sale_payments (
    id TEXT PRIMARY KEY DEFAULT (lower(hex(randomblob(16)))),
    sale_id TEXT NOT NULL REFERENCES sales(id) ON DELETE CASCADE,
    payment_method TEXT NOT NULL,
    amount REAL NOT NULL
);

CREATE TABLE currencies (
    code TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    symbol TEXT NOT NULL,
    decimal_places INTEGER NOT NULL DEFAULT 2
);
CREATE TABLE tax_rules (
    id TEXT PRIMARY KEY DEFAULT (lower(hex(randomblob(16)))),
    branch_id TEXT NOT NULL REFERENCES branches(id),
    name TEXT NOT NULL,
    rate_percent REAL NOT NULL,
    applies_to_category_id TEXT REFERENCES categories(id)
);

CREATE TABLE audit_log (
    id TEXT PRIMARY KEY DEFAULT (lower(hex(randomblob(16)))),
    user_id TEXT REFERENCES users(id),
    action TEXT NOT NULL,
    entity_type TEXT NOT NULL,
    entity_id TEXT NOT NULL,
    details_json TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);