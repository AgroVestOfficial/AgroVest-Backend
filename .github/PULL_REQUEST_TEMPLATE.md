Closes #(issue number)

## Description

<!-- What does this PR do? Why is it needed? Reference the issue: Closes #123 -->



## Type of Change

- [ ] Bug fix (non-breaking change that fixes an issue)
- [ ] New feature (non-breaking change that adds functionality)
- [ ] Breaking change (fix or feature that would cause existing functionality to break)
- [ ] Database migration
- [ ] Documentation update
- [ ] Refactoring (no functional changes)

## Related Issues

Closes #(issue number)

## Changes Made

<!-- List the key changes. Keep it concise — reviewers can read the diff. -->

-

## Files Changed

<!-- List the files you modified and a one-line summary of each. -->

| File | Change |
|------|--------|
|      |        |

## Testing

<!-- How did you verify this works? Be honest — do not claim tests you did not run. -->

### Automated

```bash
cargo fmt -- --check
cargo clippy -- -D warnings
cargo check
cargo test
```

- [ ] All four commands pass locally

### Manual

<!-- Describe any manual testing (curl commands, database queries, psql checks). -->
<!-- If you ran EXPLAIN ANALYZE, migration verification, or similar, paste the output. -->



## Migration Notes (if applicable)

<!-- Fill this section if your PR includes a migration. Remove it if not. -->

- [ ] Migration applies cleanly with `sqlx migrate run`
- [ ] Rollback migration (`.down.sql`) is provided
- [ ] No data loss or breaking changes to existing rows

## Checklist

- [ ] My code follows the existing patterns in this codebase
- [ ] I have performed a self-review of my code
- [ ] I have added tests where appropriate
- [ ] New and existing unit tests pass locally with my changes
- [ ] I have not introduced unnecessary comments, abstractions, or features beyond the scope of this PR
- [ ] I have not committed secrets, keys, or `.env` files
- [ ] My changes generate no new warnings from `cargo clippy`

