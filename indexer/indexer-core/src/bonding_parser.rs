use crate::{
    models::BondingCurveTrade,
    spl_parser::{BlockRef, InstructionRef, TransactionRef},
};
use chrono::{TimeZone, Utc};
use sha2::{Digest, Sha256};

pub const PUMP_PROGRAM_ID: &str = "6EF8rrecthR5Dkzon8Nwu78hRvfCKubJ14M5uBEwF6P";

fn anchor_discriminator(ix_name: &str) -> [u8; 8] {
    let preimage = format!("global:{ix_name}");
    let hash = Sha256::digest(preimage.as_bytes());
    let mut out = [0u8; 8];
    out.copy_from_slice(&hash[..8]);
    out
}

fn read_u64_le(bytes: &[u8]) -> Option<u64> {
    if bytes.len() < 8 {
        return None;
    }
    let mut arr = [0u8; 8];
    arr.copy_from_slice(&bytes[..8]);
    Some(u64::from_le_bytes(arr))
}

pub fn extract_pump_trades_from_block(block: &BlockRef) -> Vec<BondingCurveTrade> {
    let buy_disc = anchor_discriminator("buy");
    let sell_disc = anchor_discriminator("sell");

    let block_time = block
        .block_time_unix
        .and_then(|t| Utc.timestamp_opt(t, 0).single());

    let mut out = Vec::new();

    for tx in &block.transactions {
        for ix in &tx.instructions {
            if ix.program_id != PUMP_PROGRAM_ID {
                continue;
            }

            if ix.data.len() < 8 {
                continue;
            }
            let disc: [u8; 8] = ix.data[0..8].try_into().unwrap();

            if disc == buy_disc {
                if let Some(trade) = parse_buy(block.slot, block_time, tx, ix) {
                    out.push(trade);
                }
            } else if disc == sell_disc {
                if let Some(trade) = parse_sell(block.slot, block_time, tx, ix) {
                    out.push(trade);
                }
            }
        }
    }

    out
}

// Pump IDL (public): buy accounts order includes mint at index 2 and user at index 6.
// Source: pump.fun IDL JSON (see public gist in research).
fn parse_buy(
    slot: i64,
    block_time: Option<chrono::DateTime<chrono::Utc>>,
    tx: &TransactionRef,
    ix: &InstructionRef,
) -> Option<BondingCurveTrade> {
    let (mint, user) = pump_mint_and_user(tx, ix)?;

    // args: amount(u64), maxSolCost(u64)
    let args = &ix.data[8..];
    let token_amount = read_u64_le(args)?;
    let max_sol_cost = read_u64_le(&args[8..])?;

    let price = if token_amount == 0 {
        0u64
    } else {
        max_sol_cost / token_amount
    };

    Some(BondingCurveTrade {
        signature: tx.signature.clone(),
        slot,
        block_time,
        mint_pubkey: mint,
        trader: user,
        side: "buy".to_string(),
        token_amount: token_amount as i64,
        sol_amount: max_sol_cost as i64,
        price_nanos_per_token: price as i64,
        tx_index: tx.index,
        ix_index: ix.index,
    })
}

// Pump IDL: sell accounts include mint at index 2 and user at index 6.
// args: amount(u64), minSolOutput(u64)
fn parse_sell(
    slot: i64,
    block_time: Option<chrono::DateTime<chrono::Utc>>,
    tx: &TransactionRef,
    ix: &InstructionRef,
) -> Option<BondingCurveTrade> {
    let (mint, user) = pump_mint_and_user(tx, ix)?;

    let args = &ix.data[8..];
    let token_amount = read_u64_le(args)?;
    let min_sol_output = read_u64_le(&args[8..])?;

    let price = if token_amount == 0 {
        0u64
    } else {
        min_sol_output / token_amount
    };

    Some(BondingCurveTrade {
        signature: tx.signature.clone(),
        slot,
        block_time,
        mint_pubkey: mint,
        trader: user,
        side: "sell".to_string(),
        token_amount: token_amount as i64,
        sol_amount: min_sol_output as i64,
        price_nanos_per_token: price as i64,
        tx_index: tx.index,
        ix_index: ix.index,
    })
}

fn pump_mint_and_user(tx: &TransactionRef, ix: &InstructionRef) -> Option<(String, String)> {
    // `ix.accounts` are indices into message.account_keys.
    // Pump IDL: mint at accounts[2], user at accounts[6].
    if ix.accounts.len() < 7 {
        return None;
    }
    let mint_idx = ix.accounts.get(2).copied()? as usize;
    let user_idx = ix.accounts.get(6).copied()? as usize;

    let mint = tx.message.account_keys.get(mint_idx)?.clone();
    let user = tx.message.account_keys.get(user_idx)?.clone();

    Some((mint, user))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::spl_parser::{BlockRef, TransactionRef, MessageRef, InstructionRef};

    #[test]
    fn test_anchor_discriminator() {
        let buy_disc = anchor_discriminator("buy");
        let sell_disc = anchor_discriminator("sell");
        
        // Discriminators should be 8 bytes
        assert_eq!(buy_disc.len(), 8);
        assert_eq!(sell_disc.len(), 8);
        
        // They should be different
        assert_ne!(buy_disc, sell_disc);
    }

    fn create_buy_instruction() -> Vec<u8> {
        let buy_disc = anchor_discriminator("buy");
        let mut data = buy_disc.to_vec();
        // amount: 1_000_000 (1M tokens)
        data.extend_from_slice(&(1_000_000u64).to_le_bytes());
        // maxSolCost: 100_000_000 (0.1 SOL = 100M lamports)
        data.extend_from_slice(&(100_000_000u64).to_le_bytes());
        data
    }

    fn create_sell_instruction() -> Vec<u8> {
        let sell_disc = anchor_discriminator("sell");
        let mut data = sell_disc.to_vec();
        // amount: 500_000 (500k tokens)
        data.extend_from_slice(&(500_000u64).to_le_bytes());
        // minSolOutput: 50_000_000 (0.05 SOL = 50M lamports)
        data.extend_from_slice(&(50_000_000u64).to_le_bytes());
        data
    }

    #[test]
    fn test_parse_pump_buy() {
        let block = BlockRef {
            slot: 100,
            block_time_unix: Some(1000),
            transactions: vec![TransactionRef {
                signature: "buy_sig_123".to_string(),
                index: 0,
                message: MessageRef {
                    account_keys: vec![
                        "acc0".to_string(),
                        "acc1".to_string(),
                        "mint_abc".to_string(),     // index 2 (mint)
                        "acc3".to_string(),
                        "acc4".to_string(),
                        "acc5".to_string(),
                        "trader_wallet".to_string(), // index 6 (user)
                    ],
                },
                instructions: vec![InstructionRef {
                    program_id: PUMP_PROGRAM_ID.to_string(),
                    accounts: vec![0, 1, 2, 3, 4, 5, 6],
                    data: create_buy_instruction(),
                    index: 0,
                }],
            }],
        };

        let trades = extract_pump_trades_from_block(&block);

        assert_eq!(trades.len(), 1);
        assert_eq!(trades[0].mint_pubkey, "mint_abc");
        assert_eq!(trades[0].trader, "trader_wallet");
        assert_eq!(trades[0].side, "buy");
        assert_eq!(trades[0].token_amount, 1_000_000);
        assert_eq!(trades[0].sol_amount, 100_000_000);
        // price = maxSolCost / token_amount = 100_000_000 / 1_000_000 = 100
        assert_eq!(trades[0].price_nanos_per_token, 100);
        assert_eq!(trades[0].slot, 100);
    }

    #[test]
    fn test_parse_pump_sell() {
        let block = BlockRef {
            slot: 200,
            block_time_unix: Some(2000),
            transactions: vec![TransactionRef {
                signature: "sell_sig_456".to_string(),
                index: 1,
                message: MessageRef {
                    account_keys: vec![
                        "acc0".to_string(),
                        "acc1".to_string(),
                        "mint_xyz".to_string(),      // index 2 (mint)
                        "acc3".to_string(),
                        "acc4".to_string(),
                        "acc5".to_string(),
                        "trader_wallet2".to_string(), // index 6 (user)
                    ],
                },
                instructions: vec![InstructionRef {
                    program_id: PUMP_PROGRAM_ID.to_string(),
                    accounts: vec![0, 1, 2, 3, 4, 5, 6],
                    data: create_sell_instruction(),
                    index: 0,
                }],
            }],
        };

        let trades = extract_pump_trades_from_block(&block);

        assert_eq!(trades.len(), 1);
        assert_eq!(trades[0].mint_pubkey, "mint_xyz");
        assert_eq!(trades[0].trader, "trader_wallet2");
        assert_eq!(trades[0].side, "sell");
        assert_eq!(trades[0].token_amount, 500_000);
        assert_eq!(trades[0].sol_amount, 50_000_000);
        // price = minSolOutput / token_amount = 50_000_000 / 500_000 = 100
        assert_eq!(trades[0].price_nanos_per_token, 100);
    }

    #[test]
    fn test_parse_multiple_pump_trades_in_block() {
        let block = BlockRef {
            slot: 300,
            block_time_unix: Some(3000),
            transactions: vec![
                TransactionRef {
                    signature: "tx1".to_string(),
                    index: 0,
                    message: MessageRef {
                        account_keys: vec![
                            "a".to_string(),
                            "b".to_string(),
                            "mint1".to_string(),
                            "c".to_string(),
                            "d".to_string(),
                            "e".to_string(),
                            "user1".to_string(),
                        ],
                    },
                    instructions: vec![InstructionRef {
                        program_id: PUMP_PROGRAM_ID.to_string(),
                        accounts: vec![0, 1, 2, 3, 4, 5, 6],
                        data: create_buy_instruction(),
                        index: 0,
                    }],
                },
                TransactionRef {
                    signature: "tx2".to_string(),
                    index: 1,
                    message: MessageRef {
                        account_keys: vec![
                            "a".to_string(),
                            "b".to_string(),
                            "mint1".to_string(),
                            "c".to_string(),
                            "d".to_string(),
                            "e".to_string(),
                            "user2".to_string(),
                        ],
                    },
                    instructions: vec![InstructionRef {
                        program_id: PUMP_PROGRAM_ID.to_string(),
                        accounts: vec![0, 1, 2, 3, 4, 5, 6],
                        data: create_sell_instruction(),
                        index: 0,
                    }],
                },
            ],
        };

        let trades = extract_pump_trades_from_block(&block);

        assert_eq!(trades.len(), 2);
        assert_eq!(trades[0].side, "buy");
        assert_eq!(trades[0].trader, "user1");
        assert_eq!(trades[1].side, "sell");
        assert_eq!(trades[1].trader, "user2");
    }

    #[test]
    fn test_pump_mint_and_user_extraction() {
        let tx = TransactionRef {
            signature: "test".to_string(),
            index: 0,
            message: MessageRef {
                account_keys: vec![
                    "a0".to_string(),
                    "a1".to_string(),
                    "the_mint".to_string(),
                    "a3".to_string(),
                    "a4".to_string(),
                    "a5".to_string(),
                    "the_user".to_string(),
                ],
            },
            instructions: vec![],
        };

        let ix = InstructionRef {
            program_id: PUMP_PROGRAM_ID.to_string(),
            accounts: vec![0, 1, 2, 3, 4, 5, 6],
            data: vec![],
            index: 0,
        };

        let (mint, user) = pump_mint_and_user(&tx, &ix).expect("should extract");
        assert_eq!(mint, "the_mint");
        assert_eq!(user, "the_user");
    }

    #[test]
    fn test_pump_mint_and_user_insufficient_accounts() {
        let tx = TransactionRef {
            signature: "test".to_string(),
            index: 0,
            message: MessageRef {
                account_keys: vec!["a0".to_string(), "a1".to_string()],
            },
            instructions: vec![],
        };

        let ix = InstructionRef {
            program_id: PUMP_PROGRAM_ID.to_string(),
            accounts: vec![0, 1], // Only 2 accounts, need 7
            data: vec![],
            index: 0,
        };

        let result = pump_mint_and_user(&tx, &ix);
        assert!(result.is_none());
    }

    #[test]
    fn test_pump_zero_token_amount() {
        let block = BlockRef {
            slot: 400,
            block_time_unix: Some(4000),
            transactions: vec![TransactionRef {
                signature: "zero_tx".to_string(),
                index: 0,
                message: MessageRef {
                    account_keys: vec![
                        "a".to_string(),
                        "b".to_string(),
                        "mint".to_string(),
                        "c".to_string(),
                        "d".to_string(),
                        "e".to_string(),
                        "user".to_string(),
                    ],
                },
                instructions: vec![InstructionRef {
                    program_id: PUMP_PROGRAM_ID.to_string(),
                    accounts: vec![0, 1, 2, 3, 4, 5, 6],
                    data: {
                        let disc = anchor_discriminator("buy");
                        let mut d = disc.to_vec();
                        d.extend_from_slice(&(0u64).to_le_bytes()); // zero amount
                        d.extend_from_slice(&(100u64).to_le_bytes());
                        d
                    },
                    index: 0,
                }],
            }],
        };

        let trades = extract_pump_trades_from_block(&block);

        assert_eq!(trades.len(), 1);
        // price should be 0 when token_amount is 0
        assert_eq!(trades[0].price_nanos_per_token, 0);
    }
}