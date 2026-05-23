use anyhow::{Context, Result};
use solana_sdk::signer::keypair::Keypair;
use std::fs;
use std::path::PathBuf;

/// Default wallet directory: ~/.config/tx-terminal/
fn wallet_dir() -> PathBuf {
    let home = dirs::home_dir().unwrap_or_else(|| PathBuf::from("."));
    home.join(".config").join("tx-terminal")
}

/// Default wallet file path: ~/.config/tx-terminal/wallet.json
pub fn wallet_path() -> PathBuf {
    wallet_dir().join("wallet.json")
}

/// Check if a saved wallet exists on disk.
pub fn wallet_exists() -> bool {
    wallet_path().exists()
}

/// Load an existing wallet keypair from disk.
/// The file format is the standard Solana CLI JSON format (byte array).
pub fn load_wallet() -> Result<Keypair> {
    let path = wallet_path();
    let data = fs::read_to_string(&path)
        .with_context(|| format!("Failed to read wallet file: {}", path.display()))?;
    let bytes: Vec<u8> = serde_json::from_str(&data)
        .with_context(|| "Failed to parse wallet JSON (expected byte array)")?;
    Keypair::from_bytes(&bytes).map_err(|e| anyhow::anyhow!("Invalid keypair bytes: {}", e))
}

/// Generate a new Solana keypair, save it to disk, and return it.
/// Creates the config directory if it doesn't exist.
pub fn create_wallet() -> Result<Keypair> {
    let dir = wallet_dir();
    fs::create_dir_all(&dir)
        .with_context(|| format!("Failed to create wallet directory: {}", dir.display()))?;

    let keypair = Keypair::new();
    let bytes = keypair.to_bytes();
    let json =
        serde_json::to_string(&bytes.to_vec()).with_context(|| "Failed to serialize keypair")?;

    let path = wallet_path();
    fs::write(&path, &json)
        .with_context(|| format!("Failed to write wallet file: {}", path.display()))?;

    Ok(keypair)
}

/// Load existing wallet or create a new one.
/// Returns (keypair, was_newly_created).
pub fn load_or_create_wallet() -> Result<(Keypair, bool)> {
    if wallet_exists() {
        let kp = load_wallet()?;
        Ok((kp, false))
    } else {
        let kp = create_wallet()?;
        Ok((kp, true))
    }
}

/// Get the short display string for a pubkey: "ABC1...XYZ9"
pub fn short_pubkey(pubkey: &solana_sdk::pubkey::Pubkey) -> String {
    let s = pubkey.to_string();
    if s.len() > 8 {
        format!("{}...{}", &s[..4], &s[s.len() - 4..])
    } else {
        s
    }
}

/// Export the private key as a base58 string (for importing into Phantom etc.)
pub fn export_private_key_bs58(keypair: &Keypair) -> String {
    bs58::encode(keypair.to_bytes()).into_string()
}
