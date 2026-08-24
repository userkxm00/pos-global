-- 010_registers.sql
-- F1.03 — Register / Device Model

CREATE TABLE registers (
    id TEXT PRIMARY KEY DEFAULT (lower(hex(randomblob(16)))),
    organization_id TEXT NOT NULL REFERENCES organizations(id),
    branch_id TEXT NOT NULL REFERENCES branches(id),
    name TEXT NOT NULL,
    code TEXT,
    is_active INTEGER NOT NULL DEFAULT 1,
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX idx_registers_org ON registers(organization_id);
CREATE INDEX idx_registers_branch ON registers(branch_id);
CREATE INDEX idx_registers_branch_active ON registers(branch_id, is_active);
