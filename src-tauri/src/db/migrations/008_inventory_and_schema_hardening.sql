-- 008_inventory_and_schema_hardening.sql
-- Harden inventory identity without editing applied migrations.
-- Also make tenant scope explicit for future organization enforcement.

-- SQLite UNIQUE permits multiple NULL variant_id values. The partial unique
-- indexes below enforce the intended identity for variant and non-variant stock.
CREATE UNIQUE INDEX IF NOT EXISTS ux_inventory_branch_product_no_variant
    ON inventory(branch_id, product_id)
    WHERE variant_id IS NULL;

CREATE UNIQUE INDEX IF NOT EXISTS ux_inventory_branch_product_variant
    ON inventory(branch_id, product_id, variant_id)
    WHERE variant_id IS NOT NULL;

-- Preserve the tenancy model while keeping existing rows valid. Future tenant
-- enforcement is owned by the organization model/auth phases and should not be
-- inferred from this migration alone.
CREATE INDEX IF NOT EXISTS idx_business_org_branch_lookup
    ON business_settings(organization_id, id);

CREATE INDEX IF NOT EXISTS idx_branch_org_active
    ON branches(organization_id, is_active);
