-- 004_exact_money_and_identity.sql
-- Additive migration. Legacy REAL money columns remain for compatibility only;
-- *_minor columns are authoritative for new financial code.

ALTER TABLE sales ADD COLUMN subtotal_minor INTEGER NOT NULL DEFAULT 0;
ALTER TABLE sales ADD COLUMN discount_amount_minor INTEGER NOT NULL DEFAULT 0;
ALTER TABLE sales ADD COLUMN tax_amount_minor INTEGER NOT NULL DEFAULT 0;
ALTER TABLE sales ADD COLUMN total_minor INTEGER NOT NULL DEFAULT 0;
ALTER TABLE sale_items ADD COLUMN unit_price_minor INTEGER NOT NULL DEFAULT 0;
ALTER TABLE sale_items ADD COLUMN line_total_minor INTEGER NOT NULL DEFAULT 0;
ALTER TABLE sale_payments ADD COLUMN amount_minor INTEGER NOT NULL DEFAULT 0;
ALTER TABLE shifts ADD COLUMN opening_balance_minor INTEGER NOT NULL DEFAULT 0;
ALTER TABLE shifts ADD COLUMN closing_balance_minor INTEGER;

UPDATE sales SET subtotal_minor=CAST(ROUND(subtotal*100.0) AS INTEGER), discount_amount_minor=CAST(ROUND(discount_amount*100.0) AS INTEGER), tax_amount_minor=CAST(ROUND(tax_amount*100.0) AS INTEGER), total_minor=CAST(ROUND(total*100.0) AS INTEGER) WHERE subtotal_minor=0 AND (subtotal!=0 OR total!=0);
UPDATE sale_items SET unit_price_minor=CAST(ROUND(unit_price*100.0) AS INTEGER), line_total_minor=CAST(ROUND(line_total*100.0) AS INTEGER) WHERE unit_price_minor=0 AND (unit_price!=0 OR line_total!=0);
UPDATE sale_payments SET amount_minor=CAST(ROUND(amount*100.0) AS INTEGER) WHERE amount_minor=0 AND amount!=0;
UPDATE shifts SET opening_balance_minor=CAST(ROUND(opening_balance*100.0) AS INTEGER), closing_balance_minor=CASE WHEN closing_balance IS NULL THEN NULL ELSE CAST(ROUND(closing_balance*100.0) AS INTEGER) END;

ALTER TABLE users ADD COLUMN supabase_user_id TEXT;
ALTER TABLE users ADD COLUMN auth_provider TEXT NOT NULL DEFAULT 'local';
CREATE UNIQUE INDEX idx_users_supabase_user_id ON users(supabase_user_id) WHERE supabase_user_id IS NOT NULL;

CREATE TABLE permissions (
 id TEXT PRIMARY KEY DEFAULT (lower(hex(randomblob(16)))),
 code TEXT NOT NULL UNIQUE,
 description TEXT,
 created_at TEXT NOT NULL DEFAULT (datetime('now'))
);
CREATE TABLE role_permissions (
 role TEXT NOT NULL,
 permission_id TEXT NOT NULL REFERENCES permissions(id) ON DELETE CASCADE,
 PRIMARY KEY (role, permission_id)
);
CREATE TABLE user_permissions (
 user_id TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
 permission_id TEXT NOT NULL REFERENCES permissions(id) ON DELETE CASCADE,
 effect TEXT NOT NULL DEFAULT 'allow',
 PRIMARY KEY (user_id, permission_id)
);
CREATE TABLE local_sessions (
 id TEXT PRIMARY KEY DEFAULT (lower(hex(randomblob(16)))),
 user_id TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
 branch_id TEXT NOT NULL REFERENCES branches(id),
 created_at TEXT NOT NULL DEFAULT (datetime('now')),
 expires_at TEXT NOT NULL,
 revoked_at TEXT,
 auth_level TEXT NOT NULL DEFAULT 'pin'
);

INSERT INTO permissions (code,description) VALUES
('sales.create','Create sales'),('sales.refund','Refund sales'),('sales.void','Void sales'),
('inventory.adjust','Adjust inventory'),('inventory.transfer','Transfer inventory'),('products.manage','Manage products'),
('purchases.manage','Manage purchases'),('customers.manage','Manage customers'),('debts.manage','Manage customer debts'),
('cash.open','Open cash shift'),('cash.close','Close cash shift'),('cash.adjust','Adjust cash'),
('reports.view','View reports'),('reports.export','Export reports'),('users.manage','Manage users'),
('settings.manage','Manage settings'),('license.manage','Manage license');
INSERT INTO role_permissions (role,permission_id) SELECT 'admin',id FROM permissions;
INSERT INTO role_permissions (role,permission_id) SELECT 'manager',id FROM permissions WHERE code NOT IN ('users.manage','license.manage');
INSERT INTO role_permissions (role,permission_id) SELECT 'cashier',id FROM permissions WHERE code IN ('sales.create','customers.manage','reports.view','cash.open','cash.close');