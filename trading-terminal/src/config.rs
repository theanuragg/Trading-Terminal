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
}

pub fn load_config() -> Args {
    dotenv().ok();
    Args::parse()
}
