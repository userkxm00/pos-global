-- Hardening for units and unit_conversions (F2.04)
-- Enforce case-insensitive uniqueness on unit codes at database level
CREATE UNIQUE INDEX IF NOT EXISTS idx_units_code_nocase_unique ON units(code COLLATE NOCASE);

-- Index foreign keys on unit_conversions for graph traversal and lookups
CREATE INDEX IF NOT EXISTS idx_unit_conversions_to_unit_id ON unit_conversions(to_unit_id);
