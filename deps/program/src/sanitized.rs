use crate::sanitize::{Sanitize, SanitizeError};
use crate::{compiled_keys::CompiledKeys, instruction::Instruction, pubkey::Pubkey};
use anyhow::{anyhow, Result};
use bitcode::{Decode, Encode};
use borsh::{BorshDeserialize, BorshSerialize};
#[cfg(feature = "fuzzing")]
use libfuzzer_sys::arbitrary;
use serde::{Deserialize, Serialize};
use sha256::digest;
use std::collections::HashSet;
const MAX_INSTRUCTION_COUNT_PER_TRANSACTION: usize = u8::MAX as usize;
/// A sanitized message that has been checked for validity and processed to improve
/// runtime performance.
///
/// This struct wraps an `ArchMessage` and provides additional caching of account
/// permissions for more efficient runtime access.
#[derive(Debug, Clone)]
pub struct SanitizedMessage {
    /// The underlying message containing instructions, account keys, and header information
    pub message: ArchMessage,
    /// List of boolean with same length as account_keys(), each boolean value indicates if
    /// corresponding account key is writable or not.
    pub is_writable_account_cache: Vec<bool>,
}

impl SanitizedMessage {
    /// Creates a new `SanitizedMessage` by processing the provided `ArchMessage`.
    ///
    /// This constructor will initialize the writable account cache for faster permission checks.
    ///
    /// # Arguments
    ///
    /// * `message` - The `ArchMessage` to wrap and process
    ///
    /// # Returns
    ///
    /// A new `SanitizedMessage` instance
    pub fn new(message: ArchMessage) -> Self {
        let is_writable_account_cache = message
            .account_keys
            .iter()
            .enumerate()
            .map(|(i, _key)| message.is_writable_index(i))
            .collect::<Vec<_>>();
        Self {
            message,
            is_writable_account_cache,
        }
    }

    /// Checks if the account at the given index is a signer.
    ///
    /// # Arguments
    ///
    /// * `index` - The index of the account in the account keys list
    ///
    /// # Returns
    ///
    /// `true` if the account is a signer, `false` otherwise
    pub fn is_signer(&self, index: usize) -> bool {
        self.message.is_signer(index)
    }

    /// Checks if the account at the given index is writable.
    ///
    /// This method uses the pre-computed writable account cache for efficiency.
    ///
    /// # Arguments
    ///
    /// * `index` - The index of the account in the account keys list
    ///
    /// # Returns
    ///
    /// `true` if the account is writable, `false` otherwise
    pub fn is_writable(&self, index: usize) -> bool {
        *self.is_writable_account_cache.get(index).unwrap_or(&false)
    }

    /// Returns a reference to the instructions in the message.
    ///
    /// # Returns
    ///
    /// A reference to the vector of `SanitizedInstruction`s
    pub fn instructions(&self) -> &Vec<SanitizedInstruction> {
        &self.message.instructions
    }
}

/// A message in the Arch Network that contains instructions to be executed,
/// account keys involved in the transaction, and metadata in the header.
#[derive(
    Clone,
    Debug,
    Eq,
    PartialEq,
    Serialize,
    Deserialize,
    BorshSerialize,
    BorshDeserialize,
    Default,
    Encode,
    Decode,
)]
#[cfg_attr(feature = "fuzzing", derive(arbitrary::Arbitrary))]

pub struct ArchMessage {
    /// Header containing metadata about the message
    pub header: MessageHeader,
    /// List of all account public keys used in this message
    pub account_keys: Vec<Pubkey>,
    /// The id of a recent ledger entry.
    pub recent_blockhash: String,
    /// List of instructions to execute
    pub instructions: Vec<SanitizedInstruction>,
}

impl Sanitize for ArchMessage {
    fn sanitize(&self) -> Result<(), SanitizeError> {
        // Check for duplicate account keys
        let mut unique_keys = HashSet::new();
        for key in &self.account_keys {
            if !unique_keys.insert(key) {
                return Err(SanitizeError::DuplicateAccount);
            }
        }

        // signing area and read-only non-signing area should not overlap
        if self.header.num_required_signatures as usize
            + self.header.num_readonly_unsigned_accounts as usize
            > self.account_keys.len()
        {
            return Err(SanitizeError::IndexOutOfBounds);
        }

        // there should be at least 1 RW fee-payer account.
        if self.header.num_readonly_signed_accounts >= self.header.num_required_signatures {
            return Err(SanitizeError::IndexOutOfBounds);
        }

        if let Ok(recent_blockhash) = hex::decode(&self.recent_blockhash) {
            if recent_blockhash.len() != 32 {
                return Err(SanitizeError::InvalidRecentBlockhash);
            }
        } else {
            return Err(SanitizeError::InvalidRecentBlockhash);
        }

        for ci in &self.instructions {
            if ci.program_id_index as usize >= self.account_keys.len() {
                return Err(SanitizeError::IndexOutOfBounds);
            }
            // A program cannot be a payer.
            if ci.program_id_index == 0 {
                return Err(SanitizeError::IndexOutOfBounds);
            }
            for ai in &ci.accounts {
                if *ai as usize >= self.account_keys.len() {
                    return Err(SanitizeError::IndexOutOfBounds);
                }
            }
        }
        Ok(())
    }
}

impl ArchMessage {
    /// Returns true if the account at the specified index was requested to be
    /// writable. This method should not be used directly.
    ///
    /// # Arguments
    ///
    /// * `i` - The index of the account to check
    ///
    /// # Returns
    ///
    /// `true` if the account is writable, `false` otherwise
    pub fn is_writable_index(&self, i: usize) -> bool {
        i < (self.header.num_required_signatures - self.header.num_readonly_signed_accounts)
            as usize
            || (i >= self.header.num_required_signatures as usize
                && i < self.account_keys.len()
                    - self.header.num_readonly_unsigned_accounts as usize)
    }

    /// Returns a reference to the message header.
    ///
    /// # Returns
    ///
    /// A reference to the `MessageHeader`
    pub fn header(&self) -> &MessageHeader {
        &self.header
    }

    /// Checks if the account at the given index is a signer.
    ///
    /// An account is a signer if its index is less than the number of required signatures
    /// specified in the message header.
    ///
    /// # Arguments
    ///
    /// * `index` - The index of the account in the account keys list
    ///
    /// # Returns
    ///
    /// `true` if the account is a signer, `false` otherwise
    pub fn is_signer(&self, index: usize) -> bool {
        index < usize::from(self.header().num_required_signatures)
    }

    pub fn get_account_key(&self, index: usize) -> Option<&Pubkey> {
        self.account_keys.get(index)
    }
    /// Collects all unique Pubkeys used in instructions, including program_ids and account references
    pub fn get_unique_instruction_account_keys(&self) -> HashSet<Pubkey> {
        let mut unique_keys = HashSet::new();

        for instruction in &self.instructions {
            // Add all account references
            for account_index in &instruction.accounts {
                let pubkey = self
                    .account_keys
                    .get(*account_index as usize)
                    .expect("Account index out of bounds"); // Panic if index doesn't exist
                unique_keys.insert(*pubkey);
            }
        }

        unique_keys
    }

    pub fn get_recent_blockhash(&self) -> String {
        self.recent_blockhash.clone()
    }

    pub fn new(
        instructions: &[Instruction],
        payer: Option<Pubkey>,
        recent_blockhash: String,
    ) -> Self {
        let compiled_keys = CompiledKeys::compile(instructions, payer);
        let (header, account_keys) = compiled_keys
            .try_into_message_components()
            .expect("overflow when compiling message keys");
        let instructions = compile_instructions(instructions, &account_keys);
        Self::new_with_compiled_instructions(
            header.num_required_signatures,
            header.num_readonly_signed_accounts,
            header.num_readonly_unsigned_accounts,
            account_keys,
            recent_blockhash,
            instructions,
        )
    }

    pub fn new_with_compiled_instructions(
        num_required_signatures: u8,
        num_readonly_signed_accounts: u8,
        num_readonly_unsigned_accounts: u8,
        account_keys: Vec<Pubkey>,
        recent_blockhash: String,
        instructions: Vec<SanitizedInstruction>,
    ) -> Self {
        Self {
            header: MessageHeader {
                num_required_signatures,
                num_readonly_signed_accounts,
                num_readonly_unsigned_accounts,
            },
            account_keys,
            recent_blockhash,
            instructions,
        }
    }

    pub fn serialize(&self) -> Vec<u8> {
        let mut buffer = Vec::new();

        // Serialize MessageHeader
        buffer.extend_from_slice(&[
            self.header.num_required_signatures,
            self.header.num_readonly_signed_accounts,
            self.header.num_readonly_unsigned_accounts,
        ]);

        // Serialize account_keys
        buffer.extend_from_slice(&(self.account_keys.len() as u32).to_le_bytes());
        for key in &self.account_keys {
            buffer.extend_from_slice(key.as_ref());
        }

        // Serialize recent_blockhash
        buffer.extend_from_slice(&hex::decode(&self.recent_blockhash).unwrap());

        // Serialize instructions
        buffer.extend_from_slice(&(self.instructions.len() as u32).to_le_bytes());
        for instruction in &self.instructions {
            buffer.extend(instruction.serialize());
        }

        buffer
    }

    pub fn deserialize(bytes: &[u8]) -> Result<Self> {
        if bytes.len() < 3 {
            return Err(anyhow!("Invalid message length: less than header size"));
        }

        let mut pos = 0;

        // Deserialize header
        let header = MessageHeader {
            num_required_signatures: bytes[pos],
            num_readonly_signed_accounts: bytes[pos + 1],
            num_readonly_unsigned_accounts: bytes[pos + 2],
        };
        pos += 3;

        // Deserialize account_keys
        if bytes.len() < pos + 4 {
            return Err(anyhow!(
                "Invalid message length: insufficient bytes for account keys length"
            ));
        }
        let num_keys = u32::from_le_bytes(
            bytes[pos..pos + 4]
                .try_into()
                .map_err(|_| anyhow!("Invalid byte conversion for account keys length"))?,
        );
        pos += 4;

        let account_keys_size = num_keys
            .checked_mul(32)
            .ok_or_else(|| anyhow!("Message length overflow in account keys"))?;

        if bytes.len() < pos + account_keys_size as usize {
            return Err(anyhow!(
                "Invalid message length: insufficient bytes for account keys"
            ));
        }

        let mut account_keys = Vec::with_capacity(num_keys as usize);
        for _ in 0..num_keys {
            if bytes.len() < pos + 32 {
                return Err(anyhow!(
                    "Invalid message length: insufficient bytes for {} account keys",
                    num_keys
                ));
            }
            account_keys.push(Pubkey::from_slice(&bytes[pos..pos + 32]));
            pos += 32;
        }

        if bytes.len() < pos + 32 {
            return Err(anyhow!(
                "Invalid message length: insufficient bytes for recent blockhash"
            ));
        }
        let recent_blockhash = hex::encode(&bytes[pos..pos + 32]);
        pos += 32;

        // Deserialize instructions
        if bytes.len() < pos + 4 {
            return Err(anyhow!(
                "Invalid message length: insufficient bytes for instructions length"
            ));
        }
        let num_instructions = u32::from_le_bytes(
            bytes[pos..pos + 4]
                .try_into()
                .map_err(|_| anyhow!("Invalid byte conversion for instructions length"))?,
        );
        pos += 4;

        if num_instructions as usize > MAX_INSTRUCTION_COUNT_PER_TRANSACTION {
            return Err(anyhow!(
                "Invalid message length: too many instructions: {} > {}",
                num_instructions,
                MAX_INSTRUCTION_COUNT_PER_TRANSACTION
            ));
        }

        let mut instructions = Vec::with_capacity(num_instructions as usize);
        for _ in 0..num_instructions {
            let (instruction, bytes_read) = SanitizedInstruction::deserialize(&bytes[pos..])?;
            instructions.push(instruction);
            pos += bytes_read;
        }

        Ok(Self {
            header,
            account_keys,
            recent_blockhash,
            instructions,
        })
    }

    pub fn hash(&self) -> Vec<u8> {
        let serialized_message = self.serialize();
        let first_hash = digest(serialized_message);
        digest(first_hash.as_bytes()).as_bytes().to_vec()
    }

    /// Program instructions iterator which includes each instruction's program
    /// id.
    pub fn program_instructions_iter(
        &self,
    ) -> impl Iterator<Item = (&Pubkey, &SanitizedInstruction)> {
        self.instructions.iter().map(|ix| {
            (
                self.account_keys
                    .get(usize::from(ix.program_id_index))
                    .expect("program id index is sanitized"),
                ix,
            )
        })
    }
}

fn position(keys: &[Pubkey], key: &Pubkey) -> u8 {
    keys.iter().position(|k| k == key).unwrap() as u8
}

fn compile_instruction(ix: &Instruction, keys: &[Pubkey]) -> SanitizedInstruction {
    let accounts: Vec<_> = ix
        .accounts
        .iter()
        .map(|account_meta| position(keys, &account_meta.pubkey))
        .collect();

    SanitizedInstruction {
        program_id_index: position(keys, &ix.program_id),
        data: ix.data.clone(),
        accounts,
    }
}

fn compile_instructions(ixs: &[Instruction], keys: &[Pubkey]) -> Vec<SanitizedInstruction> {
    ixs.iter().map(|ix| compile_instruction(ix, keys)).collect()
}

/// A sanitized instruction included in an `ArchMessage`.
///
/// This struct contains information about a single instruction including
/// the program to execute, the accounts to operate on, and the instruction data.
#[derive(
    Clone,
    Debug,
    Eq,
    PartialEq,
    Serialize,
    Deserialize,
    BorshSerialize,
    BorshDeserialize,
    Encode,
    Decode,
)]
#[cfg_attr(feature = "fuzzing", derive(arbitrary::Arbitrary))]

pub struct SanitizedInstruction {
    /// The public key of the program that will process this instruction
    pub program_id_index: u8,
    /// Ordered indices into the message's account keys, indicating which accounts
    /// this instruction will operate on
    pub accounts: Vec<u8>,
    /// The program-specific instruction data
    pub data: Vec<u8>,
}

/// The header of an `ArchMessage` that contains metadata about the message
/// and its authorization requirements.
#[derive(
    Clone,
    Debug,
    Eq,
    PartialEq,
    Serialize,
    Deserialize,
    BorshSerialize,
    BorshDeserialize,
    Default,
    Encode,
    Decode,
)]
#[cfg_attr(feature = "fuzzing", derive(arbitrary::Arbitrary))]

pub struct MessageHeader {
    /// The number of signatures required for this message to be considered
    /// valid
    pub num_required_signatures: u8,

    /// The last `num_readonly_signed_accounts` of the signed keys are read-only
    /// accounts.
    pub num_readonly_signed_accounts: u8,

    /// The last `num_readonly_unsigned_accounts` of the unsigned keys are
    /// read-only accounts.
    pub num_readonly_unsigned_accounts: u8,
}

impl SanitizedInstruction {
    pub fn serialize(&self) -> Vec<u8> {
        let mut buffer = Vec::new();

        // Write program_id_index
        buffer.push(self.program_id_index);

        // Write accounts (now using u8 instead of u16)
        buffer.extend_from_slice(&(self.accounts.len() as u32).to_le_bytes());
        buffer.extend_from_slice(&self.accounts); // Direct extend since accounts are now u8

        // Write data
        buffer.extend_from_slice(&(self.data.len() as u32).to_le_bytes());
        buffer.extend_from_slice(&self.data);

        buffer
    }

    pub fn deserialize(bytes: &[u8]) -> Result<(Self, usize)> {
        if bytes.len() < 1 {
            return Err(anyhow!("Invalid instruction length: empty buffer"));
        }

        let mut pos = 0;
        let program_id_index = bytes[pos];
        pos += 1;

        if bytes.len() < pos + 4 {
            return Err(anyhow!(
                "Invalid instruction length: insufficient bytes for accounts length"
            ));
        }
        let num_accounts = u32::from_le_bytes(
            bytes[pos..pos + 4]
                .try_into()
                .map_err(|_| anyhow!("Invalid byte conversion for accounts length"))?,
        );
        pos += 4;

        if bytes.len() < pos + num_accounts as usize {
            return Err(anyhow!("Insufficient bytes for account indices"));
        }
        let accounts = bytes[pos..pos + num_accounts as usize].to_vec();
        pos += num_accounts as usize;

        if bytes.len() < pos + 4 {
            return Err(anyhow!(
                "Invalid instruction length: insufficient bytes for data length"
            ));
        }
        let data_len = u32::from_le_bytes(
            bytes[pos..pos + 4]
                .try_into()
                .map_err(|_| anyhow!("Invalid byte conversion for data length"))?,
        );
        pos += 4;

        if bytes.len() < pos + data_len as usize {
            return Err(anyhow!("Insufficient bytes for instruction data"));
        }
        let data = bytes[pos..pos + data_len as usize].to_vec();
        pos += data_len as usize;

        Ok((
            Self {
                program_id_index,
                accounts,
                data,
            },
            pos,
        ))
    }
}

#[cfg(test)]
mod tests {
    use crate::account::AccountMeta;

    use super::*;

    #[test]
    fn test_message_with_mixed_signer_privileges() {
        let signer_pubkey = Pubkey::new_unique();
        let account_1 = Pubkey::new_unique();
        let account_2 = Pubkey::new_unique();

        // Instruction 1: signer_pubkey is a signer
        let instruction_1 = Instruction {
            program_id: Pubkey::new_unique(),
            accounts: vec![
                AccountMeta::new(account_1, false),
                AccountMeta::new(signer_pubkey, true), // signer here
            ],
            data: vec![],
        };

        // Instruction 2: signer_pubkey is not a signer
        let instruction_2 = Instruction {
            program_id: Pubkey::new_unique(),
            accounts: vec![
                AccountMeta::new(account_2, false),
                AccountMeta::new(signer_pubkey, false), // not a signer here
            ],
            data: vec![],
        };

        // Create ArchMessage from instructions
        let message = ArchMessage::new(&[instruction_1, instruction_2], None, hex::encode([0; 32]));

        // Print message details
        println!("Message Header:");
        println!(
            "  Required signatures: {}",
            message.header.num_required_signatures
        );
        println!(
            "  Readonly signed accounts: {}",
            message.header.num_readonly_signed_accounts
        );
        println!(
            "  Readonly unsigned accounts: {}",
            message.header.num_readonly_unsigned_accounts
        );

        println!("\nAccount Keys:");
        for (i, key) in message.account_keys.iter().enumerate() {
            println!(
                "  {}: {} (writable: {})",
                i,
                key,
                message.is_writable_index(i)
            );
        }

        println!("\nInstructions:");
        for (i, instruction) in message.instructions.iter().enumerate() {
            println!("  Instruction {}:", i);
            println!("    Program ID Index: {}", instruction.program_id_index);
            println!("    Account Indices: {:?}", instruction.accounts);
            println!("    Data: {:?}", instruction.data);
        }

        // Add some assertions to verify the message structure
        assert!(
            message.account_keys.contains(&signer_pubkey),
            "Signer pubkey should be in account keys"
        );
        assert!(
            message.account_keys.contains(&account_1),
            "Account 1 should be in account keys"
        );
        assert!(
            message.account_keys.contains(&account_2),
            "Account 2 should be in account keys"
        );
    }

    #[test]
    fn test_message_serialization_deserialization() {
        // Create a sample message
        let header = MessageHeader {
            num_required_signatures: 2,
            num_readonly_signed_accounts: 1,
            num_readonly_unsigned_accounts: 1,
        };

        let account_keys = vec![
            Pubkey::new_unique(), // signer 1
            Pubkey::new_unique(), // signer 2
            Pubkey::new_unique(), // non-signer program
            Pubkey::new_unique(), // non-signer data account
        ];

        let instruction1 = SanitizedInstruction {
            program_id_index: 2,     // Using the third account as program
            accounts: vec![0, 1, 3], // Using signers and data account
            data: vec![1, 2, 3, 4],  // Some instruction data
        };

        let instruction2 = SanitizedInstruction {
            program_id_index: 2,
            accounts: vec![1, 3],
            data: vec![5, 6, 7],
        };

        let original_message = ArchMessage {
            header,
            account_keys,
            recent_blockhash: hex::encode([0; 32]),
            instructions: vec![instruction1, instruction2],
        };

        // Serialize the message
        let serialized = original_message.serialize();

        // Deserialize back
        let deserialized_message =
            ArchMessage::deserialize(&serialized).expect("Failed to deserialize message");

        // Verify all fields match
        assert_eq!(
            deserialized_message.header.num_required_signatures,
            original_message.header.num_required_signatures
        );
        assert_eq!(
            deserialized_message.header.num_readonly_signed_accounts,
            original_message.header.num_readonly_signed_accounts
        );
        assert_eq!(
            deserialized_message.header.num_readonly_unsigned_accounts,
            original_message.header.num_readonly_unsigned_accounts
        );

        // Check account keys
        assert_eq!(
            deserialized_message.account_keys.len(),
            original_message.account_keys.len()
        );
        for (original, deserialized) in original_message
            .account_keys
            .iter()
            .zip(deserialized_message.account_keys.iter())
        {
            assert_eq!(original, deserialized);
        }

        // Check instructions
        assert_eq!(
            deserialized_message.instructions.len(),
            original_message.instructions.len()
        );
        for (original, deserialized) in original_message
            .instructions
            .iter()
            .zip(deserialized_message.instructions.iter())
        {
            assert_eq!(original.program_id_index, deserialized.program_id_index);
            assert_eq!(original.accounts, deserialized.accounts);
            assert_eq!(original.data, deserialized.data);
        }
    }

    #[test]
    fn test_instruction_deserialization_error_cases() {
        // Test empty buffer
        let result = SanitizedInstruction::deserialize(&[]);
        assert!(result.is_err());
        assert_eq!(
            result.unwrap_err().to_string(),
            "Invalid instruction length: empty buffer"
        );

        // Test buffer too small for program_id_index
        let result = SanitizedInstruction::deserialize(&[]);
        assert!(result.is_err());
        assert_eq!(
            result.unwrap_err().to_string(),
            "Invalid instruction length: empty buffer"
        );

        // Test buffer too small for accounts length
        let result = SanitizedInstruction::deserialize(&[0]);
        assert!(result.is_err());
        assert_eq!(
            result.unwrap_err().to_string(),
            "Invalid instruction length: insufficient bytes for accounts length"
        );

        // Test impossibly large accounts length
        let invalid_instruction = vec![
            0, // program_id_index
            255, 255, 255, 255, // impossibly large number of accounts
        ];
        let result = SanitizedInstruction::deserialize(&invalid_instruction);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Insufficient bytes"));

        // Test truncated account indices
        let truncated_accounts = vec![
            0, // program_id_index
            2, 0, 0, 0, // 2 accounts
            0, // first account index
               // missing second account index
        ];
        let result = SanitizedInstruction::deserialize(&truncated_accounts);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Insufficient bytes"));

        // Test invalid data length
        let invalid_data = vec![
            0, // program_id_index
            1, 0, 0, 0, // 1 account
            0, // account index
            255, 255, 255, 255, // impossibly large data length
        ];
        let result = SanitizedInstruction::deserialize(&invalid_data);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Insufficient bytes"));

        // Test truncated data
        let truncated_data = vec![
            0, // program_id_index
            1, 0, 0, 0, // 1 account
            0, // account index
            2, 0, 0, 0, // data length of 2
            1, // only 1 byte of data instead of 2
        ];
        let result = SanitizedInstruction::deserialize(&truncated_data);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Insufficient bytes"));
    }

    #[test]
    fn test_account_privileges_and_ordering() {
        let header = MessageHeader {
            num_required_signatures: 3,        // Total number of signers
            num_readonly_signed_accounts: 1,   // Last 1 signer is read-only
            num_readonly_unsigned_accounts: 2, // Last 2 non-signers are read-only
        };

        // Create accounts with different privileges
        let account_keys = vec![
            Pubkey::new_unique(), // Account 0: Writable and Signer
            Pubkey::new_unique(), // Account 1: Writable and Signer
            Pubkey::new_unique(), // Account 2: Read-only and Signer
            Pubkey::new_unique(), // Account 3: Writable and non-Signer
            Pubkey::new_unique(), // Account 4: Read-only and non-Signer
            Pubkey::new_unique(), // Account 5: Read-only and non-Signer
        ];

        let instruction = SanitizedInstruction {
            program_id_index: 3,           // Using a non-signer writable account as program
            accounts: vec![0, 1, 2, 4, 5], // Mix of different privilege accounts
            data: vec![1, 2, 3],
        };

        let message = ArchMessage {
            header,
            account_keys: account_keys.clone(),
            recent_blockhash: hex::encode([0; 32]),
            instructions: vec![instruction],
        };

        // Test account privileges
        assert!(
            message.is_writable_index(0),
            "First signer should be writable"
        );
        assert!(
            message.is_writable_index(1),
            "Second signer should be writable"
        );
        assert!(
            !message.is_writable_index(2),
            "Third signer should be read-only"
        );
        assert!(
            message.is_writable_index(3),
            "First non-signer should be writable"
        );
        assert!(
            !message.is_writable_index(4),
            "Second non-signer should be read-only"
        );
        assert!(
            !message.is_writable_index(5),
            "Third non-signer should be read-only"
        );

        // Test signer privileges
        assert!(message.is_signer(0), "Account 0 should be a signer");
        assert!(message.is_signer(1), "Account 1 should be a signer");
        assert!(message.is_signer(2), "Account 2 should be a signer");
        assert!(!message.is_signer(3), "Account 3 should not be a signer");
        assert!(!message.is_signer(4), "Account 4 should not be a signer");
        assert!(!message.is_signer(5), "Account 5 should not be a signer");

        // Test serialization/deserialization preserves privileges
        let serialized = message.serialize();
        let deserialized = ArchMessage::deserialize(&serialized).unwrap();

        // Verify header values are preserved
        assert_eq!(deserialized.header.num_required_signatures, 3);
        assert_eq!(deserialized.header.num_readonly_signed_accounts, 1);
        assert_eq!(deserialized.header.num_readonly_unsigned_accounts, 2);

        // Verify account ordering is preserved
        assert_eq!(deserialized.account_keys, account_keys);

        // Verify privileges are correctly interpreted in deserialized message
        for i in 0..6 {
            assert_eq!(
                message.is_writable_index(i),
                deserialized.is_writable_index(i),
                "Writable privilege mismatch for account {}",
                i
            );
            assert_eq!(
                message.is_signer(i),
                deserialized.is_signer(i),
                "Signer privilege mismatch for account {}",
                i
            );
        }
    }

    #[test]
    fn test_sanitized_message_privilege_cache() {
        let header = MessageHeader {
            num_required_signatures: 2,
            num_readonly_signed_accounts: 1,
            num_readonly_unsigned_accounts: 1,
        };

        let account_keys = vec![
            Pubkey::new_unique(), // Writable signer
            Pubkey::new_unique(), // Read-only signer
            Pubkey::new_unique(), // Writable non-signer
            Pubkey::new_unique(), // Read-only non-signer
        ];

        let message = ArchMessage {
            header,
            account_keys,
            recent_blockhash: hex::encode([0; 32]),
            instructions: vec![],
        };

        let sanitized_message = SanitizedMessage::new(message);

        // Test writable cache
        assert!(
            sanitized_message.is_writable(0),
            "First account should be writable"
        );
        assert!(
            !sanitized_message.is_writable(1),
            "Second account should be read-only"
        );
        assert!(
            sanitized_message.is_writable(2),
            "Third account should be writable"
        );
        assert!(
            !sanitized_message.is_writable(3),
            "Fourth account should be read-only"
        );

        // Test signer cache
        assert!(
            sanitized_message.is_signer(0),
            "First account should be signer"
        );
        assert!(
            sanitized_message.is_signer(1),
            "Second account should be signer"
        );
        assert!(
            !sanitized_message.is_signer(2),
            "Third account should not be signer"
        );
        assert!(
            !sanitized_message.is_signer(3),
            "Fourth account should not be signer"
        );
    }

    #[test]
    fn test_get_unique_account_keys() {
        let header = MessageHeader {
            num_required_signatures: 2,
            num_readonly_signed_accounts: 1,
            num_readonly_unsigned_accounts: 1,
        };

        let account_keys = vec![
            Pubkey::new_unique(), // 0: signer
            Pubkey::new_unique(), // 1: signer
            Pubkey::new_unique(), // 2: program
            Pubkey::new_unique(), // 3: unused account
            Pubkey::new_unique(), // 4: data account
        ];

        let instruction1 = SanitizedInstruction {
            program_id_index: 2,
            accounts: vec![0, 1, 4], // Using signers and data account
            data: vec![1, 2, 3],
        };

        let instruction2 = SanitizedInstruction {
            program_id_index: 2,  // Same program
            accounts: vec![1, 4], // Subset of accounts from instruction1
            data: vec![4, 5, 6],
        };

        let message = ArchMessage {
            header,
            account_keys: account_keys.clone(),
            recent_blockhash: hex::encode([0; 32]),
            instructions: vec![instruction1, instruction2],
        };

        let unique_keys = message.get_unique_instruction_account_keys();

        // Should contain: 2 signers +  1 data account = 3 unique keys
        assert_eq!(unique_keys.len(), 3);
        assert!(unique_keys.contains(&account_keys[0])); // signer 1
        assert!(unique_keys.contains(&account_keys[1])); // signer 2
        assert!(!unique_keys.contains(&account_keys[2])); // program
        assert!(!unique_keys.contains(&account_keys[3])); // unused account
        assert!(unique_keys.contains(&account_keys[4])); // data account
    }
}

#[cfg(test)]
mod sanitize_tests {
    use super::*;

    // Helper function to create a basic valid message
    fn create_valid_message() -> ArchMessage {
        ArchMessage {
            header: MessageHeader {
                num_required_signatures: 2,
                num_readonly_signed_accounts: 1,
                num_readonly_unsigned_accounts: 1,
            },
            account_keys: vec![
                Pubkey::new_unique(), // fee-payer (writable, signer)
                Pubkey::new_unique(), // readonly signer
                Pubkey::new_unique(), // program
                Pubkey::new_unique(), // writable non-signer
                Pubkey::new_unique(), // readonly non-signer
            ],
            recent_blockhash: hex::encode([0; 32]),
            instructions: vec![SanitizedInstruction {
                program_id_index: 2,
                accounts: vec![0, 1, 3, 4],
                data: vec![1, 2, 3],
            }],
        }
    }

    #[test]
    fn test_valid_message() {
        let message = create_valid_message();
        assert!(message.sanitize().is_ok());
    }

    #[test]
    fn test_overlapping_signing_and_readonly_areas() {
        let mut message = create_valid_message();
        // Set num_required_signatures + num_readonly_unsigned_accounts > account_keys.len()
        message.header.num_required_signatures = 3;
        message.header.num_readonly_unsigned_accounts = 3;

        assert_eq!(message.sanitize(), Err(SanitizeError::IndexOutOfBounds));
    }

    #[test]
    fn test_no_writable_fee_payer() {
        let mut message = create_valid_message();
        // Make all signed accounts readonly
        message.header.num_readonly_signed_accounts = message.header.num_required_signatures;

        assert_eq!(message.sanitize(), Err(SanitizeError::IndexOutOfBounds));
    }

    #[test]
    fn test_invalid_program_id_index() {
        let mut message = create_valid_message();
        message.instructions[0].program_id_index = message.account_keys.len() as u8;

        assert_eq!(message.sanitize(), Err(SanitizeError::IndexOutOfBounds));
    }

    #[test]
    fn test_program_as_payer() {
        let mut message = create_valid_message();
        message.instructions[0].program_id_index = 0;

        assert_eq!(message.sanitize(), Err(SanitizeError::IndexOutOfBounds));
    }

    #[test]
    fn test_invalid_account_index() {
        let mut message = create_valid_message();
        message.instructions[0]
            .accounts
            .push(message.account_keys.len() as u8);

        assert_eq!(message.sanitize(), Err(SanitizeError::IndexOutOfBounds));
    }

    #[test]
    fn test_complex_valid_message() {
        let message = ArchMessage {
            header: MessageHeader {
                num_required_signatures: 3,
                num_readonly_signed_accounts: 1,
                num_readonly_unsigned_accounts: 2,
            },
            account_keys: vec![
                Pubkey::new_unique(), // writable signer (fee-payer)
                Pubkey::new_unique(), // writable signer
                Pubkey::new_unique(), // readonly signer
                Pubkey::new_unique(), // program 1
                Pubkey::new_unique(), // program 2
                Pubkey::new_unique(), // writable non-signer
                Pubkey::new_unique(), // readonly non-signer
                Pubkey::new_unique(), // readonly non-signer
            ],
            recent_blockhash: hex::encode([0; 32]),
            instructions: vec![
                SanitizedInstruction {
                    program_id_index: 3,
                    accounts: vec![0, 1, 5],
                    data: vec![1, 2, 3],
                },
                SanitizedInstruction {
                    program_id_index: 4,
                    accounts: vec![1, 2, 6, 7],
                    data: vec![4, 5, 6],
                },
            ],
        };

        assert!(message.sanitize().is_ok());
    }

    #[test]
    fn test_duplicate_account_in_instr() {
        let message = ArchMessage {
            header: MessageHeader {
                num_required_signatures: 2,
                num_readonly_signed_accounts: 1,
                num_readonly_unsigned_accounts: 1,
            },
            account_keys: vec![
                Pubkey::new_unique(), // fee-payer (writable, signer)
                Pubkey::new_unique(), // readonly signer
                Pubkey::new_unique(), // program
                Pubkey::new_unique(), // writable non-signer
                Pubkey::new_unique(), // readonly non-signer
            ],
            recent_blockhash: hex::encode([0; 32]),
            instructions: vec![SanitizedInstruction {
                program_id_index: 2,
                accounts: vec![0, 1, 3, 3],
                data: vec![1, 2, 3],
            }],
        };
        // Duplicate index are allowed in instructions
        assert!(message.sanitize().is_ok());
    }

    #[test]
    fn test_duplicate_account_in_keys_list() {
        let malicious = Pubkey::new_unique();
        let message = ArchMessage {
            header: MessageHeader {
                num_required_signatures: 2,
                num_readonly_signed_accounts: 1,
                num_readonly_unsigned_accounts: 1,
            },
            account_keys: vec![
                Pubkey::new_unique(), // fee-payer (writable, signer)
                Pubkey::new_unique(), // readonly signer
                Pubkey::new_unique(), // program
                malicious,
                malicious,
            ],
            recent_blockhash: hex::encode([0; 32]),
            instructions: vec![SanitizedInstruction {
                program_id_index: 2,
                accounts: vec![0, 1, 3, 4],
                data: vec![1, 2, 3],
            }],
        };
        assert_eq!(
            message.sanitize().unwrap_err(),
            SanitizeError::DuplicateAccount
        );
    }
}
