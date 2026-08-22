-- 006_quantity_precision_hardening.sql
-- Make thousandths an explicit integer source of truth for inventory and sale quantities.
-- Never edit an applied migration; create a new migration instead.

ALTER TABLE inventory ADD COLUMN quantity_milli INTEGER NOT NULL DEFAULT 0;
UPDATE inventory
SET quantity_milli = CAST(ROUND(quantity * 1000.0) AS INTEGER),
    quantity = CAST(ROUND(quantity * 1000.0) AS INTEGER) / 1000.0;

ALTER TABLE sale_items ADD COLUMN quantity_milli INTEGER NOT NULL DEFAULT 0;
UPDATE sale_items
SET quantity_milli = CAST(ROUND(quantity * 1000.0) AS INTEGER),
    quantity = CAST(ROUND(quantity * 1000.0) AS INTEGER) / 1000.0;

ALTER TABLE stock_movements ADD COLUMN quantity_delta_milli INTEGER NOT NULL DEFAULT 0;
ALTER TABLE stock_movements ADD COLUMN quantity_before_milli INTEGER;
ALTER TABLE stock_movements ADD COLUMN quantity_after_milli INTEGER;
UPDATE stock_movements
SET quantity_delta_milli = CAST(ROUND(quantity_delta * 1000.0) AS INTEGER),
    quantity_before_milli = CASE
        WHEN quantity_before IS NULL THEN NULL
        ELSE CAST(ROUND(quantity_before * 1000.0) AS INTEGER)
    END,
    quantity_after_milli = CASE
        WHEN quantity_after IS NULL THEN NULL
        ELSE CAST(ROUND(quantity_after * 1000.0) AS INTEGER)
    END,
    quantity_delta = CAST(ROUND(quantity_delta * 1000.0) AS INTEGER) / 1000.0,
    quantity_before = CASE
        WHEN quantity_before IS NULL THEN NULL
        ELSE CAST(ROUND(quantity_before * 1000.0) AS INTEGER) / 1000.0
    END,
    quantity_after = CASE
        WHEN quantity_after IS NULL THEN NULL
        ELSE CAST(ROUND(quantity_after * 1000.0) AS INTEGER) / 1000.0
    END;

CREATE INDEX idx_inventory_branch_product_quantity_milli
    ON inventory(branch_id, product_id, variant_id, quantity_milli);
