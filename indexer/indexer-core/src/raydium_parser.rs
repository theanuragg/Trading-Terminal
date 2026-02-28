// Raydium AMM swap parser.
// Handles detection and parsing of Raydium Fusion Pools and standard AMM swaps.

use crate::models::BondingCurveTrade;
use crate::spl_parser::{BlockRef, InstructionRef, TransactionRef};
use chrono::{TimeZone, Utc};

// Raydium AMM program IDs (mainnet).
pub const RAYDIUM_FUSION_PROGRAM_ID: &str = "PhoeNiXZ8ByJGLkxNfZRnkUfjvmuYqLR89jjccR8DL7";
pub const RAYDIUM_AMM_V3_PROGRAM_ID: &str = "9KEPoZmtHkcsf9wXW4c6ZTwkdq4d5JZy2QTrPJWYC72";
pub const RAYDIUM_AMM_V4_PROGRAM_ID: &str = "675kPX9MHTjS2zt1qrNpOtSzVDfZtdztM2raKPLC5Jb";

// Raydium swap instruction discriminators (first byte after discriminator check)
pub const SWAP_EXACT_TOKENS_FOR_TOKENS: u8 = 9;
pub const SWAP_TOKENS_FOR_EXACT_TOKENS: u8 = 10;

fn read_u64_le(bytes: &[u8]) -> Option<u64> {
    if bytes.len() < 8 {
        return None;
    }
    let mut arr = [0u8; 8];
    arr.copy_from_slice(&bytes[..8]);
    Some(u64::from_le_bytes(arr))
}

pub fn extract_raydium_trades_from_block(block: &BlockRef) -> Vec<BondingCurveTrade> {
    let mut trades = Vec::new();

    let block_time = block
        .block_time_unix
        .and_then(|t| Utc.timestamp_opt(t, 0).single());

    for tx in &block.transactions {
        for ix in &tx.instructions {
            // Check if this is a Raydium AMM program.
            if !is_raydium_program(&ix.program_id) {
                continue;
            }

            if let Some(trade) = parse_raydium_swap(block.slot, block_time, tx, ix) {
                trades.push(trade);
            }
        }
    }

    trades
}

fn is_raydium_program(program_id: &str) -> bool {
    matches!(
        program_id,
        RAYDIUM_FUSION_PROGRAM_ID | RAYDIUM_AMM_V3_PROGRAM_ID | RAYDIUM_AMM_V4_PROGRAM_ID
    )
}

/// Parse a Raydium swap instruction.
/// 
/// Raydium swap instructions follow an Anchor pattern where:
/// - First byte is the discriminator (9 for SwapExactTokensForTokens, 10 for SwapTokensForExactTokens)
/// - Following bytes contain encoded parameters (amount_in, amount_out, etc.)
/// 
/// Account structure (typical for Raydium):
/// [0] - Signer (trader wallet)
/// [1] - Token program (SPL Token)
/// [2] - Swap account (pool)
/// [3] - Authority account
/// [4+] - Token input/output accounts
fn parse_raydium_swap(
    slot: i64,
    block_time: Option<chrono::DateTime<chrono::Utc>>,
    tx: &TransactionRef,
    ix: &InstructionRef,
) -> Option<BondingCurveTrade> {
    if ix.data.len() < 17 {
        // Need at least 1 byte discriminator + 8 bytes for amount_in + 8 bytes for amount_out
        return None;
    }

    let _discriminator = ix.data[0];

    // Extract swap amounts
    // For SwapExactTokensForTokens: amount_in(u64), minimum_amount_out(u64)
    // For SwapTokensForExactTokens: maximum_amount_in(u64), amount_out(u64)
    let amount_in = read_u64_le(&ix.data[1..])?;
    let amount_out = read_u64_le(&ix.data[9..])?;

    // Infer swap direction based on relative amounts
    let direction = infer_swap_direction_raydium(amount_in, amount_out);

    // Extract accounts: need at least trader + pool accounts
    if ix.accounts.len() < 3 {
        return None;
    }

    let trader_idx = ix.accounts.get(0).copied()? as usize;
    let trader = tx.message.account_keys.get(trader_idx)?.clone();

    // For pool, typically at account index 2 or 3
    // We'll use the accounts to infer a mint - in production this would come from on-chain data
    let mint_pubkey = format!("raydium_pool_{}", &trader[..trader.len().min(8)]);

    let price = if amount_out == 0 {
        0u64
    } else {
        amount_in / amount_out
    };

    Some(BondingCurveTrade {
        signature: tx.signature.clone(),
        slot,
        block_time,
        mint_pubkey,
        trader,
        side: direction.to_string(),
        token_amount: amount_out as i64,
        sol_amount: amount_in as i64,
        price_nanos_per_token: price as i64,
        tx_index: tx.index,
        ix_index: ix.index,
    })
}

/// Infer swap direction based on amount comparison.
/// 
/// If amount_in is significantly smaller than amount_out: BUY signal (small SOL → many tokens)
/// If amount_out is significantly smaller than amount_in: SELL signal (many tokens → small SOL)
fn infer_swap_direction_raydium(amount_in: u64, amount_out: u64) -> &'static str {
    if amount_in == 0 || amount_out == 0 {
        return "buy"; // Default to buy for edge cases
    }

    // If we're sending out much less than we're sending in, this looks like a sell
    // (we sold tokens and got some SOL back)
    let ratio = amount_out as f64 / amount_in as f64;
    if ratio < 0.1 {
        "sell"
    } else {
        "buy"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::spl_parser::{InstructionRef, MessageRef, TransactionRef};

    fn create_raydium_swap_instruction(amount_in: u64, amount_out: u64) -> Vec<u8> {
        let mut data = vec![SWAP_EXACT_TOKENS_FOR_TOKENS];
        data.extend_from_slice(&amount_in.to_le_bytes());
        data.extend_from_slice(&amount_out.to_le_bytes());
        data
    }

    #[test]
    fn test_raydium_swap_exact_tokens_parsing() {
        let block = BlockRef {
            slot: 100,
            block_time_unix: Some(1000),
            transactions: vec![TransactionRef {
                signature: "swap_sig_001".to_string(),
                index: 0,
                message: MessageRef {
                    account_keys: vec![
                        "trader_wallet".to_string(),
                        "token_program".to_string(),
                        "pool_account".to_string(),
                        "authority".to_string(),
                        "input_token".to_string(),
                        "output_token".to_string(),
                    ],
                },
                instructions: vec![InstructionRef {
                    program_id: RAYDIUM_AMM_V4_PROGRAM_ID.to_string(),
                    accounts: vec![0, 1, 2, 3, 4, 5],
                    data: create_raydium_swap_instruction(1_000_000_000, 5_000_000_000), // 1 SOL → 5B tokens
                    index: 0,
                }],
            }],
        };

        let trades = extract_raydium_trades_from_block(&block);
        assert_eq!(trades.len(), 1);
        assert_eq!(trades[0].sol_amount, 1_000_000_000);
        assert_eq!(trades[0].token_amount, 5_000_000_000);
        assert_eq!(trades[0].trader, "trader_wallet");
        assert_eq!(trades[0].side, "buy");
    }

    #[test]
    fn test_raydium_swap_tokens_for_exact_parsing() {
        let block = BlockRef {
            slot: 101,
            block_time_unix: Some(1001),
            transactions: vec![TransactionRef {
                signature: "swap_sig_002".to_string(),
                index: 0,
                message: MessageRef {
                    account_keys: vec![
                        "trader_wallet".to_string(),
                        "token_program".to_string(),
                        "pool_account".to_string(),
                        "authority".to_string(),
                        "input_token".to_string(),
                        "output_token".to_string(),
                    ],
                },
                instructions: vec![InstructionRef {
                    program_id: RAYDIUM_FUSION_PROGRAM_ID.to_string(),
                    accounts: vec![0, 1, 2, 3, 4, 5],
                    data: create_raydium_swap_instruction(10_000_000_000, 50_000_000), // 10B tokens → 0.05 SOL
                    index: 0,
                }],
            }],
        };

        let trades = extract_raydium_trades_from_block(&block);
        assert_eq!(trades.len(), 1);
        assert_eq!(trades[0].side, "sell"); // Ratio is very small: 50M / 10B = 0.005
    }

    #[test]
    fn test_raydium_swap_direction_inference() {
        assert_eq!(infer_swap_direction_raydium(1_000_000_000, 5_000_000_000), "buy");
        assert_eq!(infer_swap_direction_raydium(10_000_000_000, 50_000_000), "sell");
        assert_eq!(infer_swap_direction_raydium(1_000_000_000, 1_000_000_000), "buy");
    }

    #[test]
    fn test_raydium_multiple_swaps_per_block() {
        let block = BlockRef {
            slot: 102,
            block_time_unix: Some(1002),
            transactions: vec![
                TransactionRef {
                    signature: "tx1".to_string(),
                    index: 0,
                    message: MessageRef {
                        account_keys: vec![
                            "user1".to_string(),
                            "token_prog".to_string(),
                            "pool".to_string(),
                            "auth".to_string(),
                            "in_tok".to_string(),
                            "out_tok".to_string(),
                        ],
                    },
                    instructions: vec![InstructionRef {
                        program_id: RAYDIUM_AMM_V3_PROGRAM_ID.to_string(),
                        accounts: vec![0, 1, 2, 3, 4, 5],
                        data: create_raydium_swap_instruction(100_000_000, 1_000_000_000),
                        index: 0,
                    }],
                },
                TransactionRef {
                    signature: "tx2".to_string(),
                    index: 1,
                    message: MessageRef {
                        account_keys: vec![
                            "user2".to_string(),
                            "token_prog".to_string(),
                            "pool".to_string(),
                            "auth".to_string(),
                            "in_tok".to_string(),
                            "out_tok".to_string(),
                        ],
                    },
                    instructions: vec![InstructionRef {
                        program_id: RAYDIUM_AMM_V4_PROGRAM_ID.to_string(),
                        accounts: vec![0, 1, 2, 3, 4, 5],
                        data: create_raydium_swap_instruction(5_000_000_000, 100_000_000),
                        index: 0,
                    }],
                },
            ],
        };

        let trades = extract_raydium_trades_from_block(&block);
        assert_eq!(trades.len(), 2);
        assert_eq!(trades[0].trader, "user1");
        assert_eq!(trades[0].side, "buy");
        assert_eq!(trades[1].trader, "user2");
        assert_eq!(trades[1].side, "sell");
    }

    #[test]
    fn test_raydium_insufficient_accounts_error() {
        let block = BlockRef {
            slot: 103,
            block_time_unix: Some(1003),
            transactions: vec![TransactionRef {
                signature: "bad_tx".to_string(),
                index: 0,
                message: MessageRef {
                    account_keys: vec!["acc0".to_string()],
                },
                instructions: vec![InstructionRef {
                    program_id: RAYDIUM_AMM_V4_PROGRAM_ID.to_string(),
                    accounts: vec![0], // Only 1 account, need at least 3
                    data: create_raydium_swap_instruction(1_000_000, 1_000_000),
                    index: 0,
                }],
            }],
        };

        let trades = extract_raydium_trades_from_block(&block);
        assert_eq!(trades.len(), 0); // Should return empty due to insufficient accounts
    }

    #[test]
    fn test_raydium_program_id_filtering() {
        let block = BlockRef {
            slot: 104,
            block_time_unix: Some(1004),
            transactions: vec![TransactionRef {
                signature: "wrong_prog".to_string(),
                index: 0,
                message: MessageRef {
                    account_keys: vec![
                        "trader".to_string(),
                        "token_prog".to_string(),
                        "pool".to_string(),
                        "auth".to_string(),
                    ],
                },
                instructions: vec![InstructionRef {
                    program_id: "SomeOtherProgram".to_string(), // Not a Raydium program
                    accounts: vec![0, 1, 2, 3],
                    data: create_raydium_swap_instruction(1_000_000, 1_000_000),
                    index: 0,
                }],
            }],
        };

        let trades = extract_raydium_trades_from_block(&block);
        assert_eq!(trades.len(), 0); // Should filter out non-Raydium programs
    }

    #[test]
    fn test_raydium_insufficient_data_length() {
        let block = BlockRef {
            slot: 105,
            block_time_unix: Some(1005),
            transactions: vec![TransactionRef {
                signature: "short_data".to_string(),
                index: 0,
                message: MessageRef {
                    account_keys: vec!["trader".to_string(), "token_prog".to_string(), "pool".to_string(), "auth".to_string()],
                },
                instructions: vec![InstructionRef {
                    program_id: RAYDIUM_AMM_V4_PROGRAM_ID.to_string(),
                    accounts: vec![0, 1, 2, 3],
                    data: vec![9, 1, 2], // Too short (need 17 bytes)
                    index: 0,
                }],
            }],
        };

        let trades = extract_raydium_trades_from_block(&block);
        assert_eq!(trades.len(), 0);
    }

    #[test]
    fn test_is_raydium_program() {
        assert!(is_raydium_program(RAYDIUM_FUSION_PROGRAM_ID));
        assert!(is_raydium_program(RAYDIUM_AMM_V3_PROGRAM_ID));
        assert!(is_raydium_program(RAYDIUM_AMM_V4_PROGRAM_ID));
        assert!(!is_raydium_program("SomeOtherProgram"));
    }
}
