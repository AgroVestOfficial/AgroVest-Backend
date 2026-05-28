# AgroVest Backend

[![CI](https://github.com/AgroVestOfficial/AgroVest-Backend/actions/workflows/ci.yml/badge.svg)](https://github.com/AgroVestOfficial/AgroVest-Backend/actions/workflows/ci.yml)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)
[![Rust](https://img.shields.io/badge/Rust-1.70%2B-orange.svg)](https://www.rust-lang.org/)

Backend API server for **AgroVest** — a Web3 agricultural investment and marketplace platform built on [Stellar Soroban](https://soroban.stellar.org/).

AgroVest enables African farmers to tokenize their businesses, attract investors, and sell products on a decentralized marketplace. This backend provides REST APIs, blockchain event indexing, IPFS file storage, and wallet-based authentication.

## Architecture

```
┌─────────────┐     ┌──────────────────┐     ┌─────────────────┐
│   Frontend   │────▶│  AgroVest API    │────▶│   PostgreSQL    │
│  (Next.js)   │◀────│   (Axum/Rust)    │────▶│     Redis       │
└─────────────┘     └──────┬───────────┘     └─────────────────┘
                           │
                    ┌──────┴───────┐
                    │              │
              ┌─────▼─────┐  ┌────▼─────┐
              │  Soroban   │  │  IPFS    │
              │  RPC Node  │  │ (Pinata) │
              └───────────┘  └──────────┘
```

## Features

- **REST API** — Full CRUD for users, farms, products, investments, escrows, and DAO governance
- **Wallet Authentication** — Stellar Ed25519 signature verification with JWT sessions
- **Blockchain Indexer** — Background Soroban event polling and PostgreSQL sync
- **IPFS Uploads** — File pinning via Pinata with metadata tracking
- **Redis Caching** — Query result caching, auth nonces, and rate limiting
- **Pagination** — Consistent paginated responses across all list endpoints
- **Docker Ready** — Multi-stage Dockerfile and docker-compose for one-command setup

## Tech Stack

| Layer | Technology |
|-------|------------|
| Framework | [Axum](https://github.com/tokio-rs/axum) 0.7 |
| Runtime | [Tokio](https://tokio.rs/) 1.x |
| Database | [PostgreSQL](https://www.postgresql.org/) 16 via [sqlx](https://github.com/launchbadge/sqlx) |
| Cache | [Redis](https://redis.io/) 7 |
| Auth | Ed25519 signatures + [jsonwebtoken](https://github.com/Keats/jsonwebtoken) |
| Blockchain | [Soroban RPC](https://soroban.stellar.org/) via reqwest |
| IPFS | [Pinata](https://www.pinata.cloud/) API |
| Serialization | [serde](https://serde.rs/) + [serde_json](https://github.com/serde-rs/json) |

## Quick Start

### Prerequisites

- [Rust](https://www.rust-lang.org/tools/install) 1.70+
- [Docker](https://docs.docker.com/get-docker/) and Docker Compose
- A Stellar account (for contract interaction)

### 1. Clone the repository

```bash
git clone https://github.com/AgroVestOfficial/AgroVest-Backend.git
cd AgroVest-Backend
```

### 2. Start dependencies

```bash
docker-compose up -d postgres redis
```

### 3. Configure environment

```bash
cp .env.example .env
# Edit .env with your configuration
```

### 4. Run migrations

The application will connect to the database on startup. To run migrations manually:

```bash
cargo install sqlx-cli
sqlx migrate run
```

### 5. Start the server

```bash
cargo run
```

The API will be available at `http://localhost:8080`.

### Docker (full stack)

```bash
docker-compose up --build
```

## Environment Variables

| Variable | Description | Default |
|----------|-------------|---------|
| `DATABASE_URL` | PostgreSQL connection string | `postgres://postgres:postgres@localhost:5432/agrovest` |
| `REDIS_URL` | Redis connection string | `redis://127.0.0.1:6379` |
| `JWT_SECRET` | Secret key for JWT signing | *required* |
| `JWT_EXPIRATION_HOURS` | JWT token lifetime | `24` |
| `PINATA_API_KEY` | Pinata API key for IPFS | *optional* |
| `PINATA_SECRET_KEY` | Pinata secret key | *optional* |
| `IPFS_BACKEND` | IPFS provider (`pinata` or `local`) | `pinata` |
| `IPFS_GATEWAY_URL` | IPFS gateway URL | `https://gateway.pinata.cloud/ipfs` |
| `SOROBAN_RPC_URL` | Soroban RPC endpoint | `https://soroban-testnet.stellar.org` |
| `FARM_CONTRACT_ADDRESS` | Deployed Farm contract address | *optional* |
| `INVESTMENT_CONTRACT_ADDRESS` | Deployed Investment contract address | *optional* |
| `ESCROW_CONTRACT_ADDRESS` | Deployed Escrow contract address | *optional* |
| `DAO_CONTRACT_ADDRESS` | Deployed DAO contract address | *optional* |
| `INDEXER_POLL_INTERVAL_SECS` | Blockchain polling interval | `5` |
| `SERVER_HOST` | Server bind address | `0.0.0.0` |
| `SERVER_PORT` | Server port | `8080` |
| `CORS_ORIGINS` | Allowed CORS origins (comma-separated) | `http://localhost:3000` |

## API Endpoints

### Authentication
| Method | Path | Auth | Description |
|--------|------|------|-------------|
| `POST` | `/api/v1/auth/nonce` | No | Generate nonce for wallet signing |
| `POST` | `/api/v1/auth/verify` | No | Verify signature, get JWT |
| `GET` | `/api/v1/auth/me` | Yes | Get current user info |

### Users
| Method | Path | Auth | Description |
|--------|------|------|-------------|
| `GET` | `/api/v1/users/:address` | No | Get user profile |
| `PUT` | `/api/v1/users/:address` | Yes | Update own profile |

### Farms
| Method | Path | Auth | Description |
|--------|------|------|-------------|
| `GET` | `/api/v1/farms` | No | List farms (paginated) |
| `GET` | `/api/v1/farms/:id` | No | Get farm by ID |
| `GET` | `/api/v1/farms/by-address/:address` | No | Get farm by owner address |
| `POST` | `/api/v1/farms` | Yes | Register a new farm |
| `PUT` | `/api/v1/farms/:id` | Yes | Update farm (owner only) |

### Products
| Method | Path | Auth | Description |
|--------|------|------|-------------|
| `GET` | `/api/v1/products` | No | List products (filterable) |
| `GET` | `/api/v1/products/:id` | No | Get product by ID |
| `GET` | `/api/v1/farms/:farm_id/products` | No | Get farm products |
| `POST` | `/api/v1/products` | Yes | Add a product |
| `PUT` | `/api/v1/products/:id` | Yes | Update product (owner only) |

### Reviews
| Method | Path | Auth | Description |
|--------|------|------|-------------|
| `GET` | `/api/v1/products/:id/reviews` | No | Get product reviews |
| `POST` | `/api/v1/products/:id/reviews` | Yes | Submit a review |

### Cart
| Method | Path | Auth | Description |
|--------|------|------|-------------|
| `GET` | `/api/v1/cart` | Yes | Get user's cart |
| `POST` | `/api/v1/cart` | Yes | Add item to cart |
| `DELETE` | `/api/v1/cart/:product_id` | Yes | Remove item from cart |
| `DELETE` | `/api/v1/cart` | Yes | Clear cart |

### Investments
| Method | Path | Auth | Description |
|--------|------|------|-------------|
| `GET` | `/api/v1/investments` | No | List investments |
| `GET` | `/api/v1/investments/:id` | No | Get investment details |
| `GET` | `/api/v1/investments/:id/investors` | No | Get investors |
| `POST` | `/api/v1/investments` | Yes | Create investment opportunity |
| `POST` | `/api/v1/investments/:id/invest` | Yes | Invest in a farm |

### Escrows
| Method | Path | Auth | Description |
|--------|------|------|-------------|
| `GET` | `/api/v1/escrows` | Yes | List user's escrows |
| `GET` | `/api/v1/escrows/:id` | No | Get escrow details |
| `POST` | `/api/v1/escrows` | Yes | Create escrow |
| `PUT` | `/api/v1/escrows/:id/approve` | Yes | Approve delivery |
| `PUT` | `/api/v1/escrows/:id/dispute` | Yes | Raise dispute |

### DAO Governance
| Method | Path | Auth | Description |
|--------|------|------|-------------|
| `GET` | `/api/v1/proposals` | No | List proposals |
| `GET` | `/api/v1/proposals/:id` | No | Get proposal details |
| `POST` | `/api/v1/proposals` | Yes | Create proposal |
| `POST` | `/api/v1/proposals/:id/vote` | Yes | Vote on proposal |
| `GET` | `/api/v1/challenges` | No | List challenges |
| `POST` | `/api/v1/challenges` | Yes | Create challenge |
| `GET` | `/api/v1/disputes` | No | List disputes |
| `POST` | `/api/v1/disputes` | Yes | Initiate dispute |

### Upload & System
| Method | Path | Auth | Description |
|--------|------|------|-------------|
| `POST` | `/api/v1/upload` | Yes | Upload file to IPFS |
| `GET` | `/api/v1/indexer/status` | No | Blockchain indexer status |

## Project Structure

```
src/
├── main.rs              # Entry point, server setup
├── config.rs            # Environment configuration
├── app_state.rs         # Shared application state
├── error.rs             # Unified error handling
├── middleware/
│   └── auth.rs          # JWT authentication extractor
├── models/              # Database models (sqlx + serde)
├── services/            # Business logic layer
├── routes/              # HTTP handlers
├── blockchain/          # Soroban RPC client & event parser
├── db/
│   └── redis.rs         # Redis caching helpers
└── utils/
    ├── crypto.rs        # Ed25519 signature verification
    └── pagination.rs    # Pagination query helpers
migrations/              # SQL migration files
```

## Related Repositories

| Repository | Description |
|------------|-------------|
| [AgroVest-Contract](https://github.com/AgroVestOfficial/AgroVest-Contract) | Stellar Soroban smart contracts |
| [AgroVest-Frontend](https://github.com/AgroVestOfficial/AgroVest-Frontend) | Next.js frontend application |

## Contributing

We welcome contributions! Please see [CONTRIBUTING.md](CONTRIBUTING.md) for guidelines.

## Security

For security concerns, please see [SECURITY.md](SECURITY.md).

## License

This project is licensed under the MIT License — see [LICENSE](LICENSE) for details.

## Acknowledgments

- [Stellar Development Foundation](https://stellar.org/) for the Soroban platform
- [Axum](https://github.com/tokio-rs/axum) team for the excellent web framework
- The African agricultural community this platform aims to serve
