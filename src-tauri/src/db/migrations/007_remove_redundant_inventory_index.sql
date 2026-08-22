-- 007_remove_redundant_inventory_index.sql
-- Remove the quantity-milli index added by migration 006.
-- The existing UNIQUE(branch_id, product_id, variant_id) constraint already
-- supplies the lookup path used by inventory operations; the extra index only
-- adds write amplification without a demonstrated query-plan benefit.

DROP INDEX IF EXISTS idx_inventory_branch_product_quantity_milli;
