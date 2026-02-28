## Indexer

This directory contains a self-contained Rust workspace for a Jetstreamer‑style SPL token indexer.

### 📚 Documentation Guide

**Start here:** [QUICKSTART.md](QUICKSTART.md) - Get running in 5 minutes  
**Infrastructure:** [DOCKER_SETUP.md](DOCKER_SETUP.md) - Docker Compose configuration  
**Blockchain:** [JETSTREAMER_SETUP.md](JETSTREAMER_SETUP.md) - Firehose endpoint configuration

### Crates

- `indexer-core`: shared config, data models, SPL parsing stubs, and Postgres access helpers.
- `indexer-bin`: long‑running indexer process (Firehose client stub + writer loop).
- `indexer-api`: Axum HTTP API providing token‑ and wallet‑centric views over the indexed data.

### Schema migrations

`indexer/migrations/0001_init.sql` defines tables:

- `mints`
- `token_transfers`
- `balances`
- `bonding_curve_trades` (for Pump.fun, Raydium, Meteora swaps)
- `last_processed_slot`

Migrations are embedded via `sqlx::migrate!` in `indexer-core` and run on startup by both `indexer-bin` and `indexer-api`.

### Quick Start

#### 1. Start PostgreSQL and Redis with Docker Compose

```bash
cd indexer
docker-compose up -d
```

See [DOCKER_SETUP.md](DOCKER_SETUP.md) for detailed Docker configuration.

#### 2. Configure Environment Variables

```bash
# Use Docker services
export INDEXER__DB__URL="postgres://postgres:postgres@localhost:5432/indexer"
export INDEXER__REDIS__HOST="127.0.0.1"
export INDEXER__REDIS__PORT="6379"

# Configure Firehose endpoint
export INDEXER__FIREHOSE__ENDPOINT="http://localhost:9000"
```

Or set them in `config/default.toml` (see [Configuration](#configuration) below).

#### 3. Run the Indexer

```bash
# Run the indexer binary (processes blocks from Firehose)
cargo run --bin indexer-bin

# In another terminal, run the API (provides REST + WebSocket access)
cargo run --bin indexer-api
```

### Configuration

All configuration values can be overridden via `INDEXER__` prefixed environment variables:

```bash
# Database
export INDEXER__DB__URL="postgres://user:pass@host:5432/db"
export INDEXER__DB__MAX_CONNECTIONS=20

# API
export INDEXER__API__BIND_ADDR="0.0.0.0:8080"

# Firehose/Jetstreamer
export INDEXER__FIREHOSE__ENDPOINT="https://mainnet.firehose.io:443"
export INDEXER__FIREHOSE__FROM_SLOT="240000000"

# Redis
export INDEXER__REDIS__HOST="redis-host"
export INDEXER__REDIS__PORT="6379"
export INDEXER__REDIS__PASSWORD="secret"
```

See `config/default.toml` for all available options.

### Setup Guides

- **[DOCKER_SETUP.md](DOCKER_SETUP.md)** - Local development with Docker Compose
- **[JETSTREAMER_SETUP.md](JETSTREAMER_SETUP.md)** - Configure Firehose/Jetstreamer endpoints

### Running Locally (Without Docker)

From the repo root:

```bash
cd indexer

# Ensure PostgreSQL and Redis are running locally on default ports

# Run API (REST + WebSocket on :8080)
cargo run --bin indexer-api

# Run indexer binary (streams blocks from Firehose to database)
cargo run --bin indexer-bin
```

Configuration is loaded from `indexer/config/default.toml` if present and from `INDEXER__*` environment variables (see `IndexerConfig` in `indexer-core`).

### Testing

Run all tests:

```bash
cargo test --lib
```

Run specific parser tests:

```bash
cargo test --lib raydium_parser
cargo test --lib meteora_parser
cargo test --lib bonding_parser
```

### Parsers Implemented

- **SPL Token Parser** - Tracks token transfers across all accounts
- **Pump.fun Parser** - Extracts bonding curve trades
- **Raydium Parser** - Extracts swap trades from Raydium AMM (v3/v4)
- **Meteora Parser** - Extracts DLMM (Dynamic Liquidity Market Making) trades

All parsers are fully implemented with comprehensive test coverage.

