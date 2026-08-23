-- 008_inventory_and_schema_hardening.sql
-- Harden inventory identity without editing applied migrations.
-- Applied migrations are append-only.

-- SQLite UNIQUE permits multiple NULL variant_id values, so the legacy
-- inventory table could contain multiple non-variant rows for the same
-- branch/product pair. Consolidate those rows before adding the invariant.
UPDATE inventory
SET
    quantity = (
        SELECT SUM(i2.quantity)
        FROM inventory AS i2
        WHERE i2.branch_id = inventory.branch_id
          AND i2.product_id = inventory.product_id
          AND i2.variant_id IS NULL
    ),
    quantity_milli = (
        SELECT SUM(i2.quantity_milli)
        FROM inventory AS i2
        WHERE i2.branch_id = inventory.branch_id
          AND i2.product_id = inventory.product_id
          AND i2.variant_id IS NULL
    ),
    low_stock_threshold = (
        SELECT MAX(i2.low_stock_threshold)
        FROM inventory AS i2
        WHERE i2.branch_id = inventory.branch_id
          AND i2.product_id = inventory.product_id
          AND i2.variant_id IS NULL
    ),
    updated_at = (
        SELECT MAX(i2.updated_at)
        FROM inventory AS i2
        WHERE i2.branch_id = inventory.branch_id
          AND i2.product_id = inventory.product_id
          AND i2.variant_id IS NULL
    )
WHERE inventory.variant_id IS NULL
  AND inventory.id IN (
      SELECT MIN(i3.id)
      FROM inventory AS i3
      WHERE i3.variant_id IS NULL
      GROUP BY i3.branch_id, i3.product_id
      HAVING COUNT(*) > 1
  );

DELETE FROM inventory
WHERE inventory.variant_id IS NULL
  AND inventory.id NOT IN (
      SELECT MIN(i4.id)
      FROM inventory AS i4
      WHERE i4.variant_id IS NULL
      GROUP BY i4.branch_id, i4.product_id
  );

-- Enforce the intended inventory identity:
--   * one non-variant stock row per branch/product
--   * one variant stock row per branch/product/variant
CREATE UNIQUE INDEX IF NOT EXISTS ux_inventory_branch_product_no_variant
    ON inventory(branch_id, product_id)
    WHERE variant_id IS NULL;

CREATE UNIQUE INDEX IF NOT EXISTS ux_inventory_branch_product_variant
    ON inventory(branch_id, product_id, variant_id)
    WHERE variant_id IS NOT NULL;

-- Prepare efficient organization-scoped lookups without making tenant
-- ownership authoritative before the organization/auth phases define it.
CREATE INDEX IF NOT EXISTS idx_business_org
    ON business_settings(organization_id);

CREATE INDEX IF NOT EXISTS idx_branch_org_active
    ON branches(organization_id, is_active);
