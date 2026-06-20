-- Rollback for migration 015: remove the investors.farm_id foreign key, the
-- positivity CHECK constraint, and the supporting index.
DROP INDEX IF EXISTS idx_investors_farm;
ALTER TABLE investors DROP CONSTRAINT IF EXISTS chk_investors_farm_id_positive;
ALTER TABLE investors DROP CONSTRAINT IF EXISTS fk_investors_farm_id;
