use crate::models::{Balance, BondingCurveTrade, Candle, Mint, TokenTransfer};
use anyhow::Result;
use sqlx::{postgres::PgPoolOptions, PgPool, Row};

 pub async fn create_pool(database_url: &str, max_connections: u32) -> Result<PgPool> {
     let pool = PgPoolOptions::new()
         .max_connections(max_connections)
         .connect(database_url)
         .await?;
     Ok(pool)
 }

pub async fn run_migrations(pool: &PgPool) -> Result<()> {
    // Embed migrations from the `indexer/migrations` directory.
    sqlx::migrate!("../migrations").run(pool).await?;
    Ok(())
}

pub async fn insert_transfers(pool: &PgPool, transfers: &[TokenTransfer]) -> Result<()> {
     if transfers.is_empty() {
         return Ok(());
     }

    for t in transfers {
        sqlx::query(
            r#"
            INSERT INTO token_transfers (
                signature,
                slot,
                block_time,
                mint_pubkey,
                source_owner,
                dest_owner,
                source_ata,
                dest_ata,
                amount,
                tx_index,
                ix_index
            ) VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11)
            ON CONFLICT (signature, ix_index) DO NOTHING
            "#,
        )
        .bind(&t.signature)
        .bind(t.slot)
        .bind(t.block_time)
        .bind(&t.mint_pubkey)
        .bind(&t.source_owner)
        .bind(&t.dest_owner)
        .bind(&t.source_ata)
        .bind(&t.dest_ata)
        .bind(t.amount)
        .bind(t.tx_index)
        .bind(t.ix_index)
        .execute(pool)
        .await?;
    }
     Ok(())
 }

pub async fn upsert_mints(pool: &PgPool, mints: &[Mint]) -> Result<()> {
     if mints.is_empty() {
         return Ok(());
     }

    for m in mints {
        sqlx::query(
            r#"
            INSERT INTO mints (mint_pubkey, symbol, decimals, first_seen_slot)
            VALUES ($1,$2,$3,$4)
            ON CONFLICT (mint_pubkey) DO UPDATE
            SET symbol = COALESCE(EXCLUDED.symbol, mints.symbol),
                decimals = EXCLUDED.decimals,
                first_seen_slot = LEAST(mints.first_seen_slot, EXCLUDED.first_seen_slot)
            "#,
        )
        .bind(&m.mint_pubkey)
        .bind(&m.symbol)
        .bind(m.decimals)
        .bind(m.first_seen_slot)
        .execute(pool)
        .await?;
     }
     Ok(())
 }

pub async fn update_balances_for_transfers(pool: &PgPool, transfers: &[TokenTransfer]) -> Result<()> {
     if transfers.is_empty() {
         return Ok(());
     }

     for t in transfers {
         // source wallet loses amount
         apply_delta(pool, &t.source_owner, &t.mint_pubkey, -t.amount).await?;
         // dest wallet gains amount
         apply_delta(pool, &t.dest_owner, &t.mint_pubkey, t.amount).await?;
     }
     Ok(())
 }

 async fn apply_delta(
    pool: &PgPool,
     wallet: &str,
     mint_pubkey: &str,
    delta: i64,
 ) -> Result<()> {
    sqlx::query(
        r#"
        INSERT INTO balances (wallet, mint_pubkey, amount)
        VALUES ($1,$2,$3)
        ON CONFLICT (wallet, mint_pubkey)
        DO UPDATE SET amount = balances.amount + EXCLUDED.amount
        "#,
    )
    .bind(wallet)
    .bind(mint_pubkey)
    .bind(delta)
    .execute(pool)
    .await?;
     Ok(())
 }

pub async fn get_token_transfers_for_mint(
    pool: &PgPool,
    mint_pubkey: &str,
    limit: i64,
    before_slot: Option<i64>,
) -> Result<Vec<TokenTransfer>> {
    let rows = if let Some(before) = before_slot {
        sqlx::query_as::<_, TokenTransfer>(
            r#"
            SELECT
                signature,
                slot,
                block_time,
                mint_pubkey,
                source_owner,
                dest_owner,
                source_ata,
                dest_ata,
                amount,
                tx_index,
                ix_index
            FROM token_transfers
            WHERE mint_pubkey = $1
              AND slot < $2
            ORDER BY slot DESC
            LIMIT $3
            "#,
        )
        .bind(mint_pubkey)
        .bind(before)
        .bind(limit)
        .fetch_all(pool)
        .await?
    } else {
        sqlx::query_as::<_, TokenTransfer>(
            r#"
            SELECT
                signature,
                slot,
                block_time,
                mint_pubkey,
                source_owner,
                dest_owner,
                source_ata,
                dest_ata,
                amount,
                tx_index,
                ix_index
            FROM token_transfers
            WHERE mint_pubkey = $1
            ORDER BY slot DESC
            LIMIT $2
            "#,
        )
        .bind(mint_pubkey)
        .bind(limit)
        .fetch_all(pool)
        .await?
    };

    Ok(rows)
}

pub async fn get_balances_for_mint(
    pool: &PgPool,
    mint_pubkey: &str,
    limit: i64,
    offset: i64,
) -> Result<Vec<Balance>> {
    let rows = sqlx::query_as::<_, Balance>(
        r#"
        SELECT
            wallet,
            mint_pubkey,
            amount
        FROM balances
        WHERE mint_pubkey = $1
        ORDER BY amount DESC
        LIMIT $2 OFFSET $3
        "#,
    )
    .bind(mint_pubkey)
    .bind(limit)
    .bind(offset)
    .fetch_all(pool)
    .await?;

    Ok(rows)
}

pub async fn get_portfolio_for_wallet(pool: &PgPool, wallet: &str) -> Result<Vec<Balance>> {
    let rows = sqlx::query_as::<_, Balance>(
        r#"
        SELECT
            wallet,
            mint_pubkey,
            amount
        FROM balances
        WHERE wallet = $1
        ORDER BY amount DESC
        "#,
    )
    .bind(wallet)
    .fetch_all(pool)
    .await?;

    Ok(rows)
}

pub async fn insert_event(
    pool: &PgPool,
    topic: &str,
    mint_pubkey: Option<&str>,
    payload: serde_json::Value,
) -> Result<()> {
    sqlx::query(
        r#"
        INSERT INTO indexer_events (topic, mint_pubkey, payload)
        VALUES ($1,$2,$3)
        "#,
    )
    .bind(topic)
    .bind(mint_pubkey)
    .bind(&payload)
    .execute(pool)
    .await?;

    // Mirror the insert via NOTIFY for websocket consumers.
    // Payload is small JSON: {topic, mint_pubkey, payload}
    let notify_payload = serde_json::json!({
        "topic": topic,
        "mint_pubkey": mint_pubkey,
        "payload": payload
    })
    .to_string();

    sqlx::query(r#"SELECT pg_notify('indexer_events', $1)"#)
        .bind(notify_payload)
        .execute(pool)
        .await?;

    Ok(())
}

pub async fn insert_bonding_curve_trades(pool: &PgPool, trades: &[BondingCurveTrade]) -> Result<()> {
    if trades.is_empty() {
        return Ok(());
    }

    for t in trades {
        sqlx::query(
            r#"
            INSERT INTO bonding_curve_trades (
                signature,
                slot,
                block_time,
                mint_pubkey,
                trader,
                side,
                token_amount,
                sol_amount,
                price_nanos_per_token,
                tx_index,
                ix_index
            ) VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11)
            ON CONFLICT (signature, ix_index) DO NOTHING
            "#,
        )
        .bind(&t.signature)
        .bind(t.slot)
        .bind(t.block_time)
        .bind(&t.mint_pubkey)
        .bind(&t.trader)
        .bind(&t.side)
        .bind(t.token_amount)
        .bind(t.sol_amount)
        .bind(t.price_nanos_per_token)
        .bind(t.tx_index)
        .bind(t.ix_index)
        .execute(pool)
        .await?;
    }

    Ok(())
}

pub async fn upsert_candle(
    pool: &PgPool,
    candle: &Candle,
) -> Result<()> {
    sqlx::query(
        r#"
        INSERT INTO candles (
            mint_pubkey,
            timeframe_secs,
            bucket_start,
            open,
            high,
            low,
            close,
            volume_token,
            volume_sol,
            trades_count
        ) VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10)
        ON CONFLICT (mint_pubkey, timeframe_secs, bucket_start)
        DO UPDATE SET
            high = GREATEST(candles.high, EXCLUDED.high),
            low = LEAST(candles.low, EXCLUDED.low),
            close = EXCLUDED.close,
            volume_token = candles.volume_token + EXCLUDED.volume_token,
            volume_sol = candles.volume_sol + EXCLUDED.volume_sol,
            trades_count = candles.trades_count + EXCLUDED.trades_count
        "#,
    )
    .bind(&candle.mint_pubkey)
    .bind(candle.timeframe_secs)
    .bind(candle.bucket_start)
    .bind(candle.open)
    .bind(candle.high)
    .bind(candle.low)
    .bind(candle.close)
    .bind(candle.volume_token)
    .bind(candle.volume_sol)
    .bind(candle.trades_count)
    .execute(pool)
    .await?;

    Ok(())
}

pub async fn get_candles(
    pool: &PgPool,
    mint_pubkey: &str,
    timeframe_secs: i32,
    limit: i64,
    before: Option<chrono::DateTime<chrono::Utc>>,
) -> Result<Vec<Candle>> {
    let rows = if let Some(before_ts) = before {
        sqlx::query_as::<_, Candle>(
            r#"
            SELECT
                mint_pubkey,
                timeframe_secs,
                bucket_start,
                open,
                high,
                low,
                close,
                volume_token,
                volume_sol,
                trades_count
            FROM candles
            WHERE mint_pubkey = $1
              AND timeframe_secs = $2
              AND bucket_start < $3
            ORDER BY bucket_start DESC
            LIMIT $4
            "#,
        )
        .bind(mint_pubkey)
        .bind(timeframe_secs)
        .bind(before_ts)
        .bind(limit)
        .fetch_all(pool)
        .await?
    } else {
        sqlx::query_as::<_, Candle>(
            r#"
            SELECT
                mint_pubkey,
                timeframe_secs,
                bucket_start,
                open,
                high,
                low,
                close,
                volume_token,
                volume_sol,
                trades_count
            FROM candles
            WHERE mint_pubkey = $1
              AND timeframe_secs = $2
            ORDER BY bucket_start DESC
            LIMIT $3
            "#,
        )
        .bind(mint_pubkey)
        .bind(timeframe_secs)
        .bind(limit)
        .fetch_all(pool)
        .await?
    };

    Ok(rows)
}

pub async fn get_bonding_trades_for_mint(
    pool: &PgPool,
    mint_pubkey: &str,
    limit: i64,
    before_slot: Option<i64>,
) -> Result<Vec<BondingCurveTrade>> {
    let rows = if let Some(before) = before_slot {
        sqlx::query_as::<_, BondingCurveTrade>(
            r#"
            SELECT
                signature,
                slot,
                block_time,
                mint_pubkey,
                trader,
                side,
                token_amount,
                sol_amount,
                price_nanos_per_token,
                tx_index,
                ix_index
            FROM bonding_curve_trades
            WHERE mint_pubkey = $1
              AND slot < $2
            ORDER BY slot DESC
            LIMIT $3
            "#,
        )
        .bind(mint_pubkey)
        .bind(before)
        .bind(limit)
        .fetch_all(pool)
        .await?
    } else {
        sqlx::query_as::<_, BondingCurveTrade>(
            r#"
            SELECT
                signature,
                slot,
                block_time,
                mint_pubkey,
                trader,
                side,
                token_amount,
                sol_amount,
                price_nanos_per_token,
                tx_index,
                ix_index
            FROM bonding_curve_trades
            WHERE mint_pubkey = $1
            ORDER BY slot DESC
            LIMIT $2
            "#,
        )
        .bind(mint_pubkey)
        .bind(limit)
        .fetch_all(pool)
        .await?
    };

    Ok(rows)
}

 pub async fn get_last_processed_slot(pool: &PgPool) -> Result<Option<i64>> {
    let rec = sqlx::query(
        r#"
        SELECT slot FROM last_processed_slot
        WHERE id = 1
        "#,
    )
    .fetch_optional(pool)
    .await?;

    Ok(rec.map(|row| row.get::<i64, _>("slot")))
 }

 pub async fn set_last_processed_slot(pool: &PgPool, slot: i64) -> Result<()> {
    sqlx::query(
        r#"
        INSERT INTO last_processed_slot (id, slot)
        VALUES (1, $1)
        ON CONFLICT (id) DO UPDATE SET slot = EXCLUDED.slot
        "#,
    )
    .bind(slot)
    .execute(pool)
    .await?;
     Ok(())
 }

