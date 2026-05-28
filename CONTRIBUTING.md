# Contributing to AgroVest Backend

Thank you for your interest in contributing to AgroVest! This document provides guidelines and instructions for contributing.

## Table of Contents

- [Code of Conduct](#code-of-conduct)
- [Getting Started](#getting-started)
- [Development Setup](#development-setup)
- [Making Changes](#making-changes)
- [Pull Request Process](#pull-request-process)
- [Coding Standards](#coding-standards)
- [Reporting Issues](#reporting-issues)

## Code of Conduct

By participating in this project, you agree to abide by our [Code of Conduct](CODE_OF_CONDUCT.md).

## Getting Started

1. Fork the repository on GitHub
2. Clone your fork locally
3. Create a branch for your changes
4. Make your changes
5. Push to your fork and submit a pull request

## Development Setup

### Prerequisites

- [Rust](https://www.rust-lang.org/tools/install) 1.70 or later
- [Docker](https://docs.docker.com/get-docker/) and Docker Compose
- [sqlx-cli](https://github.com/launchbadge/sqlx) (optional, for migrations)

### Setup Steps

```bash
# Clone your fork
git clone https://github.com/YOUR_USERNAME/AgroVest-Backend.git
cd AgroVest-Backend

# Add upstream remote
git remote add upstream https://github.com/AgroVestOfficial/AgroVest-Backend.git

# Start dependencies
docker-compose up -d postgres redis

# Copy environment file
cp .env.example .env

# Run migrations
cargo install sqlx-cli
sqlx migrate run

# Run the server
cargo run
```

### Running Tests

```bash
# Run all tests
cargo test

# Run with output
cargo test -- --nocapture

# Run specific test
cargo test test_name
```

### Code Quality

```bash
# Format code
cargo fmt

# Check formatting
cargo fmt -- --check

# Run linter
cargo clippy -- -D warnings

# Check compilation
cargo check
```

## Making Changes

### Branch Naming

Use descriptive branch names:

- `feature/add-product-categories` — new features
- `fix/escrow-status-update` — bug fixes
- `docs/update-api-docs` — documentation
- `refactor/extract-auth-middleware` — refactoring

### Commit Messages

Follow [Conventional Commits](https://www.conventionalcommits.org/):

```
feat: add product category filtering
fix: resolve escrow status update race condition
docs: update API endpoint documentation
refactor: extract auth middleware into separate module
test: add integration tests for farm service
chore: update dependencies
```

### Migration Guidelines

When adding database changes:

1. Create a new migration file in `migrations/` with the next sequence number
2. Use descriptive names: `014_add_product_tags.sql`
3. Always use `IF NOT EXISTS` / `IF EXISTS` for idempotency
4. Add appropriate indexes for new query patterns
5. Update the corresponding model in `src/models/`

## Pull Request Process

1. **Update your branch** with the latest upstream changes:
   ```bash
   git fetch upstream
   git rebase upstream/main
   ```

2. **Ensure quality checks pass**:
   ```bash
   cargo fmt -- --check
   cargo clippy -- -D warnings
   cargo check
   cargo test
   ```

3. **Write a clear PR description** explaining:
   - What the change does
   - Why it's needed
   - Any breaking changes
   - Related issues

4. **Request review** from maintainers

5. **Address feedback** promptly

### PR Title Format

Use the same conventional commit format for PR titles:

- `feat: add user profile endpoints`
- `fix: handle missing IPFS gateway gracefully`

## Coding Standards

### General

- Write idiomatic Rust
- Prefer `clarity` over `brevity` — descriptive names matter
- Handle errors explicitly — avoid `.unwrap()` in production code
- Use `tracing` for logging, not `println!`

### API Design

- Use appropriate HTTP methods (GET, POST, PUT, DELETE)
- Return consistent JSON responses
- Use pagination for list endpoints
- Validate input early
- Return meaningful error messages

### Database

- Use parameterized queries (sqlx handles this)
- Add indexes for frequently queried columns
- Use transactions for multi-step operations
- Keep migrations idempotent

### Error Handling

Use the `ApiError` enum for API errors:

```rust
use crate::error::ApiError;

// In handlers
let user = user_service::get_user(&state.db, &address)
    .await
    .map_err(|_| ApiError::NotFound)?;
```

## Reporting Issues

### Bug Reports

Include:

- Steps to reproduce
- Expected behavior
- Actual behavior
- Environment details (OS, Rust version)
- Relevant logs or error messages

### Feature Requests

Include:

- Problem description
- Proposed solution
- Alternatives considered
- Additional context

## Questions?

Open a [GitHub Discussion](https://github.com/AgroVestOfficial/AgroVest-Backend/discussions) for questions or ideas.

## License

By contributing, you agree that your contributions will be licensed under the MIT License.
