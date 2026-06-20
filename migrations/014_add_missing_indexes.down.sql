-- Rollback migration: Remove the indexes added in 014_add_missing_indexes.sql
-- Use this to revert the migration if needed

-- Remove indexes in reverse order
DROP INDEX IF EXISTS idx_products_sold;
DROP INDEX IF EXISTS idx_escrows_status;
