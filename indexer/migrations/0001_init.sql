 -- Core SPL indexer schema

 CREATE TABLE IF NOT EXISTS mints (
     mint_pubkey TEXT PRIMARY KEY,
     symbol TEXT,
     decimals INT NOT NULL,
     first_seen_slot BIGINT NOT NULL,
     created_at TIMESTAMPTZ NOT NULL DEFAULT now()
 );

 CREATE TABLE IF NOT EXISTS token_transfers (
     id BIGSERIAL PRIMARY KEY,
     signature TEXT NOT NULL,
     slot BIGINT NOT NULL,
     block_time TIMESTAMPTZ,
     mint_pubkey TEXT NOT NULL REFERENCES mints(mint_pubkey),
     source_owner TEXT NOT NULL,
     dest_owner TEXT NOT NULL,
     source_ata TEXT NOT NULL,
     dest_ata TEXT NOT NULL,
     amount BIGINT NOT NULL,
     tx_index INT NOT NULL,
     ix_index INT NOT NULL,
     created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
     UNIQUE (signature, ix_index)
 );

 CREATE INDEX IF NOT EXISTS idx_token_transfers_mint_slot
     ON token_transfers (mint_pubkey, slot);

 CREATE INDEX IF NOT EXISTS idx_token_transfers_signature
     ON token_transfers (signature);

 CREATE TABLE IF NOT EXISTS balances (
     wallet TEXT NOT NULL,
     mint_pubkey TEXT NOT NULL REFERENCES mints(mint_pubkey),
     amount BIGINT NOT NULL,
     PRIMARY KEY (wallet, mint_pubkey)
 );

 CREATE TABLE IF NOT EXISTS last_processed_slot (
     id INT PRIMARY KEY,
     slot BIGINT NOT NULL
 );

