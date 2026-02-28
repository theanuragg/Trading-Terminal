-- Meteora DLMM (Dynamic Liquidity Market Maker) trading table
-- Extends the indexer with support for Meteora DLMM pool swaps and liquidity management

-- Create DLMM-specific trading table
CREATE TABLE IF NOT EXISTS meteora_dlmm_trades (
    id BIGSERIAL PRIMARY KEY,
    tx_hash TEXT NOT NULL,
    ix_index INT NOT NULL,
    block_height BIGINT NOT NULL,
    timestamp BIGINT NOT NULL,
    pool_id TEXT NOT NULL,
    token_mint TEXT NOT NULL,
    trader TEXT NOT NULL,
    amount BIGINT NOT NULL,
    direction TEXT NOT NULL CHECK (direction IN ('buy', 'sell')),
    dlmm_version INT NOT NULL CHECK (dlmm_version IN (1, 2)),
    bins_used INT[],
    fee_tier BIGINT,
    active_bin INT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    UNIQUE (tx_hash, ix_index)
);

CREATE INDEX IF NOT EXISTS idx_meteora_dlmm_trades_pool_id
    ON meteora_dlmm_trades (pool_id);

CREATE INDEX IF NOT EXISTS idx_meteora_dlmm_trades_pool_timestamp
    ON meteora_dlmm_trades (pool_id, timestamp DESC);

CREATE INDEX IF NOT EXISTS idx_meteora_dlmm_trades_token_mint
    ON meteora_dlmm_trades (token_mint, timestamp DESC);

CREATE INDEX IF NOT EXISTS idx_meteora_dlmm_trades_trader
    ON meteora_dlmm_trades (trader, timestamp DESC);
