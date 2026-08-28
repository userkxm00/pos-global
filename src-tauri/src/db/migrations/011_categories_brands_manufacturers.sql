-- 011_categories_brands_manufacturers.sql
-- F2.02 — Categories, Brands, and Manufacturers taxonomy

-- 1. Additive columns for existing categories table
ALTER TABLE categories ADD COLUMN description TEXT;
ALTER TABLE categories ADD COLUMN is_active INTEGER NOT NULL DEFAULT 1;
ALTER TABLE categories ADD COLUMN updated_at TEXT NOT NULL DEFAULT (datetime('now'));

-- 2. Indexes and database-level unique constraints for categories
CREATE INDEX IF NOT EXISTS idx_categories_parent ON categories(parent_id);
CREATE INDEX IF NOT EXISTS idx_categories_is_active ON categories(is_active);
CREATE UNIQUE INDEX IF NOT EXISTS idx_categories_root_name_active 
    ON categories(name COLLATE NOCASE) 
    WHERE parent_id IS NULL AND is_active = 1;
CREATE UNIQUE INDEX IF NOT EXISTS idx_categories_sibling_name_active 
    ON categories(parent_id, name COLLATE NOCASE) 
    WHERE parent_id IS NOT NULL AND is_active = 1;

-- 3. Brands table and active uniqueness index
CREATE TABLE IF NOT EXISTS brands (
    id TEXT PRIMARY KEY DEFAULT (lower(hex(randomblob(16)))),
    name TEXT NOT NULL,
    description TEXT,
    website TEXT,
    is_active INTEGER NOT NULL DEFAULT 1,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);
CREATE INDEX IF NOT EXISTS idx_brands_is_active ON brands(is_active);
CREATE UNIQUE INDEX IF NOT EXISTS idx_brands_name_active 
    ON brands(name COLLATE NOCASE) 
    WHERE is_active = 1;

-- 4. Manufacturers table and active uniqueness index
CREATE TABLE IF NOT EXISTS manufacturers (
    id TEXT PRIMARY KEY DEFAULT (lower(hex(randomblob(16)))),
    name TEXT NOT NULL,
    description TEXT,
    website TEXT,
    support_phone TEXT,
    support_email TEXT,
    is_active INTEGER NOT NULL DEFAULT 1,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);
CREATE INDEX IF NOT EXISTS idx_manufacturers_is_active ON manufacturers(is_active);
CREATE UNIQUE INDEX IF NOT EXISTS idx_manufacturers_name_active 
    ON manufacturers(name COLLATE NOCASE) 
    WHERE is_active = 1;
