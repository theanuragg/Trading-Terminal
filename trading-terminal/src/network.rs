use crate::app::Token;
use anyhow::Result;
use serde::{Deserialize, Serialize};
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_sdk::pubkey::Pubkey;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

const JUPITER_API_KEY: &str = "// will add config (todo) ";

#[derive(Clone)]
pub struct NetworkClient {
    pub rpc_client: Arc<RpcClient>,
}

impl NetworkClient {
    pub fn new(rpc_url: &str) -> Self {
        let rpc_client = RpcClient::new(rpc_url.to_string());
        Self {
            rpc_client: Arc::new(rpc_client),
        }
    }

    pub async fn get_block_height(&self) -> Result<u64> {
        let height = self.rpc_client.get_block_height().await?;
        Ok(height)
    }

    pub async fn get_balance(&self, pubkey: &Pubkey) -> Result<u64> {
        let balance = self.rpc_client.get_balance(pubkey).await?;
        Ok(balance)
    }
}

#[allow(non_snake_case)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiToken {
    pub id: u64,
    pub name: String,
    pub symbol: String,
    pub description: Option<String>,
    pub mintAddress: String,
    pub poolAddress: Option<String>,
    pub website: Option<String>,
    pub twitter: Option<String>,
    pub telegram: Option<String>,
    pub imageUrl: Option<String>,
    pub metadataUrl: Option<String>,
    pub creatorAddress: String,
    pub bondingCurveProgress: f64,
    pub volume: Option<f64>,
    pub liquidity: Option<f64>,
    pub marketCap: Option<f64>,
    pub holderCount: u64,
    pub stats5m: Option<StatsData>,
    pub stats1h: Option<StatsData>,
    pub stats6h: Option<StatsData>,
    pub stats24h: Option<StatsData>,
    pub createdAt: String,
    pub updatedAt: String,
}

#[allow(non_snake_case)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StatsData {
    #[serde(default)]
    pub numBuys: u64,
    #[serde(default)]
    pub numSells: u64,
    #[serde(default)]
    pub buyVolume: f64,
    #[serde(default)]
    pub numTraders: u64,
    #[serde(default)]
    pub sellVolume: f64,
    #[serde(default)]
    pub priceChange: f64,
    #[serde(default)]
    pub holderChange: Option<f64>,
    #[serde(default)]
    pub numNetBuyers: i64,
    #[serde(default)]
    pub liquidityChange: Option<f64>,
}

#[derive(Debug, Deserialize)]
pub struct TokensResponse {
    pub success: bool,
    pub tokens: Vec<ApiToken>,
}

#[derive(Debug, Deserialize)]
pub struct SingleTokenResponse {
    pub success: bool,
    pub token: ApiToken,
}

// New Jupiter API v2 Structures
#[allow(non_snake_case)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JupiterTokenInfo {
    pub address: String,
    pub name: String,
    pub symbol: String,
    pub decimals: u8,
    pub tags: Option<Vec<String>>,
    pub logoURI: Option<String>,
    pub daily_volume: Option<String>,
    pub freeze_authority: Option<String>,
    pub mint_authority: Option<String>,
    pub permanent_delegate: Option<String>,
    pub minted_at: Option<String>,
    pub extensions: Option<serde_json::Value>,
}

#[allow(non_snake_case)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JupiterTokenData {
    pub address: String,
    pub name: String,
    pub symbol: String,
    pub decimals: u8,
    pub logoURI: Option<String>,
    pub volume24h: Option<f64>,
    pub liquidity: Option<f64>,
    pub mcap: Option<f64>,
    pub holderCount: Option<u64>,
    pub stats5m: Option<StatsData>,
    pub stats1h: Option<StatsData>,
    pub stats6h: Option<StatsData>,
    pub stats24h: Option<StatsData>,
    pub last_trade_unix_time: Option<u64>,
    pub last_trade_human_time: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JupiterTokenSearchResponse {
    #[serde(default)]
    pub data: Vec<JupiterTokenInfo>,
    #[serde(default)]
    pub tokens: Vec<JupiterTokenData>,
}

// Jupiter API Models
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JupiterCandle {
    pub time: u64, // normalized to seconds
    pub open: f64,
    pub high: f64,
    pub low: f64,
    pub close: f64,
    #[serde(default)]
    pub volume: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JupiterChartResponse {
    pub success: Option<bool>,
    pub candles: Option<Vec<JupiterCandle>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JupiterTransaction {
    pub txid: String,
    pub blockTime: Option<u64>,
    pub mint: String,
    pub tokenSymbol: Option<String>,
    pub tokenName: Option<String>,
    pub amount: f64,
    pub price: f64,
    pub tradeDirection: String,
    pub maker: String,
    pub vTokens: Option<f64>,
    pub vSolChange: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JupiterTransactionResponse {
    pub success: Option<bool>,
    pub txs: Option<Vec<JupiterTransaction>>,
    // some endpoints may return `data`
    pub data: Option<Vec<JupiterTransaction>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JupiterHolder {
    pub address: String,
    pub percent: Option<f64>,
    pub amount: Option<f64>,
    #[serde(default)]
    pub uiAmount: Option<f64>,
    #[serde(default)]
    pub uiAmountString: Option<String>,
    #[serde(default)]
    pub decimals: Option<u8>,
    #[serde(default)]
    pub solBalanceDisplay: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JupiterHolderResponse {
    pub success: Option<bool>,
    pub holders: Option<Vec<JupiterHolder>>,
    pub data: Option<Vec<JupiterHolder>>,
}

pub struct IndexerClient {
    pub client: reqwest::Client,
    pub indexer_api_url: String,
}

impl Clone for IndexerClient {
    fn clone(&self) -> Self {
        Self {
            client: self.client.clone(),
            indexer_api_url: self.indexer_api_url.clone(),
        }
    }
}

impl IndexerClient {
    pub fn new(indexer_api_url: String) -> Self {
        Self {
            client: reqwest::Client::new(),
            indexer_api_url,
        }
    }

    fn verbose_log(&self, msg: &str) {
        if std::env::var("TX_TERMINAL_VERBOSE").is_ok() {
            eprintln!("{}", msg);
        }
    }

    pub async fn fetch_tokens_from_api(&self) -> Result<Vec<Token>> {
        let url = "https://onlyfounders.fun/api/tokens";
        let response = self.client.get(url).send().await?;
        let data: TokensResponse = response.json().await?;

        let tokens = data
            .tokens
            .into_iter()
            .map(|api_token| {
                let change_24h = api_token
                    .stats24h
                    .as_ref()
                    .map(|s| s.priceChange)
                    .unwrap_or(0.0);

                let volume = api_token
                    .stats24h
                    .as_ref()
                    .map(|s| s.buyVolume + s.sellVolume)
                    .or(api_token.volume)
                    .unwrap_or(0.0);

                let img_char = api_token.symbol.chars().next().unwrap_or('?');
                let img_str = img_char.to_string();
                let bonding_progress = (api_token.bondingCurveProgress * 100.0).min(100.0);

                Token {
                    name: api_token.name.clone(),
                    symbol: api_token.symbol.clone(),
                    price: 0.0, // Not provided in API
                    market_cap: api_token.marketCap.unwrap_or(0.0),
                    change_24h,
                    volume,
                    txns: (api_token
                        .stats24h
                        .as_ref()
                        .map(|s| s.numBuys + s.numSells)
                        .unwrap_or(0)) as u32,
                    image_asc: img_str,
                    image_url: api_token.imageUrl.clone().unwrap_or_default(),
                    bonding: bonding_progress,
                    mint: api_token.mintAddress.clone(),
                }
            })
            .collect();

        Ok(tokens)
    }

    pub async fn fetch_token_by_mint(&self, mint_address: &str) -> Result<ApiToken> {
        let url = format!("https://onlyfounders.fun/api/tokens/{}", mint_address);
        let response = self.client.get(&url).send().await?;
        let data: SingleTokenResponse = response.json().await?;
        Ok(data.token)
    }

    pub async fn fetch_tokens(&self) -> Result<Vec<String>> {
        // Placeholder for fetching tokens from an indexer
        Ok(vec![
            "SOL".to_string(),
            "USDC".to_string(),
            "BONK".to_string(),
        ])
    }

    // Jupiter API Methods
    pub async fn fetch_chart_data(
        &self,
        token_id: &str,
        interval: &str,
        candle_count: u32,
    ) -> Result<Vec<JupiterCandle>> {
        // datapi expects `to` in milliseconds. Try a couple interval variants if the first fails.
        let now_ms = SystemTime::now().duration_since(UNIX_EPOCH)?.as_millis();

        let mut candidates: Vec<String> = vec![interval.to_string()];
        // second candidate: transform `1h` -> `1_HOUR`, `1m` -> `1_MINUTE`, `1d` -> `1_DAY`
        let mapped = match interval {
            "1m" => "1_MINUTE",
            "5m" => "5_MINUTES",
            "15m" => "15_MINUTES",
            "1h" => "1_HOUR",
            "4h" => "4_HOURS",
            "1d" => "1_DAY",
            other => other,
        };
        if mapped != interval {
            candidates.push(mapped.to_string());
        }

        for intv in candidates {
            let url = format!(
                "https://datapi.jup.ag/v2/charts/{}?interval={}&to={}&candles={}&type=price",
                token_id, intv, now_ms, candle_count
            );

            let resp = self
                .client
                .get(&url)
                .header("x-api-key", JUPITER_API_KEY)
                .header("accept", "application/json")
                .send()
                .await;

            match resp {
                Ok(r) => {
                    let status = r.status();
                    let body = r.text().await.unwrap_or_default();
                    if !status.is_success() {
                        self.verbose_log(&format!("[CHART] datapi HTTP {} body: {}", status, body));
                        continue;
                    }

                    match serde_json::from_str::<JupiterChartResponse>(&body) {
                        Ok(mut parsed) => {
                            let mut candles = parsed.candles.unwrap_or_default();
                            // Normalize times to seconds
                            for c in candles.iter_mut() {
                                if c.time > 1_000_000_000_000u64 {
                                    c.time = c.time / 1000u64;
                                }
                            }
                            if !candles.is_empty() {
                                self.verbose_log(&format!(
                                    "[CHART] datapi returned {} candles (interval={})",
                                    candles.len(),
                                    intv
                                ));
                                return Ok(candles);
                            }
                        }
                        Err(e) => self
                            .verbose_log(&format!("[CHART] parse error: {} | body: {}", e, body)),
                    }
                }
                Err(e) => self.verbose_log(&format!("[CHART] request error: {}", e)),
            }
        }

        Ok(vec![])
    }

    pub async fn fetch_transactions(
        &self,
        mint: &str,
        limit: Option<u32>,
    ) -> Result<Vec<JupiterTransaction>> {
        let limit_param = limit.unwrap_or(50).min(100) as usize;

        // Try indexer API first
        let url = format!(
            "{}/token/{}/transfers?limit={}",
            self.indexer_api_url, mint, limit_param
        );
        match self.client.get(&url).send().await {
            Ok(resp) => {
                let status = resp.status();
                let body = resp.text().await.unwrap_or_default();
                if status.is_success() {
                    if let Ok(json_val) = serde_json::from_str::<serde_json::Value>(&body) {
                        // If API returned an array of transfers
                        if let Some(arr) = json_val.as_array() {
                            let mut out = Vec::new();
                            for item in arr {
                                let txid = item
                                    .get("tx_signature")
                                    .or_else(|| item.get("txHash"))
                                    .or_else(|| item.get("txid"))
                                    .and_then(|v| v.as_str())
                                    .map(|s| s.to_string())
                                    .unwrap_or_default();
                                let block_time = item.get("block_time").and_then(|v| v.as_u64());
                                let amount =
                                    item.get("amount").and_then(|v| v.as_f64()).unwrap_or(0.0);
                                let maker = item
                                    .get("from")
                                    .or_else(|| item.get("trader"))
                                    .or_else(|| item.get("traderAddress"))
                                    .and_then(|v| v.as_str())
                                    .map(|s| s.to_string())
                                    .unwrap_or_default();
                                let trade_type = item
                                    .get("type")
                                    .and_then(|v| v.as_str())
                                    .or_else(|| item.get("tradeDirection").and_then(|v| v.as_str()))
                                    .map(|s| s.to_string())
                                    .unwrap_or_else(|| "trade".to_string());

                                out.push(JupiterTransaction {
                                    txid,
                                    blockTime: block_time,
                                    mint: mint.to_string(),
                                    tokenSymbol: None,
                                    tokenName: None,
                                    amount,
                                    price: item
                                        .get("usdPrice")
                                        .and_then(|v| v.as_f64())
                                        .unwrap_or(0.0),
                                    tradeDirection: trade_type,
                                    maker,
                                    vTokens: Some(amount),
                                    vSolChange: None,
                                });
                            }

                            if !out.is_empty() {
                                // only verbose
                                self.verbose_log(&format!(
                                    "[TX] indexer returned {} rows",
                                    out.len()
                                ));
                                return Ok(out);
                            }
                        }
                    } else {
                        self.verbose_log(&format!("[TX] indexer response parse failed: {}", body));
                    }
                } else {
                    self.verbose_log(&format!("[TX] indexer HTTP {} body: {}", status, body));
                }
            }
            Err(e) => self.verbose_log(&format!("[TX] indexer request error: {}", e)),
        }

        // Fallback to datapi v1
        // verbose-only fallback notice
        self.verbose_log(&format!("[TX] falling back to datapi for {}", mint));
        match self.fetch_jupiter_txs(mint, limit_param).await {
            Ok(mut txs) => {
                self.verbose_log(&format!("[TX] datapi returned {} txs", txs.len()));
                Ok(txs)
            }
            Err(e) => {
                self.verbose_log(&format!("[TX] datapi error: {}", e));
                Ok(vec![])
            }
        }
    }

    pub async fn fetch_holders(
        &self,
        mint: &str,
        limit: Option<u32>,
    ) -> Result<Vec<JupiterHolder>> {
        let limit = limit.unwrap_or(100) as usize;

        // Try indexer first
        let url = format!(
            "{}/token/{}/holders?limit={}",
            self.indexer_api_url, mint, limit
        );
        if let Ok(resp) = self.client.get(&url).send().await {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            if status.is_success() {
                if let Ok(json_val) = serde_json::from_str::<serde_json::Value>(&body) {
                    if let Some(arr) = json_val
                        .as_array()
                        .or_else(|| json_val.get("holders").and_then(|v| v.as_array()))
                    {
                        let mut out = Vec::new();
                        for item in arr.iter().take(limit) {
                            let address = item
                                .get("address")
                                .or_else(|| item.get("owner"))
                                .and_then(|v| v.as_str())
                                .map(|s| s.to_string())
                                .unwrap_or_default();
                            let amount = item.get("amount").and_then(|v| v.as_f64()).unwrap_or(0.0);
                            let pct = item.get("percent").and_then(|v| v.as_f64()).unwrap_or(0.0);
                            let ui_amount = item
                                .get("ui_amount")
                                .and_then(|v| v.as_f64())
                                .or_else(|| item.get("uiAmount").and_then(|v| v.as_f64()));
                            let ui_str = item
                                .get("ui_amount_string")
                                .and_then(|v| v.as_str())
                                .map(|s| s.to_string());
                            let decimals = item
                                .get("decimals")
                                .and_then(|v| v.as_u64())
                                .map(|v| v as u8);
                            out.push(JupiterHolder {
                                address,
                                amount: Some(amount),
                                percent: Some(pct),
                                uiAmount: ui_amount,
                                uiAmountString: ui_str,
                                decimals,
                                solBalanceDisplay: None,
                            });
                        }
                        if !out.is_empty() {
                            self.verbose_log(&format!(
                                "[HOLDERS] indexer returned {} holders",
                                out.len()
                            ));
                            return Ok(out);
                        }
                    }
                } else {
                    self.verbose_log(&format!("[HOLDERS] indexer parse failed: {}", body));
                }
            } else {
                self.verbose_log(&format!("[HOLDERS] indexer HTTP {} body: {}", status, body));
            }
        }

        // Fallback to datapi
        self.verbose_log(&format!("[HOLDERS] falling back to datapi for {}", mint));
        match self.fetch_jupiter_holders(mint, limit).await {
            Ok(mut holders) => {
                self.verbose_log(&format!(
                    "[HOLDERS] datapi returned {} holders",
                    holders.len()
                ));
                Ok(holders)
            }
            Err(e) => {
                self.verbose_log(&format!("[HOLDERS] datapi error: {}", e));
                Ok(vec![])
            }
        }
    }

    // New Jupiter API v2 Token Search
    pub async fn fetch_token_info(&self, query: &str) -> Result<JupiterTokenData> {
        let url = format!(
            "https://api.jup.ag/tokens/v2/search?query={}",
            urlencoding::encode(query)
        );

        let response = self
            .client
            .get(&url)
            .header("X-API-KEY", JUPITER_API_KEY)
            .send()
            .await?;

        let search_response: JupiterTokenSearchResponse = response.json().await?;

        // Return the first result if available
        if !search_response.tokens.is_empty() {
            Ok(search_response.tokens[0].clone())
        } else if !search_response.data.is_empty() {
            // Fallback: construct token data from basic info
            let token_info = &search_response.data[0];
            Ok(JupiterTokenData {
                address: token_info.address.clone(),
                name: token_info.name.clone(),
                symbol: token_info.symbol.clone(),
                decimals: token_info.decimals,
                logoURI: token_info.logoURI.clone(),
                volume24h: None,
                liquidity: None,
                mcap: None,
                holderCount: None,
                stats5m: None,
                stats1h: None,
                stats6h: None,
                stats24h: None,
                last_trade_unix_time: None,
                last_trade_human_time: None,
            })
        } else {
            anyhow::bail!("No token found for query: {}", query)
        }
    }

    pub async fn fetch_token_chart_data(
        &self,
        mint: &str,
        interval: &str,
    ) -> Result<Vec<JupiterCandle>> {
        // Try indexer API first (better coverage)
        let timeframe_secs = match interval {
            "1m" => 60,
            "5m" => 300,
            "15m" => 900,
            "1h" => 3600,
            "4h" => 14400,
            "1d" => 86400,
            _ => 3600, // default to 1h
        };

        let url = format!(
            "{}/token/{}/candles?timeframe_secs={}&limit=96",
            self.indexer_api_url, mint, timeframe_secs
        );

        match self.client.get(&url).send().await {
            Ok(response) => {
                if response.status().is_success() {
                    if let Ok(candles) = response.json::<Vec<serde_json::Value>>().await {
                        // Convert indexer candles to JupiterCandle format
                        let converted: Vec<JupiterCandle> = candles
                            .into_iter()
                            .filter_map(|c| {
                                // timestamp may be in seconds
                                let t = c.get("timestamp").and_then(|v| v.as_u64()).unwrap_or(0);
                                Some(JupiterCandle {
                                    time: t as u64,
                                    open: c.get("open")?.as_f64().unwrap_or(0.0),
                                    high: c.get("high")?.as_f64().unwrap_or(0.0),
                                    low: c.get("low")?.as_f64().unwrap_or(0.0),
                                    close: c.get("close")?.as_f64().unwrap_or(0.0),
                                    volume: c.get("volume")?.as_f64(),
                                })
                            })
                            .collect();

                        if !converted.is_empty() {
                            return Ok(converted);
                        }
                    }
                }
            }
            Err(_) => {}
        }

        // Fallback to datapi if indexer fails
        let candle_count = 96; // Default candle count
        match self.fetch_chart_data(mint, interval, candle_count).await {
            Ok(c) if !c.is_empty() => return Ok(c),
            _ => {}
        }

        // Return empty if both fail
        Ok(vec![])
    }

    /// Fetch candles from Jupiter datapi v2. `interval` should match datapi values (e.g. "1_MINUTE","1_HOUR")
    pub async fn fetch_jupiter_chart(
        &self,
        token_id: &str,
        interval: &str,
        candles: usize,
    ) -> Result<Vec<JupiterCandle>> {
        let now_ms = SystemTime::now().duration_since(UNIX_EPOCH)?.as_millis();
        let url = format!(
            "https://datapi.jup.ag/v2/charts/{}?interval={}&to={}&candles={}&type=price",
            token_id, interval, now_ms, candles
        );
        let client = reqwest::Client::new();
        let resp = client
            .get(&url)
            .header("x-api-key", JUPITER_API_KEY)
            .header("accept", "application/json")
            .send()
            .await?;
        let status = resp.status();
        let text = resp.text().await?;
        if !status.is_success() {
            anyhow::bail!("Jupiter chart API error {}: {}", status, text);
        }
        let parsed: JupiterChartResponse = serde_json::from_str(&text)
            .map_err(|e| anyhow::anyhow!("Parse chart response: {} | body: {}", e, text))?;

        let mut candles = parsed.candles.unwrap_or_default();

        // Normalize time to seconds (lightweight-charts / many consumers expect seconds)
        for c in candles.iter_mut() {
            if c.time > 1_000_000_000_000u64 {
                // probably ms
                c.time = c.time / 1000u64;
            }
        }

        Ok(candles)
    }

    /// Fetch transactions (txs) from Jupiter datapi v1
    pub async fn fetch_jupiter_txs(
        &self,
        token_id: &str,
        limit: usize,
    ) -> Result<Vec<JupiterTransaction>> {
        let url = format!(
            "https://datapi.jup.ag/v1/txs/{}?limit={}&dir=desc&types=buy%2Csell",
            token_id, limit
        );
        let resp = self
            .client
            .get(&url)
            .header("x-api-key", JUPITER_API_KEY)
            .header("accept", "application/json")
            .send()
            .await?;
        let status = resp.status();
        let text = resp.text().await?;
        if !status.is_success() {
            anyhow::bail!("Jupiter txs API error {}: {}", status, text);
        }

        let v: serde_json::Value = serde_json::from_str(&text)?;
        let arr = v
            .get("txs")
            .or_else(|| v.get("data"))
            .and_then(|x| x.as_array())
            .cloned()
            .unwrap_or_default();
        let mut out = Vec::new();
        for item in arr.into_iter().take(limit) {
            let txid = item
                .get("tx_signature")
                .or_else(|| item.get("signature"))
                .or_else(|| item.get("txHash"))
                .or_else(|| item.get("txid"))
                .and_then(|v| v.as_str())
                .map(|s| s.to_string())
                .unwrap_or_default();
            let block_time = item
                .get("block_time")
                .and_then(|v| v.as_u64())
                .or_else(|| item.get("timestamp").and_then(|v| v.as_u64()));
            let amount = item
                .get("nativeVolume")
                .and_then(|v| v.as_f64())
                .or_else(|| item.get("native_amount").and_then(|v| v.as_f64()))
                .or_else(|| item.get("amount").and_then(|v| v.as_f64()))
                .unwrap_or(0.0);
            let price = item
                .get("usdPrice")
                .and_then(|v| v.as_f64())
                .or_else(|| item.get("price").and_then(|v| v.as_f64()))
                .unwrap_or(0.0);
            let maker = item
                .get("traderAddress")
                .or_else(|| item.get("trader"))
                .or_else(|| item.get("maker"))
                .or_else(|| item.get("from"))
                .and_then(|v| v.as_str())
                .map(|s| s.to_string())
                .unwrap_or_default();
            let trade_type = item
                .get("type")
                .and_then(|v| v.as_str())
                .or_else(|| item.get("tradeDirection").and_then(|v| v.as_str()))
                .or_else(|| item.get("side").and_then(|v| v.as_str()))
                .map(|s| s.to_string())
                .unwrap_or_else(|| "trade".to_string());

            out.push(JupiterTransaction {
                txid,
                blockTime: block_time,
                mint: token_id.to_string(),
                tokenSymbol: item
                    .get("tokenSymbol")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string()),
                tokenName: item
                    .get("tokenName")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string()),
                amount,
                price,
                tradeDirection: trade_type,
                maker,
                vTokens: Some(amount),
                vSolChange: None,
            });
        }

        Ok(out)
    }

    /// Fetch holders from Jupiter datapi v1
    pub async fn fetch_jupiter_holders(
        &self,
        token_id: &str,
        limit: usize,
    ) -> Result<Vec<JupiterHolder>> {
        let url = format!(
            "https://datapi.jup.ag/v1/holders/{}?limit={}",
            token_id, limit
        );
        let resp = self
            .client
            .get(&url)
            .header("x-api-key", JUPITER_API_KEY)
            .header("accept", "application/json")
            .send()
            .await?;
        let status = resp.status();
        let text = resp.text().await?;
        if !status.is_success() {
            anyhow::bail!("Jupiter holders API error {}: {}", status, text);
        }

        let v: serde_json::Value = serde_json::from_str(&text)?;
        let arr = v
            .get("holders")
            .or_else(|| v.get("data"))
            .and_then(|x| x.as_array())
            .cloned()
            .unwrap_or_default();
        let mut out = Vec::new();
        for item in arr.into_iter().take(limit) {
            let address = item
                .get("address")
                .or_else(|| item.get("owner"))
                .and_then(|v| v.as_str())
                .map(|s| s.to_string())
                .unwrap_or_default();
            let amount = item
                .get("amount")
                .and_then(|v| v.as_f64())
                .or_else(|| item.get("uiAmount").and_then(|v| v.as_f64()));
            let percent = item.get("percent").and_then(|v| v.as_f64());
            let ui_str = item
                .get("uiAmountString")
                .or_else(|| item.get("ui_amount_string"))
                .and_then(|v| v.as_str())
                .map(|s| s.to_string());
            let decimals = item
                .get("decimals")
                .and_then(|v| v.as_u64())
                .map(|v| v as u8);

            out.push(JupiterHolder {
                address,
                percent,
                amount,
                uiAmount: amount,
                uiAmountString: ui_str,
                decimals,
                solBalanceDisplay: None,
            });
        }

        Ok(out)
    }

    /// Optional: search token metadata via datapi tokens search endpoint
    pub async fn jupiter_search_token(&self, query: &str) -> Result<Vec<JupiterTokenInfo>> {
        let url = format!("https://api.jup.ag/tokens/v2/search?query={}", query);
        let client = reqwest::Client::new();
        let resp = client
            .get(&url)
            .header("x-api-key", JUPITER_API_KEY)
            .header("accept", "application/json")
            .send()
            .await?;
        let status = resp.status();
        let text = resp.text().await?;
        if !status.is_success() {
            anyhow::bail!("Jupiter search API error {}: {}", status, text);
        }
        // API returns tokens array; try to parse leniently
        let v: serde_json::Value = serde_json::from_str(&text)?;
        let tokens = v
            .get("tokens")
            .and_then(|t| t.as_array())
            .unwrap_or(&vec![])
            .clone();
        let mut out = Vec::new();
        for t in tokens {
            if let Ok(info) = serde_json::from_value::<JupiterTokenInfo>(t) {
                out.push(info);
            }
        }
        Ok(out)
    }
}
