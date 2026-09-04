-- 019_locations_bins.sql
-- F2.10 — Locations and Bins Master Data Architecture
-- Append-only migration. Never modify applied migrations.

-- 1. Locations table: macroscopic physical storage areas/zones within a branch
CREATE TABLE locations (
    id TEXT PRIMARY KEY DEFAULT (lower(hex(randomblob(16)))),
    branch_id TEXT NOT NULL REFERENCES branches(id) ON DELETE RESTRICT,
    parent_id TEXT REFERENCES locations(id) ON DELETE RESTRICT,
    name TEXT NOT NULL,
    code TEXT NOT NULL,
    location_type TEXT,
    is_active INTEGER NOT NULL DEFAULT 1 CHECK (is_active IN (0, 1)),
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);

-- Compound case-insensitive uniqueness scoped to branch
CREATE UNIQUE INDEX idx_locations_branch_code
    ON locations(branch_id, code COLLATE NOCASE);

-- Foreign key lookup performance indexes
CREATE INDEX idx_locations_branch_id
    ON locations(branch_id);

CREATE INDEX idx_locations_parent_id
    ON locations(parent_id);

-- 2. Bins table: addressable physical pick/put slots belonging to a location
CREATE TABLE bins (
    id TEXT PRIMARY KEY DEFAULT (lower(hex(randomblob(16)))),
    location_id TEXT NOT NULL REFERENCES locations(id) ON DELETE RESTRICT,
    name TEXT NOT NULL,
    code TEXT NOT NULL,
    is_active INTEGER NOT NULL DEFAULT 1 CHECK (is_active IN (0, 1)),
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);

-- Compound case-insensitive uniqueness scoped to parent location
CREATE UNIQUE INDEX idx_bins_location_code
    ON bins(location_id, code COLLATE NOCASE);

-- Foreign key lookup performance index
CREATE INDEX idx_bins_location_id
    ON bins(location_id);
