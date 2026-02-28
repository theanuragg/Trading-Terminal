use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
 pub struct Mint {
     pub mint_pubkey: String,
     pub symbol: Option<String>,
     pub decimals: i32,
     pub first_seen_slot: i64,
 }

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
 pub struct TokenTransfer {
     pub signature: String,
     pub slot: i64,
     pub block_time: Option<DateTime<Utc>>,
     pub mint_pubkey: String,
     pub source_owner: String,
     pub dest_owner: String,
     pub source_ata: String,
     pub dest_ata: String,
    pub amount: i64,
     pub tx_index: i32,
     pub ix_index: i32,
 }

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
 pub struct Balance {
     pub wallet: String,
     pub mint_pubkey: String,
    pub amount: i64,
 }

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct BondingCurveTrade {
    pub signature: String,
    pub slot: i64,
    pub block_time: Option<DateTime<Utc>>,
    pub mint_pubkey: String,
    pub trader: String,
    pub side: String, // "buy" | "sell"
    pub token_amount: i64,
    pub sol_amount: i64,
    pub price_nanos_per_token: i64,
    pub tx_index: i32,
    pub ix_index: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Candle {
    pub mint_pubkey: String,
    pub timeframe_secs: i32,
    pub bucket_start: DateTime<Utc>,
    pub open: i64,
    pub high: i64,
    pub low: i64,
    pub close: i64,
    pub volume_token: i64,
    pub volume_sol: i64,
    pub trades_count: i32,
}

