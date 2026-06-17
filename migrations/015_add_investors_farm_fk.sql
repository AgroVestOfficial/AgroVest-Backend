-- Adds the missing foreign key from investors.farm_id -> farms(id), along with
-- a positivity CHECK constraint and a supporting index. Every other cross-table
-- reference in the schema is constrained; investors.farm_id was the only omission
-- and allowed orphaned investor rows pointing at non-existent farms. Issue #8.

-- Step 1: Abort loudly if orphaned investor rows exist, so the foreign key is
-- never added against inconsistent data. Such rows must be cleaned up first.
DO $$
BEGIN
    IF EXISTS (
        SELECT 1
        FROM investors i
        LEFT JOIN farms f ON i.farm_id = f.id
        WHERE f.id IS NULL
    ) THEN
        RAISE EXCEPTION 'Orphaned investors.farm_id values reference non-existent farms. Clean up these rows before applying migration 015.';
    END IF;
END $$;

-- Step 2: Add the foreign key constraint. Deleting a farm cascades to its
-- investor rows, which are meaningless without the farm they reference.
DO $$
BEGIN
    IF NOT EXISTS (
        SELECT 1 FROM pg_constraint WHERE conname = 'fk_investors_farm_id'
    ) THEN
        ALTER TABLE investors
            ADD CONSTRAINT fk_investors_farm_id
            FOREIGN KEY (farm_id) REFERENCES farms(id)
            ON DELETE CASCADE;
    END IF;
END $$;

-- Step 3: Ensure farm_id is always a positive identifier.
DO $$
BEGIN
    IF NOT EXISTS (
        SELECT 1 FROM pg_constraint WHERE conname = 'chk_investors_farm_id_positive'
    ) THEN
        ALTER TABLE investors
            ADD CONSTRAINT chk_investors_farm_id_positive
            CHECK (farm_id > 0);
    END IF;
END $$;

-- Step 4: Index the foreign key column to keep farm-scoped lookups and the
-- ON DELETE CASCADE cleanup efficient (mirrors idx_investments_farm).
CREATE INDEX IF NOT EXISTS idx_investors_farm ON investors(farm_id);
