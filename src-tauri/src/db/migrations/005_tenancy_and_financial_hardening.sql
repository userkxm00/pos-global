-- 005_tenancy_and_financial_hardening.sql
-- Additive hardening after the initial prototype schema.

CREATE TABLE organizations (
    id TEXT PRIMARY KEY DEFAULT (lower(hex(randomblob(16)))),
    name TEXT NOT NULL,
    default_currency TEXT NOT NULL DEFAULT 'USD',
    default_language TEXT NOT NULL DEFAULT 'en',
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);

ALTER TABLE business_settings ADD COLUMN organization_id TEXT REFERENCES organizations(id);
CREATE INDEX idx_business_org ON business_settings(organization_id);

ALTER TABLE branches ADD COLUMN organization_id TEXT REFERENCES organizations(id);
CREATE INDEX idx_branch_org ON branches(organization_id);

ALTER TABLE units ADD COLUMN is_base INTEGER NOT NULL DEFAULT 0;

ALTER TABLE purchase_orders ADD COLUMN total_cost_minor INTEGER NOT NULL DEFAULT 0;
ALTER TABLE purchase_order_items ADD COLUMN unit_cost_minor INTEGER NOT NULL DEFAULT 0;

-- New purchase code must use *_minor fields. Legacy REAL cost fields remain only
-- as compatibility columns until a future breaking migration removes them.
UPDATE purchase_orders
SET total_cost_minor = CAST(ROUND(total_cost * 100.0) AS INTEGER)
WHERE total_cost_minor = 0 AND total_cost != 0;
UPDATE purchase_order_items
SET unit_cost_minor = CAST(ROUND(unit_cost * 100.0) AS INTEGER)
WHERE unit_cost_minor = 0 AND unit_cost != 0;

CREATE TABLE cash_movements (
    id TEXT PRIMARY KEY DEFAULT (lower(hex(randomblob(16)))),
    shift_id TEXT NOT NULL REFERENCES shifts(id),
    branch_id TEXT NOT NULL REFERENCES branches(id),
    movement_type TEXT NOT NULL,
    amount_minor INTEGER NOT NULL,
    currency TEXT NOT NULL,
    reason TEXT,
    reference_type TEXT,
    reference_id TEXT,
    user_id TEXT REFERENCES users(id),
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);
CREATE INDEX idx_cash_movements_shift ON cash_movements(shift_id, created_at);

CREATE TABLE debt_ledger (
    id TEXT PRIMARY KEY DEFAULT (lower(hex(randomblob(16)))),
    customer_id TEXT NOT NULL REFERENCES customers(id),
    branch_id TEXT NOT NULL REFERENCES branches(id),
    entry_type TEXT NOT NULL,
    amount_minor INTEGER NOT NULL,
    currency TEXT NOT NULL,
    source_type TEXT,
    source_id TEXT,
    user_id TEXT REFERENCES users(id),
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);
CREATE INDEX idx_debt_ledger_customer ON debt_ledger(customer_id, created_at);

CREATE TABLE loyalty_ledger (
    id TEXT PRIMARY KEY DEFAULT (lower(hex(randomblob(16)))),
    customer_id TEXT NOT NULL REFERENCES customers(id),
    points_delta INTEGER NOT NULL,
    reason TEXT NOT NULL,
    source_type TEXT,
    source_id TEXT,
    user_id TEXT REFERENCES users(id),
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);
CREATE INDEX idx_loyalty_customer ON loyalty_ledger(customer_id, created_at);
