-- 009_remove_redundant_product_barcode_index.sql
-- The UNIQUE(barcode) constraint already provides the lookup index.
-- Keep the applied schema lean and avoid duplicate write overhead.

DROP INDEX IF EXISTS idx_products_barcode;
