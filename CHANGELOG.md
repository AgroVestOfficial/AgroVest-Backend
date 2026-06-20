# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Security

- **fix(BOLA):** `GET /api/v1/escrows/{id}` now requires authentication and enforces
  ownership — only the buyer or farmer of an escrow may retrieve it. Unauthenticated
  requests return `401`; authenticated non-parties receive `404` to prevent resource
  enumeration. Service-level ownership filtering added via `get_escrow_for_user`.
  Closes [#6](https://github.com/AgroVestOfficial/AgroVest-Backend/issues/6).


## [0.1.0] - 2024-01-01

### Added

- Initial release
- Axum web framework with Tokio async runtime
- PostgreSQL database with sqlx (13 migrations)
- Redis caching layer
- Stellar Ed25519 wallet-signature authentication
- JWT session management
- User profile management
- Farm registration and management
- Product CRUD with category filtering
- Product review system
- Shopping cart functionality
- Investment opportunity creation and tracking
- Escrow system with dispute resolution
- DAO governance (proposals, voting, challenges, disputes)
- IPFS file uploads via Pinata
- Soroban blockchain event indexer
- Paginated API responses
- Docker Compose setup
- Comprehensive API documentation
- MIT License
