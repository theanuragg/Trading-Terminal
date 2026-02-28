-- Realtime + trading primitives for the terminal:
-- - events: normalized notifications for websockets
-- - bonding_curve_trades: (buy/sell) events used for price/volume
-- - candles: precomputed OHLCV per mint + timeframe

CREATE TABLE IF NOT EXISTS indexer_events (
    id BIGSERIAL PRIMARY KEY,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    topic TEXT NOT NULL,
    mint_pubkey TEXT,
    payload JSONB NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_indexer_events_topic_created_at
    ON indexer_events (topic, created_at DESC);

CREATE INDEX IF NOT EXISTS idx_indexer_events_mint_created_at
    ON indexer_events (mint_pubkey, created_at DESC);

-- Generic "bonding curve" trade event (pump.fun style, etc.)
CREATE TABLE IF NOT EXISTS bonding_curve_trades (
    id BIGSERIAL PRIMARY KEY,
    signature TEXT NOT NULL,
    slot BIGINT NOT NULL,
    block_time TIMESTAMPTZ,
    mint_pubkey TEXT NOT NULL REFERENCES mints(mint_pubkey),
    trader TEXT NOT NULL,
    side TEXT NOT NULL, -- 'buy' or 'sell'
    token_amount BIGINT NOT NULL,
    sol_amount BIGINT NOT NULL,
    price_nanos_per_token BIGINT NOT NULL,
    tx_index INT NOT NULL,
    ix_index INT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE (signature, ix_index)
);

CREATE INDEX IF NOT EXISTS idx_bonding_curve_trades_mint_slot
    ON bonding_curve_trades (mint_pubkey, slot);

-- Candle table: timeframe in seconds (e.g., 60, 300, 900, 3600)
CREATE TABLE IF NOT EXISTS candles (
    mint_pubkey TEXT NOT NULL REFERENCES mints(mint_pubkey),
    timeframe_secs INT NOT NULL,
    bucket_start TIMESTAMPTZ NOT NULL,
    open BIGINT NOT NULL,
    high BIGINT NOT NULL,
    low BIGINT NOT NULL,
    close BIGINT NOT NULL,
    volume_token BIGINT NOT NULL,
    volume_sol BIGINT NOT NULL,
    trades_count INT NOT NULL,
    PRIMARY KEY (mint_pubkey, timeframe_secs, bucket_start)
);

CREATE INDEX IF NOT EXISTS idx_candles_mint_tf_bucket
    ON candles (mint_pubkey, timeframe_secs, bucket_start DESC);

