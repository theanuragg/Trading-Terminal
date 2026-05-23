use clap::Parser;
use dotenv::dotenv;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
pub struct Args {
    /// RPC URL for Solana connection
    #[arg(
        short,
        long,
        env = "RPC_URL",
        default_value = "https://api.devnet.solana.com"
    )]
    pub rpc_url: String,

    /// Keypair file path
    #[arg(short, long, env = "KEYPAIR_PATH")]
    pub keypair_path: Option<String>,

    /// Indexer API URL for chart, transactions, and holder data
    #[arg(
        long,
        env = "INDEXER_API_URL",
        default_value = "http://localhost:8080"
    )]
    pub indexer_api_url: String,

    /// UI theme: light, dark, or bloomberg
    #[arg(long, env = "THEME", default_value = "bloomberg")]
    pub theme: String,
}

pub fn load_config() -> Args {
    dotenv().ok();
    let mut args = Args::parse();
    // normalize theme string
    args.theme = args.theme.to_lowercase();
    args
}
