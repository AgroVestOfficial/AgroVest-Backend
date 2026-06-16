# AgroVest — Technical Architecture

> Full-stack blockchain platform for agricultural investment and commerce on Stellar Soroban.

---

## 1. System Overview

AgroVest is a three-tier decentralized application:

```
┌──────────────────────────────────────────────────────────────┐
│                         CLIENT LAYER                          │
│                                                              │
│   Next.js 14 (App Router) + TypeScript + Tailwind + shadcn   │
│   Wagmi + Viem + Reown AppKit (WalletConnect)                │
│   Pinata IPFS (image uploads)                                │
│                                                              │
├──────────────────────────────────────────────────────────────┤
│                        BACKEND LAYER                          │
│                                                              │
│   Rust + Axum 0.7 + Tokio                                    │
│   PostgreSQL 16 (sqlx) + Redis 7                             │
│   Soroban Event Indexer (background polling)                 │
│   JWT auth (Ed25519 challenge-response)                      │
│                                                              │
├──────────────────────────────────────────────────────────────┤
│                      BLOCKCHAIN LAYER                         │
│                                                              │
│   Stellar Soroban (WASM smart contracts in Rust)             │
│   4 contracts: Farm, Investment, Escrow, DAO                 │
│   AVT Token (Stellar Asset Contract)                         │
│                                                              │
└──────────────────────────────────────────────────────────────┘
```

**Data flow:**
- **Writes:** Frontend → Wallet (signs tx) → Soroban contracts (on-chain state)
- **Reads:** Frontend → Backend REST API → PostgreSQL (indexed from chain events)
- **Files:** Frontend → Pinata IPFS → CID stored on-chain / in DB

---

## 2. Blockchain Layer — Stellar Soroban Contracts

### 2.1 Platform & SDK

| Component | Technology |
|-----------|-----------|
| Blockchain | Stellar |
| Smart contract runtime | Soroban v22 |
| Language | Rust (edition 2021) |
| Compile target | `wasm32-unknown-unknown` |
| SDK | `soroban-sdk 22.0.0` |
| Token standard | Stellar Asset Contract (SAC) |

### 2.2 Contract Architecture

Four independent contracts with no shared inheritance. They reference each other by storing contract addresses in instance storage at initialization time.

```
┌─────────┐         ┌─────────────┐
│   DAO   │────────▶│ Investment  │  (cross-contract: execute_proposal → create_investment)
│ (17 fn) │         │   (8 fn)    │
└─────────┘         └─────────────┘

┌─────────┐         ┌─────────────┐
│   Farm  │────────▶│   Escrow    │  (stores escrow address, referenced in purchases)
│ (19 fn) │         │   (6 fn)    │
└─────────┘         └─────────────┘
```

### 2.3 Contract Details

#### Farm Contract (`agrovest-farm`)

**Purpose:** Farm registration, product marketplace, cart, purchases, reviews.

**State:**
- Instance storage: `admin`, `token`, `escrow` (Addresses), `farm_ctr`, `prod_ctr` (u32), `total_sales` (i128)
- Persistent storage: farms (by address, id, name), products (by id, per-farmer), purchases, reviews
- Temporary storage: shopping carts (~24h TTL)

**Key types:**
```rust
struct Farmer { farm_id, business_name, business_image, business_location, business_contact, business_email, farmer_address, is_registered }
struct FarmProduct { product_name, product_image, product_description, product_price: i128, product_owner, product_id: u32, sold: bool }
struct Review { reviewer: Address, review: String }
```

**Public functions (19):**
`initialize`, `register_farm`, `update_details`, `add_farm_product`, `update_farm_product`, `get_farm_products`, `get_all_farm_products`, `add_to_cart`, `remove_from_cart`, `get_cart`, `purchase_product`, `submit_review`, `get_product_reviews`, `get_name`, `get_address`, `get_user`, `get_image`, `get_all_farms`, `get_total_sales`

**Error codes (16):** `NameCannotBeEmpty`, `NameAlreadyRegistered`, `InvalidFarmIndex`, `NotRegistered`, `FarmDoesNotBelongToYou`, `FarmNotFound`, `InvalidProductIndex`, `ProductDoesNotExist`, `OnlyBuyersCanReview`, `AlreadyReviewed`, `AlreadyPurchased`, `ProductAlreadySold`, `PriceMismatch`, `ProductNotInCart`, `AlreadyInitialized`, `NotInitialized`

---

#### Investment Contract (`agrovest-investment`)

**Purpose:** Time-locked farm investment campaigns.

**State:**
- Instance storage: `token` (Address), `total` (i128), `inv_ctr` (u32)
- Persistent storage: investments (by id), investors (by id, per-farm)

**Key types:**
```rust
struct FarmInvestmentDetails { id, farm_id, image, name, about, owner, min_amount, amount_raised, start_date, end_date: u64, farm_investor_count }
struct Investor { id, farm_id, investor_address, amount }
```

**Public functions (8):**
`initialize`, `create_investment`, `invest`, `claim_investment`, `get_all_investable_farms`, `get_total_investment`, `get_farm_investors`, `get_all_investors`

**Error codes (8):** `AlreadyInitialized`, `NotInitialized`, `InvestmentNotFound`, `InvestmentNotActive`, `EndDateNotReached`, `AmountBelowMinimum`, `NotFarmOwner`, `NothingToClaim`

---

#### Escrow Contract (`agrovest-escrow`)

**Purpose:** Buyer-farmer transaction protection with dispute resolution.

**State:**
- Instance storage: `admin` (Address), `token` (Address), `escrow_ctr` (u32)
- Persistent storage: escrows (by id)

**Key types:**
```rust
enum EscrowStatus { AwaitingDelivery(0), AwaitingApproval(1), Complete(2), Dispute(3) }
struct Escrow { buyer, farmer, amount: i128, status, order_id }
```

**Public functions (6):**
`initialize`, `create_escrow`, `approve_delivery`, `raise_dispute`, `resolve_dispute`, `get_escrow_details`

**Error codes (10):** `AlreadyInitialized`, `NotInitialized`, `EscrowNotFound`, `InvalidStatus`, `OnlyBuyerCanApprove`, `OnlyParticipantsCanRaise`, `OnlyAdminCanResolve`, `TransferFailed`, `AlreadyResolved`, `InvalidWinner`

**Note:** Token transfers are currently stubbed (`// In production: token.transfer(...)`). State transitions are functional.

---

#### DAO Contract (`agrovest-dao`)

**Purpose:** Governance — proposals, weighted voting, delegation, challenges, disputes.

**State:**
- Instance storage: `admin`, `token`, `investment` (Addresses), `prop_ctr`, `chall_ctr`, `disp_ctr` (u32)
- Persistent storage: proposals, votes, locked tokens, delegations, challenges, disputes

**Key types:**
```rust
enum VoteData { Null(0), Accept(1), Reject(2), Undecided(3) }
struct ProposalData { is_challenged, proposal_id, title, description, created_at, ends_at, required_votes, proposer, executed, accept_votes, reject_votes, undecided_votes }
struct Votes { proposal_id, voter, voting_power, vote_type }
struct ChallengeData { proposal_id, description, resolved, challenger }
struct DisputeData { challenge_id, arbitrator, resolved, ruling }
```

**Public functions (17):**
`initialize`, `lock_tokens`, `unlock_tokens`, `get_token_balance`, `calculate_voting_power`, `create_proposal`, `get_proposal`, `vote_proposal`, `tally_votes`, `execute_proposal`, `delegate`, `undelegate`, `get_delegate`, `create_challenge`, `resolve_challenge`, `get_challenge`, `initiate_dispute`, `resolve_dispute`, `get_dispute`

**Anti-whale mechanism:** Voting power = √(locked tokens), calculated via integer square root.

**Cross-contract call:** `execute_proposal` invokes `Investment.create_investment` via `env.invoke_contract`.

**Error codes (18):** Covers initialization, delegation, voting, proposal lifecycle, challenge/dispute states.

---

### 2.4 Storage Strategy

| Soroban Backend | Usage | TTL |
|----------------|-------|-----|
| Instance | Contract config, counters, admin/token addresses | Permanent (contract lifetime) |
| Persistent | Core domain data (farms, products, investments, escrows, proposals, votes) | Permanent |
| Temporary | Shopping carts | ~24 hours |

### 2.5 Authentication Model

Every mutating function accepts `caller: Address` and calls `caller.require_auth()` — Soroban's equivalent of verifying `msg.sender`. The frontend signs transactions via the connected wallet.

### 2.6 Build & Deploy

```bash
# Build all contracts
soroban contract build   # or: make build

# Run tests (22 total across 4 contracts)
cargo test --workspace   # or: make test

# Deploy (via script)
./scripts/deploy.sh --network testnet --source <secret_key>
# Deploys: Escrow → Investment → Farm → DAO
# Returns contract IDs for .env configuration
```

**Release profile** (optimized for minimal WASM):
```toml
[profile.release]
opt-level = "z"
overflow-checks = true
strip = "symbols"
panic = "abort"
lto = true
codegen-units = 1
```

---

## 3. Backend Layer — Rust/Axum API

### 3.1 Tech Stack

| Component | Technology | Version |
|-----------|-----------|---------|
| Language | Rust | 1.70+ (edition 2021) |
| Web framework | Axum | 0.7 |
| Async runtime | Tokio | 1.x (full) |
| Database | PostgreSQL | 16 |
| DB driver | sqlx | 0.8 (compile-time checked) |
| Cache | Redis | 7 |
| Auth crypto | ed25519-dalek + jsonwebtoken | 2.x / 9.x |
| HTTP client | reqwest | 0.12 |
| Serialization | serde + serde_json | 1.x |
| Logging | tracing + tracing-subscriber | Latest |
| Middleware | tower-http | 0.5 |
| Containerization | Docker + Docker Compose | — |

### 3.2 Project Structure

```
AgroVest-Backend/
├── src/
│   ├── main.rs                  # Entry point, server startup
│   ├── config.rs                # Environment configuration
│   ├── app_state.rs             # Shared state (DB, Redis, config)
│   ├── error.rs                 # Unified ApiError enum
│   ├── routes/
│   │   ├── mod.rs               # Router composition (11 sub-routers)
│   │   ├── auth.rs              # POST /nonce, POST /verify, GET /me
│   │   ├── users.rs             # GET/PUT /users/{address}
│   │   ├── farms.rs             # CRUD /farms
│   │   ├── products.rs          # CRUD /products
│   │   ├── reviews.rs           # GET/POST /products/{id}/reviews
│   │   ├── cart.rs              # GET/POST/DELETE /cart
│   │   ├── investments.rs       # CRUD /investments
│   │   ├── escrows.rs           # CRUD /escrows
│   │   ├── dao.rs               # Proposals, votes, challenges, disputes
│   │   ├── upload.rs            # POST /upload (IPFS via Pinata)
│   │   └── indexer.rs           # GET /indexer/status
│   ├── middleware/
│   │   └── auth.rs              # JWT extraction (AuthUser extractor)
│   ├── models/                  # SQLx FromRow structs + request DTOs
│   ├── services/                # Business logic layer
│   │   ├── auth_service.rs      # Nonce gen, signature verify, JWT issue
│   │   ├── ipfs_service.rs      # Pinata upload
│   │   ├── indexer_service.rs   # Soroban event polling
│   │   └── escrow_service.rs    # Escrow business logic
│   ├── blockchain/
│   │   ├── soroban_client.rs    # JSON-RPC getEvents client
│   │   └── event_parser.rs      # Soroban event parsing
│   ├── db/
│   │   ├── mod.rs               # Connection pool setup
│   │   └── redis.rs             # Redis connection + generic cache helpers
│   └── utils/
│       ├── crypto.rs            # Stellar Ed25519 signature verification
│       └── pagination.rs        # Reusable pagination params/response
├── migrations/                  # 13 SQL migration files
├── Dockerfile                   # Multi-stage build
├── docker-compose.yml           # postgres + redis + app
└── Cargo.toml
```

### 3.3 API Endpoints

All routes under `/api/v1`:

| Method | Endpoint | Auth | Description |
|--------|----------|------|-------------|
| POST | `/auth/nonce` | No | Generate signing nonce (stored in Redis, 300s TTL) |
| POST | `/auth/verify` | No | Verify Ed25519 signature, issue JWT |
| GET | `/auth/me` | Yes | Get current user address |
| GET | `/users/{address}` | No | Get user profile |
| PUT | `/users/{address}` | Yes | Update own profile (owner-only) |
| GET | `/farms` | No | List farms (paginated) |
| GET | `/farms/{id}` | No | Get farm by ID |
| GET | `/farms/by-address/{address}` | No | Get farm by owner address |
| POST | `/farms` | Yes | Register farm |
| PUT | `/farms/{id}` | Yes | Update farm (owner-only) |
| GET | `/products` | No | List products (filterable, paginated) |
| GET | `/products/{id}` | No | Get product |
| GET | `/farms/{farm_id}/products` | No | Get farm's products |
| POST | `/products` | Yes | Create product |
| PUT | `/products/{id}` | Yes | Update product (owner-only) |
| GET | `/products/{id}/reviews` | No | Get product reviews |
| POST | `/products/{id}/reviews` | Yes | Submit review (upsert) |
| GET | `/cart` | Yes | Get user's cart |
| POST | `/cart` | Yes | Add to cart |
| DELETE | `/cart/{product_id}` | Yes | Remove from cart |
| DELETE | `/cart` | Yes | Clear cart |
| GET | `/investments` | No | List investments (filterable) |
| GET | `/investments/{id}` | No | Get investment |
| GET | `/investments/{id}/investors` | No | Get investment's investors |
| POST | `/investments` | Yes | Create investment |
| POST | `/investments/{id}/invest` | Yes | Invest (DB tx with row lock) |
| GET | `/escrows` | Yes | List user's escrows (filterable) |
| GET | `/escrows/{id}` | Yes | Get escrow |
| POST | `/escrows` | Yes | Create escrow |
| PUT | `/escrows/{id}/approve` | Yes | Approve delivery (buyer-only) |
| PUT | `/escrows/{id}/dispute` | Yes | Raise dispute |
| GET | `/proposals` | No | List proposals (paginated) |
| GET | `/proposals/{id}` | No | Get proposal |
| POST | `/proposals` | Yes | Create proposal |
| POST | `/proposals/{id}/vote` | Yes | Vote on proposal |
| GET | `/challenges` | No | List challenges |
| POST | `/challenges` | Yes | Create challenge |
| GET | `/disputes` | No | List disputes |
| POST | `/disputes` | No | Initiate dispute |
| POST | `/upload` | Yes | Upload file to IPFS |
| GET | `/indexer/status` | No | Get indexer sync status |

### 3.4 Authentication Flow

```
Client                          Backend                         Redis
  │                               │                               │
  │── POST /auth/nonce ──────────▶│                               │
  │   { address }                 │── generate nonce ────────────▶│
  │                               │◀─ store nonce (300s TTL) ────│
  │◀─ { nonce } ─────────────────│                               │
  │                               │                               │
  │  [Client signs nonce with     │                               │
  │   Stellar Ed25519 secret key] │                               │
  │                               │                               │
  │── POST /auth/verify ─────────▶│                               │
  │   { address, signature }      │── get nonce ─────────────────▶│
  │                               │◀─ nonce ─────────────────────│
  │                               │── verify Ed25519 signature    │
  │                               │── delete nonce ──────────────▶│
  │                               │── upsert user in PostgreSQL   │
  │                               │── issue JWT (HS256)           │
  │◀─ { token } ─────────────────│                               │
  │                               │                               │
  │── GET /api/v1/auth/me ───────▶│                               │
  │   Authorization: Bearer <jwt> │── decode & verify JWT         │
  │◀─ { address } ───────────────│                               │
```

### 3.5 Database Schema (13 Tables)

```sql
-- Core user table (PK = Stellar address)
users(address VARCHAR(56) PK, display_name, bio, avatar_url, created_at, updated_at)

-- Farm registry
farms(id SERIAL PK, farm_id_onchain INT UNIQUE, name, image, location, contact, email,
      farmer_address FK→users, is_registered BOOLEAN)

-- Marketplace products
products(id SERIAL PK, product_id_onchain INT UNIQUE, product_name, product_image,
         product_description, product_price BIGINT, product_owner FK→users,
         farm_id FK→farms, sold BOOLEAN, category)

-- Product reviews (unique on reviewer+product)
reviews(id SERIAL PK, reviewer FK→users, review_text, product_id FK→products)

-- Investment campaigns
investments(id SERIAL PK, investment_id_onchain INT UNIQUE, farm_id FK→farms,
            image, name, about, owner FK→users, min_amount, amount_raised BIGINT,
            start_date BIGINT, end_date BIGINT, farm_investor_count, is_active BOOLEAN)

-- Individual investors
investors(id SERIAL PK, investor_id_onchain INT UNIQUE, farm_id,
          investment_id FK→investments, investor_address FK→users, amount BIGINT)

-- Escrow transactions
escrows(id SERIAL PK, escrow_id_onchain INT UNIQUE, buyer FK→users,
        farmer FK→users, amount, status ENUM(escrow_status), order_id)

-- DAO proposals
proposals(id SERIAL PK, proposal_id_onchain INT UNIQUE, title, description,
          created_at_onchain BIGINT, ends_at BIGINT, required_votes BIGINT,
          proposer FK→users, executed BOOLEAN, is_challenged BOOLEAN,
          accept_votes, reject_votes, undecided_votes)

-- DAO votes (unique on proposal+voter)
votes(id SERIAL PK, proposal_id FK→proposals, voter FK→users,
      voting_power, vote_type ENUM(vote_type))

-- Proposal challenges
challenges(id SERIAL PK, challenge_id_onchain INT UNIQUE, proposal_id FK→proposals,
           description, resolved BOOLEAN, challenger FK→users)

-- Dispute resolution
disputes(id SERIAL PK, dispute_id_onchain INT UNIQUE, challenge_id FK→challenges,
         arbitrator FK→users, resolved BOOLEAN, ruling BOOLEAN)

-- Shopping cart (unique on user+product)
cart_items(id SERIAL PK, user_address FK→users, product_id FK→products, added_at)

-- Blockchain indexer state
indexer_state(contract_address VARCHAR PK, last_cursor, last_synced_at, synced_height BIGINT)

-- IPFS file metadata
ipfs_metadata(cid VARCHAR PK, original_name, mime_type, size_bytes,
              uploader FK→users, pinned BOOLEAN, created_at)
```

**Custom ENUMs:**
```sql
CREATE TYPE escrow_status AS ENUM ('awaiting_delivery', 'awaiting_approval', 'complete', 'dispute');
CREATE TYPE vote_type AS ENUM ('null', 'accept', 'reject', 'undecided');
```

### 3.6 Blockchain Indexer

Background service that syncs on-chain state to PostgreSQL:

```
┌─────────────────┐     JSON-RPC      ┌─────────────────┐
│  IndexerService  │──── getEvents ───▶│  Soroban RPC    │
│  (1 task/contract│◀─── events ──────│  Node           │
│   Tokio spawn)   │                   └─────────────────┘
│                  │
│  1. Read last synced ledger from indexer_state table
│  2. Poll getEvents(startLedger, contractIds)
│  3. Parse events → update domain tables
│  4. Update indexer_state with new height
│  5. Sleep 5s, repeat
└─────────────────┘
```

**Poll interval:** 5 seconds (configurable)
**Contracts tracked:** Farm, Investment, Escrow, DAO

### 3.7 Error Handling

Unified error type with HTTP status mapping:

```rust
enum ApiError {
    NotFound(String)      → 404
    Unauthorized          → 401
    Forbidden             → 403
    BadRequest(String)    → 400
    Conflict(String)      → 409
    Internal(String)      → 500
    Database(sqlx::Error) → 500
    Blockchain(String)    → 502
}
// All serialize to: { "error": { "code": <status>, "message": "<msg>" } }
```

### 3.8 Infrastructure

**Docker Compose services:**
- `postgres` — PostgreSQL 16 Alpine
- `redis` — Redis 7 Alpine
- `app` — Rust/Axum application (multi-stage Docker build)

**Environment configuration** via `.env`:
- `DATABASE_URL`, `REDIS_URL`
- `JWT_SECRET`, `JWT_EXPIRY_HOURS`
- `SOROBAN_RPC_URL`, `SOROBAN_NETWORK_PASSPHRASE`
- `FARM_CONTRACT_ADDRESS`, `INVESTMENT_CONTRACT_ADDRESS`, `ESCROW_CONTRACT_ADDRESS`, `DAO_CONTRACT_ADDRESS`
- `PINATA_API_KEY`, `PINATA_API_SECRET`
- `SERVER_HOST`, `SERVER_PORT`

---

## 4. Frontend Layer — Next.js Application

### 4.1 Tech Stack

| Component | Technology | Version |
|-----------|-----------|---------|
| Framework | Next.js (App Router) | 14.2.12 |
| Language | TypeScript | 5.x |
| UI library | React | 18.x |
| CSS | Tailwind CSS | 3.4.1 |
| Component libs | NextUI, shadcn/ui | 2.4.8 |
| Animations | Framer Motion | 11.5.6 |
| Charts | ApexCharts / react-apexcharts | Latest |
| Web3 hooks | Wagmi | 2.14.11 |
| Web3 utils | Viem | 2.23.2 |
| Wallet connect | Reown AppKit | 1.6.8 |
| IPFS | Pinata (via Axios) | — |
| State | TanStack React Query | 5.66.3 |
| Notifications | Sonner | Latest |
| Font | Montserrat (Google Fonts) | — |

### 4.2 Project Structure

```
AgroVest-Frontend/
├── app/
│   ├── layout.tsx               # Root layout (providers, fonts)
│   ├── (guest)/                 # Public routes
│   │   ├── layout.tsx           # GuestHeader + GuestFooter
│   │   ├── page.tsx             # Landing page
│   │   ├── marketplace/page.tsx # Public marketplace showcase
│   │   └── investment/page.tsx  # Public investment showcase
│   └── user/                    # Authenticated routes
│       ├── layout.tsx           # DashboardSidebar + DashboardHeader
│       ├── page.tsx             # Dashboard (stats, charts)
│       ├── marketplace/
│       │   ├── page.tsx         # All products
│       │   ├── mine/page.tsx    # My products
│       │   └── [id]/page.tsx    # Product detail
│       ├── portfolio/
│       │   ├── page.tsx         # My farms
│       │   ├── [id]/page.tsx    # Farm detail
│       │   ├── investments/     # Investment management
│       │   └── products/        # Product management
│       ├── explore/
│       │   ├── page.tsx         # Browse investments
│       │   └── [id]/page.tsx    # Investment detail
│       ├── governance/page.tsx  # DAO proposals
│       └── transactions/
│           ├── page.tsx         # Shopping cart
│           └── purchased/       # Purchase history
├── components/
│   ├── ui/                      # shadcn/ui components
│   ├── shared/                  # WalletComponent, etc.
│   └── [feature]/               # Feature-specific components
├── hooks/
│   ├── ReadHooks/               # 11 contract read hooks
│   └── WriteHooks/              # 10 contract write hooks
├── abis/                        # Contract ABIs (JSON/TS)
├── config/
│   └── config.ts                # Chain config, Wagmi adapter
├── constants/
│   └── contracts.ts             # Ethers.js contract factories
├── context/
│   └── index.tsx                # Reown + Wagmi + React Query providers
└── utils/
    ├── types.ts                 # TypeScript type definitions
    └── uploadToIPFS.tsx         # Pinata IPFS upload helper
```

### 4.3 Contract Interaction Hooks

**Read hooks** (11) — all use `useReadContract` from Wagmi:

| Hook | Contract Function | Returns |
|------|------------------|---------|
| `useGetAllFarms` | `retrunFarms` | All registered farms |
| `useGetAllFarmProducts` | `getAllFarmProducts` | All products |
| `useGetFarmProductByAddress` | `getFarmProducts` | Products for address |
| `useGetAllAvailableInvestment` | `getAllInvestableFarms` | All investments |
| `useGetFarmInvestors` | `getAllFarmInvestors` | Farm investors |
| `useGetAllInvestors` | `getAllInvestors` | All investors |
| `useGetTotalInvestment` | `getTotalInvestment` | Total invested |
| `useGetTotalSales` | `getTotalSale` | Total sales |
| `useGetCartProducts` | `getCartProducts` | Cart items |
| `useGetAllPurchasedProduct` | `getPurchasedProducts` | Purchased items |
| `useGetProductReview` | `getProductReviews` | Product reviews |

**Write hooks** (10) — all use `useWriteContract` from Wagmi:

| Hook | Contract Function | Action |
|------|------------------|--------|
| `useRegisterFarm` | `registerFarms` | Register farm |
| `useUpdateFarmDetails` | `updateDetails` | Update farm |
| `useAddFarmProduct` | `addFarmProduct` | Add product |
| `useUpdateFarmProductDetails` | `addFarmProduct` | Update product |
| `useAddProductToCart` | `addProductToCart` | Add to cart |
| `useRemoveProductFromCart` | `removeProductFromCart` | Remove from cart |
| `usePurchaseProduct` | `purchaseProduct` | Purchase (single/batch) |
| `useSubmitReview` | `submitReview` | Submit review |
| `useCreateInvestment` | `createInvestment` | Create investment |
| `useInvestEthers` | `investEthers` | Invest in farm |

### 4.4 Wallet & Network Configuration

```typescript
// config/config.ts
const crossFiTestnet = {
  id: 4157,
  name: "CrossFi Testnet",
  nativeCurrency: { name: "XFI", symbol: "XFI", decimals: 18 },
  rpcUrls: { default: { http: ["https://rpc.testnet.ms"] } },
};

// Supported networks: CrossFi Testnet (default), Sepolia
// Wallet: Reown AppKit (email, social, or crypto wallet)
```

### 4.5 IPFS Image Flow

```
User selects image → Frontend uploads to Pinata API → Returns CID
→ CID stored on-chain (in contract) and/or in PostgreSQL
→ Images served from https://gateway.pinata.cloud/ipfs/{CID}
```

---

## 5. Cross-Layer Data Flow

### 5.1 Farm Registration

```
Frontend                    Wallet              Soroban Farm Contract
   │                          │                         │
   │── upload image ────────────────────────▶ Pinata IPFS (returns CID)
   │── registerFarm(name, CID, location...) ▶│         │
   │                          │── sign tx ──▶│         │
   │                          │              │── require_auth()
   │                          │              │── store Farmer in persistent storage
   │                          │              │── emit event
   │◀─ tx hash ──────────────│◀─ receipt ───│         │
   │                                                               │
   │                          Backend Indexer                       │
   │                          │── poll getEvents ──▶ Soroban RPC
   │                          │◀─ FarmRegistered event
   │                          │── INSERT INTO farms
```

### 5.2 Product Purchase

```
Frontend                    Wallet              Soroban Farm Contract    Backend
   │                          │                         │                │
   │── addProductToCart ─────▶│── sign tx ──▶│          │                │
   │                          │              │── store in temporary     │
   │                          │              │   storage (cart)         │
   │                                                               │
   │── purchaseProduct(amt) ─▶│── sign tx ──▶│          │                │
   │                          │              │── require_auth()         │
   │                          │              │── mark sold=true         │
   │                          │              │── record purchase        │
   │                          │              │── total_sales += amt     │
   │                                                               │
   │                          Indexer polls, syncs to PostgreSQL ──────▶│
```

---

## 6. Testing

### Smart Contracts
- **Framework:** Soroban SDK test utilities (`soroban-sdk` with `testutils` feature)
- **Runner:** `cargo test --workspace`
- **Auth:** `env.mock_all_auths()` in all tests
- **Count:** 22 tests (5 + 5 + 5 + 7)
- **Types:** Positive tests + `#[should_panic]` negative tests

### Backend
- **Status:** Test directory exists but is empty
- **Recommended:** Integration tests with `sqlx::test` + `axum::test`

### Frontend
- **CI:** GitHub Actions — lint, type-check, format-check, build
- **Tooling:** ESLint + Prettier + Husky + lint-staged

---

## 7. CI/CD

### Smart Contracts (`.github/workflows/ci.yml`)
```
Push/PR → Install Rust (stable + wasm target) → Install Soroban CLI
→ Build → Test → Format check → Clippy lint
```

### Frontend (`.github/workflows/ci.yml`)
```
Push/PR to main → Install deps → Lint → Type-check → Format check → Build
```

---

## 8. Security Considerations

| Area | Status | Notes |
|------|--------|-------|
| Contract auth | ✅ | `require_auth()` on all mutations |
| JWT auth | ✅ | Ed25519 challenge-response + HS256 JWT |
| Nonce replay | ✅ | Single-use, 300s TTL, deleted after verify |
| Ownership checks | ✅ | Inline in handlers (`auth.address != owner`) |
| CORS | ⚠️ | Currently permissive (`Any`) — should restrict in production |
| SQL injection | ⚠️ | `escrow_service::list_escrows` uses string formatting |
| Token transfers | ⚠️ | Stubbed in contracts — needs SAC integration |
| Rate limiting | ❌ | Not implemented |
| Input validation | Partial | Basic checks exist, needs hardening |
| Contract audit | ❌ | Not yet audited |

---

## 9. Known Limitations & TODOs

1. **Token transfers stubbed** — Escrow and Investment contracts record state but don't perform actual SAC transfers. DAO lock/unlock is state-only.
2. **Indexer event processing** — `process_event` is a placeholder; domain tables aren't synced from events yet.
3. **`get_all_investors`** — Investment contract function returns empty vec (TODO).
4. **No backend tests** — Test directory is empty.
5. **CORS permissive** — Needs restriction before production.
6. **SQL injection risk** — `escrow_service::list_escrows` needs parameterized queries.
7. **No rate limiting** — API is unprotected against abuse.
8. **Frontend/Backend duplication** — Frontend talks directly to contracts AND backend exists; needs clear read/write separation strategy.
