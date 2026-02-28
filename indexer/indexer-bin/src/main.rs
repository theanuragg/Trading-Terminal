use anyhow::Result;
use indexer_core::{
    bonding_parser::extract_pump_trades_from_block,
    config::IndexerConfig,
    db::{
        create_pool, get_last_processed_slot, insert_bonding_curve_trades, insert_event,
        insert_transfers, run_migrations, set_last_processed_slot, update_balances_for_transfers,
        upsert_candle,
    },
    firehose::FirehoseClient,
    models::Candle,
    raydium_parser::extract_raydium_trades_from_block,
    meteora_parser::extract_meteora_trades_from_block,
    spl_parser::{extract_transfers_from_block, BlockRef},
};
use chrono::TimeZone;
use tokio::sync::mpsc;
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .init();

    let config = IndexerConfig::from_env()?;

    tracing::info!("Starting indexer with config: {:?}", config.runtime);

    let pool = create_pool(&config.db.url, config.db.max_connections).await?;
    run_migrations(&pool).await?;

    run_indexer(config, pool).await?;

    Ok(())
}

async fn run_indexer(config: IndexerConfig, pool: sqlx::PgPool) -> Result<()> {
    let (block_tx, mut block_rx) = mpsc::channel::<BlockRef>(1024);

    // Writer task: consumes blocks, parses SPL transfers, and writes to DB.
    let writer_pool = pool.clone();
    let mint_whitelist = config.firehose.mint_whitelist.clone();
    let writer_handle = tokio::spawn(async move {
        while let Some(block) = block_rx.recv().await {
            let transfers = extract_transfers_from_block(&block, &mint_whitelist);
            let pump_trades = extract_pump_trades_from_block(&block);
            let raydium_trades = extract_raydium_trades_from_block(&block);
            let meteora_trades = extract_meteora_trades_from_block(&block);

            if transfers.is_empty() {
                // still allow pump trades / candles even if there are no SPL transfers in this block
            }

            if !transfers.is_empty() {
                if let Err(err) = insert_transfers(&writer_pool, &transfers).await {
                    tracing::error!("failed to insert transfers: {err:?}");
                    continue;
                }

                if let Err(err) = update_balances_for_transfers(&writer_pool, &transfers).await {
                    tracing::error!("failed to update balances: {err:?}");
                    continue;
                }

                // Realtime event fanout for websockets (Postgres LISTEN/NOTIFY).
                for t in &transfers {
                    let payload = serde_json::json!({
                        "signature": t.signature,
                        "slot": t.slot,
                        "mint_pubkey": t.mint_pubkey,
                        "source_owner": t.source_owner,
                        "dest_owner": t.dest_owner,
                        "amount": t.amount,
                        "tx_index": t.tx_index,
                        "ix_index": t.ix_index
                    });
                    if let Err(err) = insert_event(&writer_pool, "transfers", Some(&t.mint_pubkey), payload).await {
                        tracing::error!("failed to insert/notify transfer event: {err:?}");
                    }
                }
            }

            if !pump_trades.is_empty() {
                if let Err(err) = insert_bonding_curve_trades(&writer_pool, &pump_trades).await {
                    tracing::error!("failed to insert pump trades: {err:?}");
                    continue;
                }

                for t in &pump_trades {
                    let payload = serde_json::json!({
                        "signature": t.signature,
                        "slot": t.slot,
                        "mint_pubkey": t.mint_pubkey,
                        "trader": t.trader,
                        "side": t.side,
                        "token_amount": t.token_amount,
                        "sol_amount": t.sol_amount,
                        "price_nanos_per_token": t.price_nanos_per_token,
                        "venue": "pump",
                        "tx_index": t.tx_index,
                        "ix_index": t.ix_index
                    });
                    if let Err(err) = insert_event(&writer_pool, "bonding", Some(&t.mint_pubkey), payload).await {
                        tracing::error!("failed to insert/notify pump trade event: {err:?}");
                    }
                }
            }

            if !raydium_trades.is_empty() {
                if let Err(err) = insert_bonding_curve_trades(&writer_pool, &raydium_trades).await {
                    tracing::error!("failed to insert raydium trades: {err:?}");
                    continue;
                }

                for t in &raydium_trades {
                    let payload = serde_json::json!({
                        "signature": t.signature,
                        "slot": t.slot,
                        "mint_pubkey": t.mint_pubkey,
                        "trader": t.trader,
                        "side": t.side,
                        "token_amount": t.token_amount,
                        "sol_amount": t.sol_amount,
                        "price_nanos_per_token": t.price_nanos_per_token,
                        "venue": "raydium",
                        "tx_index": t.tx_index,
                        "ix_index": t.ix_index
                    });
                    if let Err(err) = insert_event(&writer_pool, "bonding", Some(&t.mint_pubkey), payload).await {
                        tracing::error!("failed to insert/notify raydium trade event: {err:?}");
                    }
                }
            }

            if !meteora_trades.is_empty() {
                if let Err(err) = insert_bonding_curve_trades(&writer_pool, &meteora_trades).await {
                    tracing::error!("failed to insert meteora trades: {err:?}");
                    continue;
                }

                for t in &meteora_trades {
                    let payload = serde_json::json!({
                        "signature": t.signature,
                        "slot": t.slot,
                        "mint_pubkey": t.mint_pubkey,
                        "trader": t.trader,
                        "side": t.side,
                        "token_amount": t.token_amount,
                        "sol_amount": t.sol_amount,
                        "price_nanos_per_token": t.price_nanos_per_token,
                        "venue": "meteora",
                        "tx_index": t.tx_index,
                        "ix_index": t.ix_index
                    });
                    if let Err(err) = insert_event(&writer_pool, "bonding", Some(&t.mint_pubkey), payload).await {
                        tracing::error!("failed to insert/notify meteora trade event: {err:?}");
                    }
                }
            }

            // Candle aggregation: process trades from all venues
            let all_trades = [pump_trades, raydium_trades, meteora_trades].concat();
            for t in &all_trades {
                let Some(bt) = t.block_time else { continue; };
                let bucket = bt.timestamp() - (bt.timestamp() % 60);
                let bucket_start = chrono::Utc.timestamp_opt(bucket, 0).single().unwrap();

                let c = Candle {
                    mint_pubkey: t.mint_pubkey.clone(),
                    timeframe_secs: 60,
                    bucket_start,
                    open: t.price_nanos_per_token,
                    high: t.price_nanos_per_token,
                    low: t.price_nanos_per_token,
                    close: t.price_nanos_per_token,
                    volume_token: t.token_amount,
                    volume_sol: t.sol_amount,
                    trades_count: 1,
                };

                if let Err(err) = upsert_candle(&writer_pool, &c).await {
                    tracing::error!("failed to upsert candle: {err:?}");
                    continue;
                }

                let payload = serde_json::json!({
                    "mint_pubkey": c.mint_pubkey,
                    "timeframe_secs": c.timeframe_secs,
                    "bucket_start": c.bucket_start,
                    "open": c.open,
                    "high": c.high,
                    "low": c.low,
                    "close": c.close,
                    "volume_token": c.volume_token,
                    "volume_sol": c.volume_sol,
                    "trades_count": c.trades_count
                });
                if let Err(err) = insert_event(&writer_pool, "candles", Some(&t.mint_pubkey), payload).await {
                    tracing::error!("failed to insert/notify candle event: {err:?}");
                }
            }

            if let Err(err) = set_last_processed_slot(&writer_pool, block.slot).await {
                tracing::error!("failed to update last_processed_slot: {err:?}");
            }
        }

        Result::<(), anyhow::Error>::Ok(())
    });

    // Firehose streaming task: connects to the configured Firehose endpoint and streams blocks.
    let firehose_config = config.firehose.clone();
    let last_slot = get_last_processed_slot(&pool).await.ok().flatten();

    let mut firehose_config_with_slot = firehose_config;
    if let Some(slot) = last_slot {
        firehose_config_with_slot.from_slot = Some(slot + 1);
    }

    let firehose_handle = tokio::spawn(async move {
        let mut client = FirehoseClient::new(firehose_config_with_slot);
        if let Err(e) = client.stream_blocks(block_tx).await {
            tracing::error!("Firehose stream failed: {e:?}");
        }
    });

    // Wait for either task to fail (they should run indefinitely).
    tokio::select! {
        result = writer_handle => {
            tracing::error!("Writer task ended: {result:?}");
        }
        result = firehose_handle => {
            tracing::error!("Firehose task ended: {result:?}");
        }
    }

    Ok(())
}

