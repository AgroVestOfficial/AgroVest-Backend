# Database Performance Optimization

## Overview

This document outlines the database indexes and performance optimizations implemented in the AgroVest Backend to ensure efficient query execution as the platform scales.

## Indexes Added (Migration 014)

### Primary Performance Indexes

| Table | Column | Index Name | Query Pattern | Performance Impact |
|-------|--------|------------|---------------|-------------------|
| `proposals` | `proposer` | `idx_proposals_proposer` | User's proposal listings | Eliminates full table scan for user-specific proposals |
| `challenges` | `proposal_id` | `idx_challenges_proposal_id` | Proposal→Challenge JOINs | Accelerates proposal detail views with challenges |
| `disputes` | `challenge_id` | `idx_disputes_challenge_id` | Challenge→Dispute JOINs | Speeds up dispute resolution workflows |
| `votes` | `voter` | `idx_votes_voter` | User voting history | Enables fast user vote lookups and verification |
| `escrows` | `status` | `idx_escrows_status` | Status filtering in listings | Critical for escrow dashboard analytics |
| `products` | `sold` | `idx_products_sold` | Marketplace availability | Essential for product inventory management |
| `investments` | `owner` | `idx_investments_owner` | Ownership verification | Optimizes user investment portfolio queries |

## Existing Indexes (Pre-Migration 014)

### Already Optimized Tables

| Table | Existing Indexes | Coverage |
|-------|------------------|----------|
| `escrows` | `idx_escrows_buyer`, `idx_escrows_farmer` | User role-based filtering ✅ |
| `investments` | `idx_investments_farm`, `idx_investments_active` | Farm-based and active filtering ✅ |
| `products` | `idx_products_owner`, `idx_products_farm`, `idx_products_category` | Ownership, farm, and category filtering ✅ |

## Query Performance Analysis

### Expected Performance Improvements

#### 1. Escrow Status Filtering
**Before Index:**
```sql
-- Full table scan on large escrows table
SELECT * FROM escrows WHERE status = 'awaiting_approval';
-- Expected: Sequential scan through entire table
```

**After Index (`idx_escrows_status`):**
```sql
-- Index scan, only examines matching rows
SELECT * FROM escrows WHERE status = 'awaiting_approval';
-- Expected: Index scan with significant improvement for large datasets
```

#### 2. User Proposal Lookups
**Before Index:**
```sql
-- Full table scan to find user's proposals
SELECT * FROM proposals WHERE proposer = 'GAB...XYZ';
-- Expected: Sequential scan through entire proposals table
```

**After Index (`idx_proposals_proposer`):**
```sql
-- Direct index lookup
SELECT * FROM proposals WHERE proposer = 'GAB...XYZ';
-- Expected: B-tree index lookup, logarithmic time complexity
```

#### 3. Marketplace Product Filtering
**Before Index:**
```sql
-- Full scan to find available products
SELECT * FROM products WHERE sold = false ORDER BY created_at DESC;
-- Expected: Sequential scan with filter
```

**After Index (`idx_products_sold`):**
```sql
-- Index-optimized filtering
SELECT * FROM products WHERE sold = false ORDER BY created_at DESC;
-- Expected: Index scan for boolean filter, significant improvement
```

## Composite Index Analysis

### Analysis of Potential Composite Indexes

#### 1. Products Table: `products(category, sold)`
**Query Pattern**: `WHERE category = $1 AND sold = $2`
**Current Solution**: Separate indexes on `category` (existing) and `sold` (new)
**Analysis**: For queries filtering by both category and sold status, a composite index would be more efficient. However, the current separate indexes provide flexibility for queries filtering by either column independently. **Recommendation**: Consider composite index `(category, sold)` in future optimization if category+sold queries become frequent.

#### 2. Escrows Table: `escrows(buyer, status)` and `escrows(farmer, status)`
**Query Pattern**: `WHERE buyer = $1 AND status = $2` or `WHERE farmer = $1 AND status = $2`
**Current Solution**: Separate indexes on `buyer`, `farmer` (existing) and `status` (new)
**Analysis**: These would benefit from composite indexes for role-specific status filtering. However, status filtering across all escrows (current dashboard requirement) uses the single-column status index effectively. **Recommendation**: Add composite indexes `(buyer, status)` and `(farmer, status)` in follow-up PR for user-specific escrow dashboards.

#### 3. General Recommendation
Single-column indexes chosen for initial implementation provide broad query coverage and flexibility. Composite indexes should be added based on specific query patterns observed in production usage.

## Write Performance Impact Analysis

### Index Maintenance Overhead

#### Low-Impact Columns (Immutable after insert)
- **`proposer`**: Set once when proposal is created, never updated
- **`voter`**: Set once when vote is cast, never updated  
- **`proposal_id`**: Foreign key, set once on challenge creation
- **`challenge_id`**: Foreign key, set once on dispute creation
- **`owner`**: Set once on investment creation, rarely transferred
- **Write Impact**: Minimal - only affects INSERT operations

#### Medium-Impact Columns (Updated during lifecycle)
- **`status`**: Updated as escrows progress through states (pending → active → completed)
- **`sold`**: Updated when products are purchased (false → true, typically once)
- **Write Impact**: Acceptable - boolean/enum updates are fast, index maintenance overhead is minimal compared to query performance gains

#### Overhead Assessment
Index maintenance adds ~5-10% overhead to write operations but provides 10-100x improvement in read performance for filtered queries. Given that most applications are read-heavy (typical 80/20 read/write ratio), this tradeoff is highly beneficial.

## Performance Monitoring

### Key Metrics to Track

1. **Query Execution Time**
   - Target: <50ms for listing endpoints
   - Target: <10ms for single-record lookups

2. **Database Load**
   - Monitor index hit ratio (target: >95%)
   - Track sequential scan frequency (target: minimize)

3. **Index Usage**
   - Verify indexes are being used with `EXPLAIN ANALYZE`
   - Monitor index size vs. table size ratio

### Monitoring Queries

```sql
-- Check index usage statistics
SELECT 
    schemaname,
    tablename,
    indexname,
    idx_scan,
    idx_tup_read,
    idx_tup_fetch
FROM pg_stat_user_indexes
WHERE idx_scan > 0
ORDER BY idx_scan DESC;

-- Identify slow queries
SELECT 
    query,
    calls,
    total_time,
    mean_time,
    rows
FROM pg_stat_statements
WHERE mean_time > 100
ORDER BY mean_time DESC;
```

## Best Practices

### 1. Index Maintenance
- Use `IF NOT EXISTS` for idempotent migrations
- Provide rollback migrations for index removal
- Monitor index bloat and perform periodic maintenance

### 2. Query Optimization
- Always include indexes for columns used in WHERE clauses
- Consider composite indexes for multi-column filtering
- Use EXPLAIN ANALYZE to verify query plans

### 3. Future Considerations
- Monitor query patterns as the application grows
- Consider partitioning for very large tables (1M+ records)
- Implement query result caching for frequently accessed data

## Migration Management

### Running Index Migrations
```bash
# Apply new indexes
sqlx migrate run

# Rollback if needed (requires .down.sql files)
sqlx migrate revert
```

### Verification Steps
1. Check migration was applied successfully
2. Verify indexes exist: `\di` in psql
3. Test query performance with `EXPLAIN ANALYZE`
4. Monitor application response times

## Impact Assessment

| Metric | Expected Improvement | Basis |
|--------|---------------------|-------|
| Query Performance | Index scan vs sequential scan | Fundamental database optimization principle |
| Database CPU Usage | Reduced | Less data scanning required |
| Concurrent User Capacity | Increased | Faster query execution allows higher throughput |

**Note**: Specific performance numbers will vary based on data volume, query patterns, and hardware. The above represents expected improvements based on index scan vs sequential scan operations.

## Conclusion

The addition of these strategic indexes significantly improves AgroVest's database performance, ensuring the platform can scale efficiently as user adoption grows. Regular monitoring and maintenance will ensure these optimizations continue to provide value as query patterns evolve.
