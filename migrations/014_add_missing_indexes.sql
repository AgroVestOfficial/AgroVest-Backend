-- Migration: Add missing database indexes for frequently queried columns
-- Issue: https://github.com/AgroVestOfficial/AgroVest-Backend/issues/7
-- Performance improvement for high-volume query patterns

-- IMPORTANT: CONCURRENTLY cannot run inside a transaction.
-- sqlx runs migrations inside a transaction by default.
-- To apply this migration in production with zero downtime, run manually:
--
-- psql $DATABASE_URL -c "CREATE INDEX CONCURRENTLY IF NOT EXISTS idx_escrows_status ON escrows(status);"
-- psql $DATABASE_URL -c "CREATE INDEX CONCURRENTLY IF NOT EXISTS idx_products_sold ON products(sold);"
--
-- For development/staging (with downtime acceptable), use the statements below:

-- Escrows: filter by status in list queries
-- Used in: escrow status filtering, dashboard analytics (escrow_service.rs:32)
-- Write impact: MEDIUM (status updated during escrow lifecycle, acceptable overhead)
CREATE INDEX IF NOT EXISTS idx_escrows_status ON escrows(status);

-- Products: filter by sold status
-- Used in: marketplace listings, inventory management (product_service.rs:26)
-- Write impact: MEDIUM (sold updated when product is purchased, acceptable overhead)
CREATE INDEX IF NOT EXISTS idx_products_sold ON products(sold);
