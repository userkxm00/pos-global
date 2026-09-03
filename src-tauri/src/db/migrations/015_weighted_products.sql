-- 015_weighted_products.sql
-- F2.06 — Weighted Products domain configuration and tare management.
-- Never edit an applied migration; create a new migration instead.

CREATE TABLE product_weight_configs (
    product_id TEXT PRIMARY KEY REFERENCES products(id) ON DELETE CASCADE,
    default_tare_milli INTEGER NOT NULL DEFAULT 0,
    min_weight_milli INTEGER,
    max_weight_milli INTEGER,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now')),
    CHECK (default_tare_milli >= 0),
    CHECK (min_weight_milli IS NULL OR min_weight_milli >= 0),
    CHECK (max_weight_milli IS NULL OR max_weight_milli >= 0),
    CHECK (min_weight_milli IS NULL OR max_weight_milli IS NULL OR min_weight_milli <= max_weight_milli)
);
