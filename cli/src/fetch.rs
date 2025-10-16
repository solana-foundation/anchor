use crate::cluster_url;
use crate::config::{get_solana_cfg_url, Config, ConfigOverride};
use anchor_lang::idl::IdlAccount;
use anyhow::{anyhow, Result};
use flate2::read::ZlibDecoder;
use solana_rpc_client::rpc_client::RpcClient;
use solana_rpc_client_api::config::RpcTransactionConfig;
use solana_rpc_client_api::response::RpcConfirmedTransactionStatusWithSignature;
use solana_sdk::commitment_config::CommitmentConfig;
use solana_sdk::pubkey::Pubkey;
use solana_sdk::signature::Signature;
use solana_transaction_status_client_types::*;
use std::io::Read;
use std::str::FromStr;

// IDL Historical Fetch - Type Definitions and Constants
const FULL_CHUNK_THRESHOLD: usize = 1000;
const MAX_SLOT_GAP: u64 = 5;
const IDL_IX_TAG: [u8; 8] = [0x40, 0xf4, 0xbc, 0x78, 0xa7, 0xe9, 0x69, 0x0a];
const WRITE_VARIANT: u8 = 0x02;

type ChunkData = Vec<u8>;
type SlotChunk = (u64, ChunkData);
type SessionChunks = Vec<SlotChunk>;

pub struct IdlFetcher<'a> {
    client: &'a RpcClient,
}

impl<'a> IdlFetcher<'a> {
    fn new(client: &'a RpcClient) -> Self {
        Self { client }
    }

    fn validate_slot(&self, target_slot: u64) -> Result<()> {
        let current_slot = self.client.get_slot()?;
        if target_slot > current_slot {
            return Err(anyhow::format_err!(
                "Target slot {} is greater than the current slot {}. Cannot fetch IDL from a future slot.",
                target_slot,
                current_slot
            ));
        }
        Ok(())
    }

    fn collect_chunks(
        &self,
        signatures: &[&RpcConfirmedTransactionStatusWithSignature],
    ) -> Vec<SlotChunk> {
        signatures
            .iter()
            .filter_map(|sig| {
                let signature = Signature::from_str(&sig.signature).ok()?;
                let chunks = extract_chunks_from_transaction(self.client, &signature).ok()?;
                if chunks.is_empty() {
                    None
                } else {
                    Some(
                        chunks
                            .into_iter()
                            .map(|chunk| (sig.slot, chunk))
                            .collect::<Vec<_>>(),
                    )
                }
            })
            .flatten()
            .collect()
    }

    fn collect_chunks_owned(
        &self,
        signatures: &[RpcConfirmedTransactionStatusWithSignature],
    ) -> Vec<SlotChunk> {
        let refs: Vec<&RpcConfirmedTransactionStatusWithSignature> = signatures.iter().collect();
        self.collect_chunks(&refs)
    }

    fn scan_backwards(
        &self,
        signatures: &[&RpcConfirmedTransactionStatusWithSignature],
        target_slot: u64,
    ) -> SessionChunks {
        let mut chunks = Vec::new();
        let mut found_data = false;

        for sig in signatures.iter().rev() {
            let slot_gap = target_slot.saturating_sub(sig.slot);
            if slot_gap > MAX_SLOT_GAP {
                println!(
                    "Stopped backward scan: slot gap {} > {}",
                    slot_gap, MAX_SLOT_GAP
                );
                break;
            }

            if let Ok(signature) = Signature::from_str(&sig.signature) {
                if let Ok(extracted) = extract_chunks_from_transaction(self.client, &signature) {
                    for chunk in extracted {
                        if chunk.is_empty() {
                            // 0-byte chunk
                            if found_data {
                                return chunks;
                            }
                            continue;
                        } else if chunk.len() >= FULL_CHUNK_THRESHOLD {
                            chunks.insert(0, (sig.slot, chunk));
                            found_data = true;
                        } else {
                            return chunks;
                        }
                    }
                }
            }
        }
        chunks
    }

    fn scan_forwards(
        &self,
        signatures: &[&RpcConfirmedTransactionStatusWithSignature],
        target_slot: u64,
    ) -> SessionChunks {
        let mut chunks = Vec::new();
        let mut last_checked_slot = target_slot;

        for sig in signatures {
            let slot_gap = sig.slot.saturating_sub(last_checked_slot);
            if slot_gap > MAX_SLOT_GAP {
                break;
            }

            if let Ok(signature) = Signature::from_str(&sig.signature) {
                if let Ok(extracted) = extract_chunks_from_transaction(self.client, &signature) {
                    for chunk in extracted {
                        if chunk.is_empty() {
                            // 0-byte chunk indicates session boundary
                            if !chunks.is_empty() {
                                return chunks;
                            }
                            // Skip if we haven't collected any chunks yet
                            continue;
                        }

                        chunks.push((sig.slot, chunk.clone()));

                        if chunk.len() < FULL_CHUNK_THRESHOLD {
                            return chunks;
                        }
                    }
                }
            }

            last_checked_slot = sig.slot;
        }
        chunks
    }
}

fn parse_date_to_timestamp(date_str: &str) -> Result<i64> {
    use chrono::NaiveDate;

    let date = NaiveDate::parse_from_str(date_str, "%Y-%m-%d").map_err(|e| {
        anyhow!(
            "Invalid date format '{}'. Expected YYYY-MM-DD: {}",
            date_str,
            e
        )
    })?;

    let datetime = date
        .and_hms_opt(0, 0, 0)
        .ok_or_else(|| anyhow!("Failed to create datetime from date"))?;

    Ok(datetime.and_utc().timestamp())
}

fn group_chunks_into_sessions(all_chunks: &[SlotChunk]) -> Vec<SessionChunks> {
    let mut upload_sessions = Vec::new();
    let mut processed = vec![false; all_chunks.len()];

    for i in 0..all_chunks.len() {
        if processed[i] {
            continue;
        }

        let start_idx = find_session_start(all_chunks, &processed, i);
        let session = collect_session_forward(all_chunks, &mut processed, start_idx);

        if !session.is_empty() {
            upload_sessions.push(session);
        }
    }

    upload_sessions
}

fn find_session_start(chunks: &[SlotChunk], processed: &[bool], start: usize) -> usize {
    let mut idx = start;
    while idx > 0 {
        let prev_idx = idx - 1;
        if processed[prev_idx] {
            break;
        }

        let slot_gap = chunks[idx].0.saturating_sub(chunks[prev_idx].0);
        let prev_is_full = chunks[prev_idx].1.len() >= FULL_CHUNK_THRESHOLD;

        if prev_is_full && slot_gap <= MAX_SLOT_GAP {
            idx = prev_idx;
        } else {
            break;
        }
    }
    idx
}

fn collect_session_forward(
    chunks: &[SlotChunk],
    processed: &mut [bool],
    start: usize,
) -> SessionChunks {
    let mut session = Vec::new();
    let mut idx = start;

    while idx < chunks.len() && !processed[idx] {
        let chunk = &chunks[idx];
        session.push(chunk.clone());
        processed[idx] = true;

        if chunk.1.len() < FULL_CHUNK_THRESHOLD {
            break;
        }

        if idx + 1 < chunks.len() {
            let slot_gap = chunks[idx + 1].0.saturating_sub(chunk.0);
            if slot_gap > MAX_SLOT_GAP {
                break;
            }
        }

        idx += 1;
    }

    session
}

pub fn idl_fetch_at_slot(
    client: &RpcClient,
    all_signatures: &[RpcConfirmedTransactionStatusWithSignature],
    target_slot: u64,
    _out_dir: Option<String>,
    out: Option<String>,
) -> Result<()> {
    let fetcher = IdlFetcher::new(client);

    let (before_target, at_target, after_target) =
        partition_signatures_by_slot(all_signatures, target_slot);
    let session_chunks = reconstruct_session_at_slot(
        &fetcher,
        &before_target,
        &at_target,
        &after_target,
        target_slot,
    )?;

    if session_chunks.is_empty() {
        println!(
            "\nFailed to reconstruct any IDL session at or before slot {}",
            target_slot
        );
        return Ok(());
    }

    let combined_data = combine_chunks(&session_chunks);
    let idl_data = decompress_and_validate(&combined_data)?;
    output_idl_data(&idl_data, target_slot, out)
}

fn partition_signatures_by_slot(
    signatures: &[RpcConfirmedTransactionStatusWithSignature],
    target_slot: u64,
) -> (
    Vec<&RpcConfirmedTransactionStatusWithSignature>,
    Vec<&RpcConfirmedTransactionStatusWithSignature>,
    Vec<&RpcConfirmedTransactionStatusWithSignature>,
) {
    let mut before: Vec<_> = signatures
        .iter()
        .filter(|sig| sig.slot < target_slot)
        .collect();
    let at: Vec<_> = signatures
        .iter()
        .filter(|sig| sig.slot == target_slot)
        .collect();
    let mut after: Vec<_> = signatures
        .iter()
        .filter(|sig| sig.slot > target_slot)
        .collect();

    before.sort_by_key(|sig| sig.slot);
    after.sort_by_key(|sig| sig.slot);

    (before, at, after)
}

fn reconstruct_session_at_slot(
    fetcher: &IdlFetcher,
    before_target: &[&RpcConfirmedTransactionStatusWithSignature],
    at_target: &[&RpcConfirmedTransactionStatusWithSignature],
    after_target: &[&RpcConfirmedTransactionStatusWithSignature],
    target_slot: u64,
) -> Result<SessionChunks> {
    let mut session_chunks = fetcher.collect_chunks(at_target);

    if session_chunks.is_empty() {
        let all_chunks = fetcher.collect_chunks(before_target);
        if all_chunks.is_empty() {
            println!(
                "No IDL Write transactions found before slot {}",
                target_slot
            );
            return Ok(Vec::new());
        }

        let sessions = group_chunks_into_sessions(&all_chunks);
        if sessions.is_empty() {
            println!("No complete IDL sessions found");
            return Ok(Vec::new());
        }

        return Ok(sessions.last().unwrap().clone());
    }

    let has_full_chunk = session_chunks
        .iter()
        .any(|(_, data)| data.len() == FULL_CHUNK_THRESHOLD);
    let has_partial_chunk = session_chunks
        .iter()
        .any(|(_, data)| data.len() < FULL_CHUNK_THRESHOLD);

    let backward_chunks = fetcher.scan_backwards(before_target, target_slot);
    for chunk in backward_chunks.into_iter().rev() {
        session_chunks.insert(0, chunk);
    }

    if has_full_chunk && !has_partial_chunk {
        let forward_chunks = fetcher.scan_forwards(after_target, target_slot);
        session_chunks.extend(forward_chunks);
    }

    Ok(session_chunks)
}

fn combine_chunks(chunks: &[SlotChunk]) -> Vec<u8> {
    chunks
        .iter()
        .flat_map(|(_, chunk)| chunk.iter())
        .copied()
        .collect()
}

fn decompress_and_validate(compressed_data: &[u8]) -> Result<Vec<u8>> {
    match decompress_idl_data(compressed_data) {
        Ok(Some(idl_data)) => {
            println!("Successfully decompressed IDL ({} bytes)", idl_data.len());
            Ok(idl_data)
        }
        Ok(None) => Err(anyhow!("Failed to decompress IDL")),
        Err(e) => Err(e),
    }
}

fn output_idl_data(idl_data: &[u8], slot: u64, out: Option<String>) -> Result<()> {
    if let Some(out_file) = out {
        std::fs::write(&out_file, idl_data)?;
        println!("Saved IDL to: {}", out_file);
    } else {
        println!("\nIDL at slot {}:", slot);
        println!("{}", String::from_utf8_lossy(idl_data));
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
pub fn idl_fetch_historical(
    cfg_override: &ConfigOverride,
    address: Pubkey,
    _all: bool,
    slot: Option<u64>,
    before: Option<String>,
    after: Option<String>,
    out_dir: Option<String>,
    out: Option<String>,
) -> Result<()> {
    let client = create_rpc_client(cfg_override)?;
    let fetcher = IdlFetcher::new(&client);

    let signatures = fetch_idl_signatures(&client, &address)?;
    if signatures.is_empty() {
        println!("The program doesn't have an IDL account");
        return Ok(());
    }

    if let Some(target_slot) = slot {
        fetcher.validate_slot(target_slot)?;
        return idl_fetch_at_slot(&client, &signatures, target_slot, out_dir, out);
    }

    let filtered_signatures = apply_date_filters(signatures, before, after)?;
    if filtered_signatures.is_empty() {
        return Ok(());
    }

    let all_chunks = collect_and_process_chunks(&fetcher, &filtered_signatures);
    if all_chunks.is_empty() {
        return Ok(());
    }

    let sessions = group_chunks_into_sessions(&all_chunks);
    let extracted_idls = decompress_sessions(&sessions, &filtered_signatures)?;

    if extracted_idls.is_empty() {
        println!("\nNo IDL data could be fetched from historical slots.");
        return Ok(());
    }

    println!(
        "\nSuccessfully extracted {} IDL version(s)",
        extracted_idls.len()
    );
    output_idls(extracted_idls, out_dir, out)
}

fn create_rpc_client(cfg_override: &ConfigOverride) -> Result<RpcClient> {
    let url = match Config::discover(cfg_override)? {
        Some(cfg) => cluster_url(&cfg, &cfg.test_validator),
        None => {
            if let Some(cluster) = cfg_override.cluster.as_ref() {
                cluster.url().to_string()
            } else {
                get_solana_cfg_url()?
            }
        }
    };
    Ok(crate::create_client(url))
}

fn fetch_idl_signatures(
    client: &RpcClient,
    address: &Pubkey,
) -> Result<Vec<RpcConfirmedTransactionStatusWithSignature>> {
    let idl_account_address = IdlAccount::address(address);
    Ok(client.get_signatures_for_address(&idl_account_address)?)
}

fn apply_date_filters(
    mut signatures: Vec<RpcConfirmedTransactionStatusWithSignature>,
    before: Option<String>,
    after: Option<String>,
) -> Result<Vec<RpcConfirmedTransactionStatusWithSignature>> {
    if let Some(before_date) = before {
        let before_timestamp = parse_date_to_timestamp(&before_date)?;
        signatures.retain(|sig| sig.block_time.is_some_and(|bt| bt <= before_timestamp));
    }

    if let Some(after_date) = after {
        let after_timestamp = parse_date_to_timestamp(&after_date)?;
        signatures.retain(|sig| sig.block_time.is_some_and(|bt| bt >= after_timestamp));
    }

    Ok(signatures)
}

fn collect_and_process_chunks(
    fetcher: &IdlFetcher,
    signatures: &[RpcConfirmedTransactionStatusWithSignature],
) -> Vec<SlotChunk> {
    let mut all_chunks = fetcher.collect_chunks_owned(signatures);
    all_chunks.sort_by_key(|(slot, _)| *slot);
    all_chunks
}

fn decompress_sessions(
    sessions: &[SessionChunks],
    signatures: &[RpcConfirmedTransactionStatusWithSignature],
) -> Result<Vec<(RpcConfirmedTransactionStatusWithSignature, Vec<u8>)>> {
    let extracted = sessions
        .iter()
        .filter_map(|session| {
            let combined_data = combine_chunks(session);
            match decompress_idl_data(&combined_data) {
                Ok(Some(idl_data)) => {
                    let session_sig = signatures
                        .iter()
                        .find(|sig| sig.slot == session.first().unwrap().0)
                        .unwrap_or(&signatures[0]);
                    Some((session_sig.clone(), idl_data))
                }
                Ok(None) => {
                    println!("Decompression failed for this session");
                    None
                }
                Err(e) => {
                    println!("Error: {}", e);
                    None
                }
            }
        })
        .collect();
    Ok(extracted)
}

fn output_idls(
    idls: Vec<(RpcConfirmedTransactionStatusWithSignature, Vec<u8>)>,
    out_dir: Option<String>,
    out: Option<String>,
) -> Result<()> {
    if let Some(out_dir) = out_dir {
        std::fs::create_dir_all(&out_dir)?;
        for (i, (sig, idl_data)) in idls.iter().enumerate() {
            let filename = format!("{}/idl_v{}_{}.json", out_dir, i + 1, sig.slot);
            std::fs::write(&filename, idl_data)?;
            println!("Saved IDL to: {}", filename);
        }
    } else if let Some(out_file) = out {
        let (sig, idl_data) = idls.last().unwrap();
        std::fs::write(&out_file, idl_data)?;
        println!("Saved IDL (slot {}) to: {}", sig.slot, out_file);
    } else {
        let (sig, idl_data) = idls.last().unwrap();
        println!("\nIDL at slot {}:", sig.slot);
        let idl: serde_json::Value = serde_json::from_slice(idl_data)?;
        println!("{}", serde_json::to_string_pretty(&idl)?);
    }

    Ok(())
}

fn extract_chunks_from_transaction(
    client: &RpcClient,
    signature: &Signature,
) -> Result<Vec<ChunkData>> {
    let transaction = fetch_transaction(client, signature)?;
    let ui_tx = parse_transaction_data(transaction)?;
    extract_chunks_from_message(ui_tx.message)
}

fn fetch_transaction(
    client: &RpcClient,
    signature: &Signature,
) -> Result<EncodedConfirmedTransactionWithStatusMeta> {
    let config = RpcTransactionConfig {
        encoding: Some(UiTransactionEncoding::Json),
        commitment: Some(CommitmentConfig::confirmed()),
        max_supported_transaction_version: Some(0),
    };

    client
        .get_transaction_with_config(signature, config)
        .map_err(|e| {
            println!("Failed to fetch transaction: {}", e);
            anyhow!("Transaction fetch failed")
        })
}

fn parse_transaction_data(
    transaction: EncodedConfirmedTransactionWithStatusMeta,
) -> Result<UiTransaction> {
    match transaction.transaction {
        EncodedTransactionWithStatusMeta {
            transaction: EncodedTransaction::Json(ui_tx),
            ..
        } => Ok(ui_tx),
        _ => {
            println!("Transaction not in JSON format");
            Err(anyhow!("Invalid transaction format"))
        }
    }
}

fn extract_chunks_from_message(message: UiMessage) -> Result<Vec<ChunkData>> {
    let chunks = match message {
        UiMessage::Parsed(parsed_msg) => {
            extract_from_parsed_instructions(&parsed_msg.instructions)?
        }
        UiMessage::Raw(raw_msg) => extract_from_raw_instructions(&raw_msg.instructions)?,
    };
    Ok(chunks)
}

fn extract_from_parsed_instructions(instructions: &[UiInstruction]) -> Result<Vec<ChunkData>> {
    let chunks = instructions
        .iter()
        .filter_map(|instruction| {
            if let UiInstruction::Compiled(UiCompiledInstruction { data, .. }) = instruction {
                extract_compressed_chunk(data).ok().flatten()
            } else {
                None
            }
        })
        .collect();
    Ok(chunks)
}

fn extract_from_raw_instructions(instructions: &[UiCompiledInstruction]) -> Result<Vec<ChunkData>> {
    let chunks = instructions
        .iter()
        .filter_map(|instruction| extract_compressed_chunk(&instruction.data).ok().flatten())
        .collect();
    Ok(chunks)
}

fn decompress_idl_data(compressed_data: &[u8]) -> Result<Option<Vec<u8>>> {
    const ZLIB_HEADER: u8 = 0x78;

    if compressed_data.is_empty() || compressed_data.first() != Some(&ZLIB_HEADER) {
        return Ok(None);
    }

    let mut decoder = ZlibDecoder::new(compressed_data);
    let mut decompressed = Vec::new();

    decoder
        .read_to_end(&mut decompressed)
        .map(|_| Some(decompressed))
        .or(Ok(None))
}

fn extract_compressed_chunk(data_str: &str) -> Result<Option<ChunkData>> {
    const IDL_HEADER_SIZE: usize = 13;

    let data = bs58::decode(data_str).into_vec()?;

    if !is_valid_idl_write_instruction(&data) {
        return Ok(None);
    }

    let vec_len = extract_payload_length(&data);

    if !has_complete_payload(&data, vec_len) {
        println!(
            "Incomplete data: expected {} bytes, got {}",
            IDL_HEADER_SIZE + vec_len,
            data.len()
        );
        return Ok(None);
    }

    Ok(Some(
        data[IDL_HEADER_SIZE..IDL_HEADER_SIZE + vec_len].to_vec(),
    ))
}

fn is_valid_idl_write_instruction(data: &[u8]) -> bool {
    if data.len() < 13 {
        return false;
    }

    if data[0..8] != IDL_IX_TAG {
        println!("Not an IDL instruction (tag mismatch)");
        return false;
    }

    if data[8] != WRITE_VARIANT {
        return false;
    }

    true
}

fn extract_payload_length(data: &[u8]) -> usize {
    u32::from_le_bytes([data[9], data[10], data[11], data[12]]) as usize
}

fn has_complete_payload(data: &[u8], payload_len: usize) -> bool {
    data.len() >= 13 + payload_len
}
