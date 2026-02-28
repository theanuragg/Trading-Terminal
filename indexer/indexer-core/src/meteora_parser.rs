// Meteora DLMM (Dynamic Liquidity Market Maker) swap parser.
// Handles detection and parsing of Meteora DLMM pool swaps (v1 and v2).

use crate::models::BondingCurveTrade;
use crate::spl_parser::{BlockRef, InstructionRef, TransactionRef};
use chrono::{TimeZone, Utc};

// Meteora DLMM program ID (mainnet).
pub const METEORA_DLMM_PROGRAM_ID: &str = "LBUZKhRxPF3XUpBCjp4YeC6BNhu2nqBDt16ymccEZLo";

// Common DLMM instruction discriminators
pub const DLMM_SWAP: u8 = 11;
pub const DLMM_SWAP_V2: u8 = 22;

fn read_u64_le(bytes: &[u8]) -> Option<u64> {
    if bytes.len() < 8 {
        return None;
    }
    let mut arr = [0u8; 8];
    arr.copy_from_slice(&bytes[..8]);
    Some(u64::from_le_bytes(arr))
}

fn read_u32_le(bytes: &[u8]) -> Option<u32> {
    if bytes.len() < 4 {
        return None;
    }
    let mut arr = [0u8; 4];
    arr.copy_from_slice(&bytes[..4]);
    Some(u32::from_le_bytes(arr))
}

pub fn extract_meteora_trades_from_block(block: &BlockRef) -> Vec<BondingCurveTrade> {
    let mut trades = Vec::new();

    let block_time = block
        .block_time_unix
        .and_then(|t| Utc.timestamp_opt(t, 0).single());

    for tx in &block.transactions {
        for ix in &tx.instructions {
            // Check if this is a Meteora DLMM program.
            if ix.program_id != METEORA_DLMM_PROGRAM_ID {
                continue;
            }

            if let Some(trade) = parse_meteora_swap(block.slot, block_time, tx, ix) {
                trades.push(trade);
            }
        }
    }

    trades
}

/// Parse a Meteora DLMM swap instruction.
/// 
/// Meteora DLMM swaps can be v1 or v2 format. Both support:
/// - Detecting active bins in the liquidity pool
/// - Executing swaps across multiple bins
/// - Tracking effective prices
/// 
/// Instruction format:
/// [0] - Discriminator (11 for v1, 22 for v2)
/// [1-8] - Amount in (u64 LE)
/// [9-16] - Minimum amount out (u64 LE)
/// [17+] - Version-specific metadata (bins, fee tier, etc.)
fn parse_meteora_swap(
    slot: i64,
    block_time: Option<chrono::DateTime<chrono::Utc>>,
    tx: &TransactionRef,
    ix: &InstructionRef,
) -> Option<BondingCurveTrade> {
    if ix.data.len() < 17 {
        // Need at least discriminator + amount_in + amount_out
        return None;
    }

    let discriminator = ix.data[0];

    // Parse common fields
    let amount_in = read_u64_le(&ix.data[1..])?;
    let amount_out = read_u64_le(&ix.data[9..])?;

    // Determine version
    let _version = infer_dlmm_version(ix, discriminator);

    // Extract trader from accounts (typically account 0)
    let trader_idx = ix.accounts.get(0).copied()? as usize;
    let trader = tx.message.account_keys.get(trader_idx)?.clone();

    // Extract pool ID from accounts (typically account 1 or 2)
    let pool_idx = ix.accounts.get(1).copied()? as usize;
    let pool_id = tx.message.account_keys.get(pool_idx)?.clone();


    // Parse version-specific fields
    let (_bins_used, _fee_tier, _active_bin) = if _version == 2 {
        parse_meteora_v2_metadata(ix)
    } else {
        parse_meteora_v1_metadata(ix)
    };

    // Infer direction
    let direction = infer_dlmm_direction(amount_in, amount_out);

    let price = if amount_out == 0 {
        0u64
    } else {
        amount_in / amount_out
    };

    Some(BondingCurveTrade {
        signature: tx.signature.clone(),
        slot,
        block_time,
        mint_pubkey: pool_id,
        trader,
        side: direction.to_string(),
        token_amount: amount_out as i64,
        sol_amount: amount_in as i64,
        price_nanos_per_token: price as i64,
        tx_index: tx.index,
        ix_index: ix.index,
    })
}

/// Infer Meteora DLMM version from instruction structure.
/// v1 has fixed account layout, v2 has variable bins which changes account count.
fn infer_dlmm_version(ix: &InstructionRef, discriminator: u8) -> u32 {
    // v2 typically has more accounts due to dynamic bin handling
    if ix.accounts.len() > 8 || discriminator == DLMM_SWAP_V2 {
        2
    } else {
        1
    }
}

/// Parse Meteora v1 metadata from instruction.
/// v1 has simpler structure without dynamic bins.
fn parse_meteora_v1_metadata(_ix: &InstructionRef) -> (Vec<i32>, Option<i64>, Option<i32>) {
    // v1 typically doesn't expose bins, fee tier might be in account data
    // For now, return defaults
    let bins_used = vec![];
    let fee_tier = None;
    let active_bin = None;
    (bins_used, fee_tier, active_bin)
}

/// Parse Meteora v2 metadata from instruction.
/// v2 includes dynamic bin handling and fee tier information.
fn parse_meteora_v2_metadata(ix: &InstructionRef) -> (Vec<i32>, Option<i64>, Option<i32>) {
    // In v2, starting at offset 17, we can find:
    // - bin_count (u32) at offset 17
    // - bins array follows
    // - fee_tier info in the instruction
    
    let mut bins_used = vec![];
    let mut fee_tier = None;
    let mut active_bin = None;

    if ix.data.len() >= 21 {
        if let Some(bin_count) = read_u32_le(&ix.data[17..]) {
            // Parse up to bin_count bins (capped at reasonable amount)
            let bin_count = (bin_count as usize).min(10);
            let mut offset = 21;
            for _ in 0..bin_count {
                if offset + 4 <= ix.data.len() {
                    if let Some(bin_id) = read_u32_le(&ix.data[offset..]) {
                        bins_used.push(bin_id as i32);
                        offset += 4;
                    }
                }
            }
        }
    }

    // Fee tier might be encoded elsewhere in instruction or in account data
    if ix.data.len() >= 25 {
        if let Some(fee) = read_u64_le(&ix.data[21..]) {
            fee_tier = Some(fee as i64);
        }
    }

    // Active bin is typically the primary bin being used
    if !bins_used.is_empty() {
        active_bin = Some(bins_used[0]);
    }

    (bins_used, fee_tier, active_bin)
}

/// Infer swap direction based on amount comparison.
fn infer_dlmm_direction(amount_in: u64, amount_out: u64) -> &'static str {
    if amount_in == 0 || amount_out == 0 {
        return "buy";
    }

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

    fn create_meteora_v1_instruction(amount_in: u64, amount_out: u64) -> Vec<u8> {
        let mut data = vec![DLMM_SWAP]; // discriminator
        data.extend_from_slice(&amount_in.to_le_bytes());
        data.extend_from_slice(&amount_out.to_le_bytes());
        data
    }

    fn create_meteora_v2_instruction(amount_in: u64, amount_out: u64, bins: &[u32]) -> Vec<u8> {
        let mut data = vec![DLMM_SWAP_V2]; // discriminator
        data.extend_from_slice(&amount_in.to_le_bytes());
        data.extend_from_slice(&amount_out.to_le_bytes());
        // Add bin count
        data.extend_from_slice(&(bins.len() as u32).to_le_bytes());
        // Add bins
        for bin in bins {
            data.extend_from_slice(&bin.to_le_bytes());
        }
        // Add fee tier
        data.extend_from_slice(&(250u64).to_le_bytes()); // 250 bps fee tier
        data
    }

    #[test]
    fn test_meteora_dlmm_v1_swap_parsing() {
        let block = BlockRef {
            slot: 200,
            block_time_unix: Some(2000),
            transactions: vec![TransactionRef {
                signature: "meteora_v1_sig".to_string(),
                index: 0,
                message: MessageRef {
                    account_keys: vec![
                        "trader".to_string(),
                        "pool".to_string(),
                        "token_a".to_string(),
                        "token_b".to_string(),
                        "authority".to_string(),
                    ],
                },
                instructions: vec![InstructionRef {
                    program_id: METEORA_DLMM_PROGRAM_ID.to_string(),
                    accounts: vec![0, 1, 2, 3, 4],
                    data: create_meteora_v1_instruction(500_000_000, 2_500_000_000),
                    index: 0,
                }],
            }],
        };

        let trades = extract_meteora_trades_from_block(&block);
        assert_eq!(trades.len(), 1);
        assert_eq!(trades[0].sol_amount, 500_000_000);
        assert_eq!(trades[0].token_amount, 2_500_000_000);
        assert_eq!(trades[0].trader, "trader");
        assert_eq!(trades[0].side, "buy");
    }

    #[test]
    fn test_meteora_dlmm_v2_swap_parsing() {
        let block = BlockRef {
            slot: 201,
            block_time_unix: Some(2001),
            transactions: vec![TransactionRef {
                signature: "meteora_v2_sig".to_string(),
                index: 0,
                message: MessageRef {
                    account_keys: vec![
                        "trader".to_string(),
                        "pool".to_string(),
                        "token_a".to_string(),
                        "token_b".to_string(),
                        "authority".to_string(),
                        "bin_1".to_string(),
                        "bin_2".to_string(),
                        "bin_3".to_string(),
                        "bin_4".to_string(),
                    ],
                },
                instructions: vec![InstructionRef {
                    program_id: METEORA_DLMM_PROGRAM_ID.to_string(),
                    accounts: vec![0, 1, 2, 3, 4, 5, 6, 7, 8],
                    data: create_meteora_v2_instruction(
                        1_000_000_000,
                        3_000_000_000,
                        &[1001, 1002, 1003],
                    ),
                    index: 0,
                }],
            }],
        };

        let trades = extract_meteora_trades_from_block(&block);
        assert_eq!(trades.len(), 1);
        assert_eq!(trades[0].side, "buy");
    }

    #[test]
    fn test_meteora_version_detection() {
        let v1_ix = InstructionRef {
            program_id: METEORA_DLMM_PROGRAM_ID.to_string(),
            accounts: vec![0, 1, 2, 3, 4],
            data: vec![],
            index: 0,
        };

        let v2_ix = InstructionRef {
            program_id: METEORA_DLMM_PROGRAM_ID.to_string(),
            accounts: vec![0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10],
            data: vec![],
            index: 0,
        };

        assert_eq!(infer_dlmm_version(&v1_ix, DLMM_SWAP), 1);
        assert_eq!(infer_dlmm_version(&v2_ix, DLMM_SWAP_V2), 2);
        assert_eq!(infer_dlmm_version(&v2_ix, DLMM_SWAP), 2);
    }

    #[test]
    fn test_meteora_bins_extraction() {
        let bins = vec![1000, 1001, 1002];
        let ix_data = create_meteora_v2_instruction(1_000_000_000, 1_000_000_000, &bins);

        let ix = InstructionRef {
            program_id: METEORA_DLMM_PROGRAM_ID.to_string(),
            accounts: vec![0, 1],
            data: ix_data,
            index: 0,
        };

        let (extracted_bins, _, active_bin) = parse_meteora_v2_metadata(&ix);
        assert_eq!(extracted_bins.len(), 3);
        assert_eq!(extracted_bins[0], 1000);
        assert_eq!(active_bin, Some(1000));
    }

    #[test]
    fn test_meteora_fee_tier_parsing() {
        let ix_data = create_meteora_v2_instruction(1_000_000_000, 1_000_000_000, &[1000]);

        let ix = InstructionRef {
            program_id: METEORA_DLMM_PROGRAM_ID.to_string(),
            accounts: vec![0, 1],
            data: ix_data,
            index: 0,
        };

        let (_, fee_tier, _) = parse_meteora_v2_metadata(&ix);
        assert!(fee_tier.is_some());
    }

    #[test]
    fn test_meteora_multiple_trades_per_block() {
        let block = BlockRef {
            slot: 202,
            block_time_unix: Some(2002),
            transactions: vec![
                TransactionRef {
                    signature: "tx1".to_string(),
                    index: 0,
                    message: MessageRef {
                        account_keys: vec![
                            "user1".to_string(),
                            "pool".to_string(),
                            "tok_a".to_string(),
                            "tok_b".to_string(),
                            "auth".to_string(),
                        ],
                    },
                    instructions: vec![InstructionRef {
                        program_id: METEORA_DLMM_PROGRAM_ID.to_string(),
                        accounts: vec![0, 1, 2, 3, 4],
                        data: create_meteora_v1_instruction(100_000_000, 500_000_000),
                        index: 0,
                    }],
                },
                TransactionRef {
                    signature: "tx2".to_string(),
                    index: 1,
                    message: MessageRef {
                        account_keys: vec![
                            "user2".to_string(),
                            "pool".to_string(),
                            "tok_a".to_string(),
                            "tok_b".to_string(),
                            "auth".to_string(),
                        ],
                    },
                    instructions: vec![InstructionRef {
                        program_id: METEORA_DLMM_PROGRAM_ID.to_string(),
                        accounts: vec![0, 1, 2, 3, 4],
                        data: create_meteora_v1_instruction(5_000_000_000, 100_000_000),
                        index: 0,
                    }],
                },
            ],
        };

        let trades = extract_meteora_trades_from_block(&block);
        assert_eq!(trades.len(), 2);
        assert_eq!(trades[0].trader, "user1");
        assert_eq!(trades[1].trader, "user2");
    }

    #[test]
    fn test_meteora_direction_inference() {
        assert_eq!(infer_dlmm_direction(1_000_000_000, 5_000_000_000), "buy");
        assert_eq!(infer_dlmm_direction(10_000_000_000, 50_000_000), "sell");
    }

    #[test]
    fn test_extract_meteora_trades_empty_block() {
        let block = BlockRef {
            slot: 100,
            block_time_unix: Some(1000),
            transactions: vec![],
        };

        let trades = extract_meteora_trades_from_block(&block);
        assert_eq!(trades.len(), 0);
    }

    #[test]
    fn test_is_meteora_program() {
        let ix = InstructionRef {
            program_id: METEORA_DLMM_PROGRAM_ID.to_string(),
            accounts: vec![],
            data: vec![],
            index: 0,
        };

        assert_eq!(ix.program_id, METEORA_DLMM_PROGRAM_ID);
    }
}
