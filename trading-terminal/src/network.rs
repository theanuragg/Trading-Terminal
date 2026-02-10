use anyhow::Result;
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_sdk::pubkey::Pubkey;
use std::sync::Arc;

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

pub struct IndexerClient {
    pub client: reqwest::Client,
}

impl IndexerClient {
    pub fn new() -> Self {
        Self {
            client: reqwest::Client::new(),
        }
    }

    pub async fn fetch_tokens(&self) -> Result<Vec<String>> {
        // Placeholder for fetching tokens from an indexer
        Ok(vec![
            "SOL".to_string(),
            "USDC".to_string(),
            "BONK".to_string(),
        ])
    }
}
