use crate::models::TokenTransfer;
use chrono::{TimeZone, Utc};

 /// Placeholder types for Firehose-derived data structures.
 /// In real integration, replace these with jetstreamer_firehose / Solana types.
 #[derive(Debug)]
 pub struct BlockRef {
     pub slot: i64,
     pub block_time_unix: Option<i64>,
     pub transactions: Vec<TransactionRef>,
 }

 #[derive(Debug)]
 pub struct TransactionRef {
     pub signature: String,
     pub index: i32,
     pub message: MessageRef,
     pub instructions: Vec<InstructionRef>,
 }

 #[derive(Debug)]
 pub struct MessageRef {
     pub account_keys: Vec<String>,
 }

 #[derive(Debug)]
 pub struct InstructionRef {
     pub program_id: String,
     pub accounts: Vec<u8>,
     pub data: Vec<u8>,
     pub index: i32,
 }

 /// SPL Token program id on Solana mainnet.
 pub const SPL_TOKEN_PROGRAM_ID: &str = "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA";

 /// SPL Token instruction discriminators.
 pub const INSTR_TRANSFER: u8 = 3;
 pub const INSTR_TRANSFER_CHECKED: u8 = 12;
 pub const INSTR_MINT_TO: u8 = 7;
 pub const INSTR_MINT_TO_CHECKED: u8 = 13;
 pub const INSTR_BURN: u8 = 8;
 pub const INSTR_BURN_CHECKED: u8 = 14;

 pub fn extract_transfers_from_block(block: &BlockRef, mint_whitelist: &[String]) -> Vec<TokenTransfer> {
     let mut transfers = Vec::new();

     for tx in &block.transactions {
         for ix in &tx.instructions {
             if ix.program_id != SPL_TOKEN_PROGRAM_ID {
                 continue;
             }

             if let Some(t) = parse_spl_transfer(block, tx, ix, mint_whitelist) {
                 transfers.push(t);
             }
         }
     }

     transfers
 }

 fn parse_spl_transfer(
     block: &BlockRef,
     tx: &TransactionRef,
     ix: &InstructionRef,
     mint_whitelist: &[String],
 ) -> Option<TokenTransfer> {
     if ix.data.is_empty() {
         return None;
     }

     let instr_type = ix.data[0];

     match instr_type {
         INSTR_TRANSFER => parse_transfer(block, tx, ix, mint_whitelist),
         INSTR_TRANSFER_CHECKED => parse_transfer_checked(block, tx, ix, mint_whitelist),
         INSTR_MINT_TO => parse_mint_to(block, tx, ix, mint_whitelist),
         INSTR_MINT_TO_CHECKED => parse_mint_to_checked(block, tx, ix, mint_whitelist),
         INSTR_BURN => parse_burn(block, tx, ix, mint_whitelist),
         INSTR_BURN_CHECKED => parse_burn_checked(block, tx, ix, mint_whitelist),
         _ => None,
     }
 }

 /// Parse Transfer instruction (3):
 /// Accounts: [source_token_account, mint, dest_token_account, owner_or_delegate]
 /// Data: [discriminator: 1 byte] [amount: u64 LE]
 fn parse_transfer(
     block: &BlockRef,
     tx: &TransactionRef,
     ix: &InstructionRef,
     mint_whitelist: &[String],
 ) -> Option<TokenTransfer> {
     if ix.accounts.len() < 3 {
         return None;
     }

     let source_ata_idx = ix.accounts.get(0).copied()? as usize;
     let dest_ata_idx = ix.accounts.get(2).copied()? as usize;

     let source_ata = tx.message.account_keys.get(source_ata_idx)?.clone();
     let dest_ata = tx.message.account_keys.get(dest_ata_idx)?.clone();

     // Amount is at bytes 1..9 (u64 LE).
     let amount = read_u64_le(&ix.data[1..])?;

     // For Transfer, we need to know the mint from on-chain data, which we might not have.
     // For now, we'll accept any transfer if mint_whitelist is empty, or skip if we can't match.
     // In a real scenario, we'd cache token account -> mint mappings from Firehose account state.
     
     // If mint_whitelist is provided, we can't match without mint knowledge, so skip.
     // If mint_whitelist is empty, we'll allow the transfer but mint_pubkey is a placeholder.
     if !mint_whitelist.is_empty() {
         // Can't determine mint from instruction alone; would need on-chain account data.
         return None;
     }

     let block_time = block
         .block_time_unix
         .map(|t| Utc.timestamp_opt(t, 0).single())
         .flatten();

     Some(TokenTransfer {
         signature: tx.signature.clone(),
         slot: block.slot,
         block_time,
         mint_pubkey: "unknown_mint".to_string(),
         source_owner: source_ata.clone(),
         dest_owner: dest_ata.clone(),
         source_ata,
         dest_ata,
         amount: amount as i64,
         tx_index: tx.index,
         ix_index: ix.index,
     })
 }

 /// Parse TransferChecked instruction (12):
 /// Accounts: [source_token_account, mint, dest_token_account, owner_or_delegate]
 /// Data: [discriminator: 1 byte] [token_amount: u64 LE] [decimals: 1 byte]
 fn parse_transfer_checked(
     block: &BlockRef,
     tx: &TransactionRef,
     ix: &InstructionRef,
     mint_whitelist: &[String],
 ) -> Option<TokenTransfer> {
     if ix.accounts.len() < 3 || ix.data.len() < 10 {
         return None;
     }

     let source_ata_idx = ix.accounts.get(0).copied()? as usize;
     let mint_idx = ix.accounts.get(1).copied()? as usize;
     let dest_ata_idx = ix.accounts.get(2).copied()? as usize;

     let source_ata = tx.message.account_keys.get(source_ata_idx)?.clone();
     let mint_pubkey = tx.message.account_keys.get(mint_idx)?.clone();
     let dest_ata = tx.message.account_keys.get(dest_ata_idx)?.clone();

     // Check if mint is in whitelist.
     if !mint_whitelist.is_empty() && !mint_whitelist.contains(&mint_pubkey) {
         return None;
     }

     let amount = read_u64_le(&ix.data[1..])?;

     let block_time = block
         .block_time_unix
         .map(|t| Utc.timestamp_opt(t, 0).single())
         .flatten();

     Some(TokenTransfer {
         signature: tx.signature.clone(),
         slot: block.slot,
         block_time,
         mint_pubkey,
         source_owner: source_ata.clone(),
         dest_owner: dest_ata.clone(),
         source_ata,
         dest_ata,
         amount: amount as i64,
         tx_index: tx.index,
         ix_index: ix.index,
     })
 }

 /// Parse MintTo instruction (7):
 /// Accounts: [mint, dest_token_account, mint_authority]
 /// Data: [discriminator: 1 byte] [amount: u64 LE]
 fn parse_mint_to(
     block: &BlockRef,
     tx: &TransactionRef,
     ix: &InstructionRef,
     mint_whitelist: &[String],
 ) -> Option<TokenTransfer> {
     if ix.accounts.len() < 2 || ix.data.len() < 9 {
         return None;
     }

     let mint_idx = ix.accounts.get(0).copied()? as usize;
     let dest_ata_idx = ix.accounts.get(1).copied()? as usize;

     let mint_pubkey = tx.message.account_keys.get(mint_idx)?.clone();
     let dest_ata = tx.message.account_keys.get(dest_ata_idx)?.clone();

     if !mint_whitelist.is_empty() && !mint_whitelist.contains(&mint_pubkey) {
         return None;
     }

     let amount = read_u64_le(&ix.data[1..])?;

     let block_time = block
         .block_time_unix
         .map(|t| Utc.timestamp_opt(t, 0).single())
         .flatten();

     Some(TokenTransfer {
         signature: tx.signature.clone(),
         slot: block.slot,
         block_time,
         mint_pubkey,
         source_owner: "system".to_string(), // MintTo has no source_owner, use system
         dest_owner: dest_ata.clone(),
         source_ata: "system".to_string(),
         dest_ata,
         amount: amount as i64,
         tx_index: tx.index,
         ix_index: ix.index,
     })
 }

 /// Parse MintToChecked instruction (13):
 /// Accounts: [mint, dest_token_account, mint_authority]
 /// Data: [discriminator: 1 byte] [token_amount: u64 LE] [decimals: 1 byte]
 fn parse_mint_to_checked(
     block: &BlockRef,
     tx: &TransactionRef,
     ix: &InstructionRef,
     mint_whitelist: &[String],
 ) -> Option<TokenTransfer> {
     if ix.accounts.len() < 2 || ix.data.len() < 10 {
         return None;
     }

     let mint_idx = ix.accounts.get(0).copied()? as usize;
     let dest_ata_idx = ix.accounts.get(1).copied()? as usize;

     let mint_pubkey = tx.message.account_keys.get(mint_idx)?.clone();
     let dest_ata = tx.message.account_keys.get(dest_ata_idx)?.clone();

     if !mint_whitelist.is_empty() && !mint_whitelist.contains(&mint_pubkey) {
         return None;
     }

     let amount = read_u64_le(&ix.data[1..])?;

     let block_time = block
         .block_time_unix
         .map(|t| Utc.timestamp_opt(t, 0).single())
         .flatten();

     Some(TokenTransfer {
         signature: tx.signature.clone(),
         slot: block.slot,
         block_time,
         mint_pubkey,
         source_owner: "system".to_string(),
         dest_owner: dest_ata.clone(),
         source_ata: "system".to_string(),
         dest_ata,
         amount: amount as i64,
         tx_index: tx.index,
         ix_index: ix.index,
     })
 }

 /// Parse Burn instruction (8):
 /// Accounts: [token_account, mint, owner_or_delegate]
 /// Data: [discriminator: 1 byte] [amount: u64 LE]
 fn parse_burn(
     block: &BlockRef,
     tx: &TransactionRef,
     ix: &InstructionRef,
     mint_whitelist: &[String],
 ) -> Option<TokenTransfer> {
     if ix.accounts.len() < 2 || ix.data.len() < 9 {
         return None;
     }

     let source_ata_idx = ix.accounts.get(0).copied()? as usize;
     let mint_idx = ix.accounts.get(1).copied()? as usize;

     let source_ata = tx.message.account_keys.get(source_ata_idx)?.clone();
     let mint_pubkey = tx.message.account_keys.get(mint_idx)?.clone();

     if !mint_whitelist.is_empty() && !mint_whitelist.contains(&mint_pubkey) {
         return None;
     }

     let amount = read_u64_le(&ix.data[1..])?;

     let block_time = block
         .block_time_unix
         .map(|t| Utc.timestamp_opt(t, 0).single())
         .flatten();

     Some(TokenTransfer {
         signature: tx.signature.clone(),
         slot: block.slot,
         block_time,
         mint_pubkey,
         source_owner: source_ata.clone(),
         dest_owner: "burn".to_string(), // Burn targets void
         source_ata,
         dest_ata: "burn".to_string(),
         amount: amount as i64,
         tx_index: tx.index,
         ix_index: ix.index,
     })
 }

 /// Parse BurnChecked instruction (14):
 /// Accounts: [token_account, mint, owner_or_delegate]
 /// Data: [discriminator: 1 byte] [token_amount: u64 LE] [decimals: 1 byte]
 fn parse_burn_checked(
     block: &BlockRef,
     tx: &TransactionRef,
     ix: &InstructionRef,
     mint_whitelist: &[String],
 ) -> Option<TokenTransfer> {
     if ix.accounts.len() < 2 || ix.data.len() < 10 {
         return None;
     }

     let source_ata_idx = ix.accounts.get(0).copied()? as usize;
     let mint_idx = ix.accounts.get(1).copied()? as usize;

     let source_ata = tx.message.account_keys.get(source_ata_idx)?.clone();
     let mint_pubkey = tx.message.account_keys.get(mint_idx)?.clone();

     if !mint_whitelist.is_empty() && !mint_whitelist.contains(&mint_pubkey) {
         return None;
     }

     let amount = read_u64_le(&ix.data[1..])?;

     let block_time = block
         .block_time_unix
         .map(|t| Utc.timestamp_opt(t, 0).single())
         .flatten();

     Some(TokenTransfer {
         signature: tx.signature.clone(),
         slot: block.slot,
         block_time,
         mint_pubkey,
         source_owner: source_ata.clone(),
         dest_owner: "burn".to_string(),
         source_ata,
         dest_ata: "burn".to_string(),
         amount: amount as i64,
         tx_index: tx.index,
         ix_index: ix.index,
     })
 }

 fn read_u64_le(bytes: &[u8]) -> Option<u64> {
    if bytes.len() < 8 {
        return None;
    }
    let mut arr = [0u8; 8];
    arr.copy_from_slice(&bytes[..8]);
    Some(u64::from_le_bytes(arr))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_transfer_checked() {
        let block = BlockRef {
            slot: 100,
            block_time_unix: Some(1000),
            transactions: vec![TransactionRef {
                signature: "sig123".to_string(),
                index: 0,
                message: MessageRef {
                    account_keys: vec![
                        "source_ata".to_string(),
                        "test_mint".to_string(),
                        "dest_ata".to_string(),
                        "owner".to_string(),
                    ],
                },
                instructions: vec![InstructionRef {
                    program_id: SPL_TOKEN_PROGRAM_ID.to_string(),
                    accounts: vec![0, 1, 2, 3],
                    data: {
                        let mut d = vec![INSTR_TRANSFER_CHECKED];
                        d.extend_from_slice(&(1_000_000u64).to_le_bytes());
                        d.push(6); // decimals
                        d
                    },
                    index: 0,
                }],
            }],
        };

        let transfers =
            extract_transfers_from_block(&block, &vec!["test_mint".to_string()]);

        assert_eq!(transfers.len(), 1);
        assert_eq!(transfers[0].mint_pubkey, "test_mint");
        assert_eq!(transfers[0].amount, 1_000_000);
        assert_eq!(transfers[0].source_ata, "source_ata");
        assert_eq!(transfers[0].dest_ata, "dest_ata");
    }

    #[test]
    fn test_parse_transfer_checked_whitelist_filter() {
        let block = BlockRef {
            slot: 100,
            block_time_unix: Some(1000),
            transactions: vec![TransactionRef {
                signature: "sig123".to_string(),
                index: 0,
                message: MessageRef {
                    account_keys: vec![
                        "source_ata".to_string(),
                        "different_mint".to_string(),
                        "dest_ata".to_string(),
                        "owner".to_string(),
                    ],
                },
                instructions: vec![InstructionRef {
                    program_id: SPL_TOKEN_PROGRAM_ID.to_string(),
                    accounts: vec![0, 1, 2, 3],
                    data: {
                        let mut d = vec![INSTR_TRANSFER_CHECKED];
                        d.extend_from_slice(&(500_000u64).to_le_bytes());
                        d.push(6);
                        d
                    },
                    index: 0,
                }],
            }],
        };

        let transfers =
            extract_transfers_from_block(&block, &vec!["test_mint".to_string()]);

        // Should be filtered out because mint is not in whitelist
        assert_eq!(transfers.len(), 0);
    }

    #[test]
    fn test_parse_mint_to_checked() {
        let block = BlockRef {
            slot: 200,
            block_time_unix: Some(2000),
            transactions: vec![TransactionRef {
                signature: "mint_sig".to_string(),
                index: 1,
                message: MessageRef {
                    account_keys: vec![
                        "test_mint".to_string(),
                        "dest_ata".to_string(),
                        "mint_authority".to_string(),
                    ],
                },
                instructions: vec![InstructionRef {
                    program_id: SPL_TOKEN_PROGRAM_ID.to_string(),
                    accounts: vec![0, 1, 2],
                    data: {
                        let mut d = vec![INSTR_MINT_TO_CHECKED];
                        d.extend_from_slice(&(10_000_000u64).to_le_bytes());
                        d.push(6);
                        d
                    },
                    index: 0,
                }],
            }],
        };

        let transfers =
            extract_transfers_from_block(&block, &vec!["test_mint".to_string()]);

        assert_eq!(transfers.len(), 1);
        assert_eq!(transfers[0].mint_pubkey, "test_mint");
        assert_eq!(transfers[0].amount, 10_000_000);
        assert_eq!(transfers[0].source_owner, "system");
        assert_eq!(transfers[0].dest_owner, "dest_ata");
    }

    #[test]
    fn test_parse_burn_checked() {
        let block = BlockRef {
            slot: 300,
            block_time_unix: Some(3000),
            transactions: vec![TransactionRef {
                signature: "burn_sig".to_string(),
                index: 2,
                message: MessageRef {
                    account_keys: vec![
                        "source_ata".to_string(),
                        "test_mint".to_string(),
                        "owner".to_string(),
                    ],
                },
                instructions: vec![InstructionRef {
                    program_id: SPL_TOKEN_PROGRAM_ID.to_string(),
                    accounts: vec![0, 1, 2],
                    data: {
                        let mut d = vec![INSTR_BURN_CHECKED];
                        d.extend_from_slice(&(500_000u64).to_le_bytes());
                        d.push(6);
                        d
                    },
                    index: 0,
                }],
            }],
        };

        let transfers =
            extract_transfers_from_block(&block, &vec!["test_mint".to_string()]);

        assert_eq!(transfers.len(), 1);
        assert_eq!(transfers[0].mint_pubkey, "test_mint");
        assert_eq!(transfers[0].amount, 500_000);
        assert_eq!(transfers[0].source_owner, "source_ata");
        assert_eq!(transfers[0].dest_owner, "burn");
    }
}

