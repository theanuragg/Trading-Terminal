 use anyhow::Result;
 use serde::Deserialize;

 #[derive(Debug, Deserialize, Clone)]
 pub struct RuntimeConfig {
     pub environment: String,
 }

 #[derive(Debug, Deserialize, Clone)]
 pub struct ApiConfig {
     pub bind_addr: String,
 }

 #[derive(Debug, Deserialize, Clone)]
 pub struct DbConfig {
     pub url: String,
     pub max_connections: u32,
 }

 #[derive(Debug, Deserialize, Clone)]
 pub struct FirehoseConfig {
     pub endpoint: String,
     pub from_slot: Option<i64>,
     pub mint_whitelist: Vec<String>,
     #[serde(default)]
     pub initial_backoff_ms: Option<u64>,
     #[serde(default)]
     pub max_backoff_ms: Option<u64>,
 }

 #[derive(Debug, Deserialize, Clone)]
 pub struct RedisConfig {
     pub host: String,
     pub port: u16,
     pub db: u8,
     pub password: String,
     pub stream_key_prefix: String,
     pub max_stream_len: u64,
 }

 #[derive(Debug, Deserialize, Clone)]
 pub struct IndexerConfig {
     pub runtime: RuntimeConfig,
     pub api: ApiConfig,
     pub db: DbConfig,
     pub firehose: FirehoseConfig,
     #[serde(default)]
     pub redis: Option<RedisConfig>,
 }

 impl IndexerConfig {
     pub fn from_env() -> Result<Self> {
        // Load base config from `indexer/config/default.(toml|yaml|json)` relative to the
        // current working directory (the `indexer/` workspace root), then override with
        // `INDEXER__...` environment variables.
        let settings = config::Config::builder()
            .add_source(config::File::with_name("config/default").required(false))
            .add_source(config::Environment::with_prefix("INDEXER").separator("__"))
            .build()?;

        settings.try_deserialize().map_err(Into::into)
     }
 }

