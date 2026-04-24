use {
    crate::{
        cluster_url,
        config::{get_solana_cfg_url, Config, ConfigOverride},
    },
    anyhow::{anyhow, Result},
    flate2::read::ZlibDecoder,
    indicatif::{ProgressBar, ProgressStyle},
    solana_commitment_config::CommitmentConfig,
    solana_pubkey::Pubkey,
    solana_rpc_client::rpc_client::{GetConfirmedSignaturesForAddress2Config, RpcClient},
    solana_rpc_client_api::{
        client_error::{reqwest::StatusCode, ErrorKind as RpcClientErrorKind},
        config::RpcTransactionConfig,
        response::RpcConfirmedTransactionStatusWithSignature,
    },
    solana_signature::Signature,
    solana_transaction_status_client_types::*,
    std::{
        io::Read,
        path::{Path, PathBuf},
        str::FromStr,
        thread,
        time::Duration,
    },
};

// IDL Historical Fetch - Type Definitions and Constants
const IDL_IX_TAG: [u8; 8] = [0x40, 0xf4, 0xbc, 0x78, 0xa7, 0xe9, 0x69, 0x0a];
const WRITE_VARIANT: u8 = 0x02;
const DEFAULT_PARALLEL_FETCH_SIGNATURE_THRESHOLD: usize = 10;
const DEFAULT_MAX_PARALLEL_FETCH_WORKERS: usize = 4;
const DEFAULT_MAX_RETRIES: u32 = 5;
const DEFAULT_RETRY_BACKOFF_MS: u64 = 500;
const PROGRESS_TICK_INTERVAL_MS: u64 = 80;

type ChunkData = Vec<u8>;
type SlotChunk = (u64, ChunkData);
type SessionChunks = Vec<SlotChunk>;

#[derive(Clone, Copy, Debug)]
pub struct FetchTuning {
    pub workers: Option<usize>,
    pub no_parallel: bool,
    pub max_retries: u32,
    pub retry_backoff_ms: u64,
    pub verbose: bool,
}

impl Default for FetchTuning {
    fn default() -> Self {
        Self {
            workers: None,
            no_parallel: false,
            max_retries: DEFAULT_MAX_RETRIES,
            retry_backoff_ms: DEFAULT_RETRY_BACKOFF_MS,
            verbose: false,
        }
    }
}

pub struct IdlFetcher<'a> {
    client: &'a RpcClient,
    tuning: FetchTuning,
}

impl<'a> IdlFetcher<'a> {
    fn new(client: &'a RpcClient, tuning: FetchTuning) -> Self {
        Self { client, tuning }
    }

    /// Validates that `target_slot` is not a future slot by comparing it to
    /// the current client slot.
    fn validate_slot(&self, target_slot: u64) -> Result<()> {
        let current_slot = self.client.get_slot()?;
        if target_slot > current_slot {
            return Err(anyhow::format_err!(
                "Target slot {} is greater than the current slot {}. Cannot fetch IDL from a \
                 future slot.",
                target_slot,
                current_slot
            ));
        }
        Ok(())
    }

    fn collect_chunks(
        &self,
        signatures: &[&RpcConfirmedTransactionStatusWithSignature],
        pb: &ProgressBar,
    ) -> Vec<SlotChunk> {
        signatures
            .iter()
            .filter_map(|sig| {
                pb.inc(1);
                collect_signature_chunks(self.client, sig, &self.tuning, pb)
            })
            .flatten()
            .collect()
    }

    fn collect_chunks_owned(
        &self,
        signatures: &[RpcConfirmedTransactionStatusWithSignature],
        pb: &ProgressBar,
    ) -> Vec<SlotChunk> {
        if should_parallelize_historical_fetch(signatures.len(), &self.tuning) {
            return self.collect_chunks_owned_parallel(signatures, pb);
        }

        let refs: Vec<&RpcConfirmedTransactionStatusWithSignature> = signatures.iter().collect();
        self.collect_chunks(&refs, pb)
    }

    fn collect_chunks_owned_parallel(
        &self,
        signatures: &[RpcConfirmedTransactionStatusWithSignature],
        pb: &ProgressBar,
    ) -> Vec<SlotChunk> {
        let worker_count = historical_fetch_worker_count(signatures.len(), &self.tuning);
        if worker_count <= 1 {
            let refs: Vec<&RpcConfirmedTransactionStatusWithSignature> =
                signatures.iter().collect();
            return self.collect_chunks(&refs, pb);
        }

        let chunk_size = signatures.len().div_ceil(worker_count);

        thread::scope(|scope| {
            let mut handles = Vec::new();

            for signature_chunk in signatures.chunks(chunk_size) {
                let progress = pb.clone();
                handles.push(scope.spawn(move || {
                    signature_chunk
                        .iter()
                        .filter_map(|sig| {
                            progress.inc(1);
                            collect_signature_chunks(self.client, sig, &self.tuning, &progress)
                        })
                        .flatten()
                        .collect::<Vec<_>>()
                }));
            }

            handles
                .into_iter()
                .flat_map(|handle| handle.join().expect("IDL fetch worker panicked"))
                .collect()
        })
    }
}

fn collect_signature_chunks(
    client: &RpcClient,
    sig: &RpcConfirmedTransactionStatusWithSignature,
    tuning: &FetchTuning,
    pb: &ProgressBar,
) -> Option<Vec<SlotChunk>> {
    let signature = Signature::from_str(&sig.signature).ok()?;
    let chunks = match extract_chunks_from_transaction(client, &signature, tuning) {
        Ok(chunks) => chunks,
        // Route through the progress bar to avoid mangling its rendering with
        // interleaved stdout writes.
        Err(e) => {
            pb.println(format!("{e}"));
            return None;
        }
    };

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
}

fn should_parallelize_historical_fetch(signature_count: usize, tuning: &FetchTuning) -> bool {
    if tuning.no_parallel {
        return false;
    }
    if matches!(tuning.workers, Some(1)) {
        return false;
    }
    signature_count > DEFAULT_PARALLEL_FETCH_SIGNATURE_THRESHOLD
}

fn historical_fetch_worker_count(signature_count: usize, tuning: &FetchTuning) -> usize {
    if !should_parallelize_historical_fetch(signature_count, tuning) {
        return 1;
    }

    let available = thread::available_parallelism()
        .map(|count| count.get())
        .unwrap_or(DEFAULT_MAX_PARALLEL_FETCH_WORKERS);
    let cap = tuning.workers.unwrap_or(DEFAULT_MAX_PARALLEL_FETCH_WORKERS);

    signature_count.min(available).min(cap).max(1)
}

/// Parses a `%Y-%m-%d` date into a UTC timestamp
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

// Session boundary detection combines two signals:
//   1. Chunk-size progression. Within a single upload, every Write chunk uses
//      the same payload size except the final (terminator) chunk, which is
//      strictly smaller. A size increase, or a chunk after a terminator, marks
//      a new upload.
//   2. Slot gap. Anchor IDL uploads are continuous bursts (adjacent chunks
//      land within a few seconds). A long idle gap between chunks means two
//      different uploads even if their chunk sizes happen to match.
const SESSION_SLOT_GAP_THRESHOLD: u64 = 5_000;

fn group_chunks_into_sessions(all_chunks: &[SlotChunk]) -> Vec<SessionChunks> {
    let mut sessions: Vec<SessionChunks> = Vec::new();
    let mut current: SessionChunks = Vec::new();
    let mut terminator_seen = false;

    for chunk in all_chunks {
        let size = chunk.1.len();
        let last = current.last();
        let prev_size = last.map(|(_, data)| data.len());
        let prev_slot = last.map(|(slot, _)| *slot);

        let slot_gap_break = matches!(
            prev_slot,
            Some(prev) if chunk.0.saturating_sub(prev) > SESSION_SLOT_GAP_THRESHOLD
        );

        let start_new = slot_gap_break
            || match prev_size {
                Some(prev) => terminator_seen || size > prev,
                None => false,
            };

        if start_new {
            sessions.push(std::mem::take(&mut current));
            terminator_seen = false;
        }

        if let Some(prev) = prev_size {
            if !start_new && size < prev {
                terminator_seen = true;
            }
        }

        current.push(chunk.clone());
    }

    if !current.is_empty() {
        sessions.push(current);
    }

    sessions
}

pub fn idl_fetch_at_slot(
    client: &RpcClient,
    all_signatures: &[RpcConfirmedTransactionStatusWithSignature],
    target_slot: u64,
    out_dir: Option<String>,
    tuning: FetchTuning,
) -> Result<()> {
    let fetcher = IdlFetcher::new(client, tuning);

    let pb = ProgressBar::new(all_signatures.len() as u64);
    pb.enable_steady_tick(Duration::from_millis(PROGRESS_TICK_INTERVAL_MS));
    pb.set_style(
        ProgressStyle::default_bar()
            .template(
                "{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {pos}/{len} \
                 transactions ({eta})",
            )
            .unwrap()
            .progress_chars("#>-"),
    );
    pb.set_message("Processing transactions...");

    let all_chunks = collect_and_process_chunks(&fetcher, all_signatures, &pb);
    pb.finish_with_message("Transaction processing complete");

    if all_chunks.is_empty() {
        println!("\nNo IDL chunks found in transactions");
        return Ok(());
    }

    let sessions = group_chunks_into_sessions(&all_chunks);
    // Candidate sessions that finished on or before target_slot, newest first.
    // The session-grouping heuristic can mis-split on chunk-size anomalies or
    // include duplicated retries, so the nominal "latest completed" session is
    // not guaranteed to decompress. Try each candidate until one succeeds —
    // that's the IDL actually visible at target_slot.
    let mut candidates: Vec<SessionChunks> = sessions
        .into_iter()
        .filter(|session| {
            session
                .last()
                .map(|(slot, _)| *slot <= target_slot)
                .unwrap_or(false)
        })
        .collect();
    candidates
        .sort_by_key(|session| std::cmp::Reverse(session.last().map(|(s, _)| *s).unwrap_or(0)));

    if candidates.is_empty() {
        println!(
            "\nNo completed IDL upload session at or before slot {}.",
            target_slot
        );
        return Ok(());
    }

    let total = candidates.len();
    for (idx, session) in candidates.into_iter().enumerate() {
        let combined = combine_chunks(&session);
        match decompress_idl_data(&combined) {
            Ok(Some(idl_data)) => {
                let last_slot = session.last().map(|(s, _)| *s).unwrap_or(target_slot);
                println!(
                    "Decompressed IDL from session ending at slot {} ({} bytes)",
                    last_slot,
                    idl_data.len()
                );
                return output_idl_data(&idl_data, target_slot, out_dir);
            }
            Ok(None) | Err(_) => {
                if idx + 1 < total {
                    continue;
                }
            }
        }
    }

    Err(anyhow!(
        "No decompressable IDL session found at or before slot {}. Try --all to see all \
         recoverable versions.",
        target_slot
    ))
}

fn combine_chunks(chunks: &[SlotChunk]) -> Vec<u8> {
    chunks
        .iter()
        .flat_map(|(_, chunk)| chunk.iter())
        .copied()
        .collect()
}

fn output_idl_data(idl_data: &[u8], slot: u64, out_dir: Option<String>) -> Result<()> {
    let out_dir = resolve_idl_output_dir(out_dir.as_deref())?;
    std::fs::create_dir_all(&out_dir)?;
    output_idl(idl_data, &single_idl_output_path(slot, &out_dir))?;
    Ok(())
}

#[allow(clippy::too_many_arguments)]
pub fn idl_fetch_historical(
    cfg_override: &ConfigOverride,
    address: Pubkey,
    all: bool,
    slot: Option<u64>,
    before: Option<String>,
    after: Option<String>,
    out_dir: Option<String>,
    tuning: FetchTuning,
) -> Result<()> {
    // Parse and validate the date range up-front so bad input fails before we
    // touch the RPC.
    let before_timestamp = before.as_deref().map(parse_date_to_timestamp).transpose()?;
    let after_timestamp = after.as_deref().map(parse_date_to_timestamp).transpose()?;
    if let (Some(after_ts), Some(before_ts)) = (after_timestamp, before_timestamp) {
        if after_ts > before_ts {
            return Err(anyhow!(
                "Invalid date range: --after ({}) must be on or before --before ({})",
                after
                    .as_deref()
                    .expect("after_timestamp is Some implies --after was provided"),
                before
                    .as_deref()
                    .expect("before_timestamp is Some implies --before was provided"),
            ));
        }
    }

    let client = create_rpc_client(cfg_override)?;
    let fetcher = IdlFetcher::new(&client, tuning);

    // Date filters are pushed into the pagination loop so we can stop fetching
    // once we cross the `after` bound. `--all` and `--slot` bypass filtering
    // and walk the full history.
    let (filter_before, filter_after) = if all || slot.is_some() {
        (None, None)
    } else {
        (before_timestamp, after_timestamp)
    };

    let signatures = fetch_idl_signatures(&client, &address, filter_before, filter_after)?;
    if signatures.is_empty() {
        println!("The program doesn't have an IDL account");
        return Ok(());
    }
    if tuning.verbose {
        println!("Found {} transactions on the IDL account", signatures.len());
    }

    if let Some(target_slot) = slot {
        fetcher.validate_slot(target_slot)?;
        return idl_fetch_at_slot(&client, &signatures, target_slot, out_dir, tuning);
    }

    if tuning.verbose {
        println!(
            "Processing {} transactions on the IDL account...",
            signatures.len()
        );
    }

    let pb = ProgressBar::new(signatures.len() as u64);
    pb.enable_steady_tick(Duration::from_millis(PROGRESS_TICK_INTERVAL_MS));
    pb.set_style(
        ProgressStyle::default_bar()
            .template(
                "{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {pos}/{len} \
                 transactions ({eta})",
            )
            .unwrap()
            .progress_chars("#>-"),
    );
    pb.set_message("Extracting IDL chunks from transactions...");

    let all_chunks = collect_and_process_chunks(&fetcher, &signatures, &pb);

    pb.finish_with_message("Transaction processing complete");

    if all_chunks.is_empty() {
        println!("\nNo IDL chunks found in transactions");
        return Ok(());
    }

    if tuning.verbose {
        println!("Grouping {} chunks into sessions...", all_chunks.len());
    }
    let sessions = group_chunks_into_sessions(&all_chunks);
    if tuning.verbose {
        println!("Found {} IDL session(s)", sessions.len());
        println!("Decompressing IDL data...");
    }
    let decompress_pb = ProgressBar::new(sessions.len() as u64);
    decompress_pb.enable_steady_tick(Duration::from_millis(PROGRESS_TICK_INTERVAL_MS));
    decompress_pb.set_style(
        ProgressStyle::default_bar()
            .template(
                "{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {pos}/{len} sessions",
            )
            .unwrap()
            .progress_chars("#>-"),
    );

    let (extracted_idls, skipped) = decompress_sessions(&sessions, &signatures, &decompress_pb)?;
    decompress_pb.finish_with_message("Decompression complete");

    if skipped > 0 {
        println!(
            "Skipped {}/{} session(s): no zlib streams found (partial uploads)",
            skipped,
            sessions.len()
        );
    }

    if extracted_idls.is_empty() {
        println!("\nNo IDL data could be fetched from historical slots.");
        return Ok(());
    }

    println!(
        "\nSuccessfully extracted {} IDL version(s)",
        extracted_idls.len()
    );
    output_idls(extracted_idls, out_dir)
}

fn create_rpc_client(cfg_override: &ConfigOverride) -> Result<RpcClient> {
    let url = match Config::discover(cfg_override)? {
        Some(cfg) => cluster_url(&cfg, &cfg.test_validator, &cfg.surfpool_config),
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
    before_timestamp: Option<i64>,
    after_timestamp: Option<i64>,
) -> Result<Vec<RpcConfirmedTransactionStatusWithSignature>> {
    let program_signer = Pubkey::find_program_address(&[], address).0;
    let idl_account_address = Pubkey::create_with_seed(&program_signer, "anchor:idl", address)
        .map_err(|e| anyhow!("Failed to derive IDL account address: {e}"))?;

    // `get_signatures_for_address` caps each response at 1000 entries. Walk the
    // full history by paginating with a `before` cursor set to the oldest
    // signature we've seen so far.
    let mut signatures = Vec::new();
    let mut cursor: Option<Signature> = None;

    loop {
        let config = GetConfirmedSignaturesForAddress2Config {
            before: cursor,
            until: None,
            limit: None,
            commitment: None,
        };
        let page = client.get_signatures_for_address_with_config(&idl_account_address, config)?;
        if page.is_empty() {
            break;
        }

        let next_cursor = page
            .last()
            .and_then(|sig| Signature::from_str(&sig.signature).ok());

        // Pages arrive newest-first. Once we've paged past the `after` bound,
        // every subsequent page is entirely out of range and we can stop.
        let has_date_filter = before_timestamp.is_some() || after_timestamp.is_some();
        let mut crossed_after_bound = false;
        for sig in page {
            // Failed transactions land on-chain but do not mutate the IDL
            // buffer. Including their Write payloads would duplicate bytes in
            // the concatenated stream and break zlib decompression.
            if sig.err.is_some() {
                continue;
            }
            if has_date_filter {
                // With a date filter active, a signature without a block time
                // can't be placed in the range, so drop it — matches the prior
                // `retain(|sig| sig.block_time.is_some_and(...))` behavior.
                let Some(bt) = sig.block_time else { continue };
                if before_timestamp.is_some_and(|ts| bt > ts) {
                    continue;
                }
                if after_timestamp.is_some_and(|ts| bt < ts) {
                    crossed_after_bound = true;
                    continue;
                }
            }
            signatures.push(sig);
        }

        if crossed_after_bound {
            break;
        }
        match next_cursor {
            Some(sig) => cursor = Some(sig),
            None => break,
        }
    }

    Ok(signatures)
}

fn collect_and_process_chunks(
    fetcher: &IdlFetcher,
    signatures: &[RpcConfirmedTransactionStatusWithSignature],
    pb: &ProgressBar,
) -> Vec<SlotChunk> {
    let mut all_chunks = fetcher.collect_chunks_owned(signatures, pb);
    all_chunks.sort_by_key(|(slot, _)| *slot);
    all_chunks
}

fn decompress_sessions(
    sessions: &[SessionChunks],
    signatures: &[RpcConfirmedTransactionStatusWithSignature],
    pb: &ProgressBar,
) -> Result<(
    Vec<(RpcConfirmedTransactionStatusWithSignature, Vec<u8>)>,
    usize,
)> {
    let mut failed = 0usize;

    let extracted: Vec<_> = sessions
        .iter()
        .flat_map(|session| {
            pb.inc(1);
            let combined_data = combine_chunks(session);
            let streams = decompress_all_streams(&combined_data);
            if streams.is_empty() {
                failed += 1;
                return Vec::new();
            }
            let session_slot = session
                .first()
                .map(|(slot, _)| *slot)
                .expect("could not reconstruct an IDL upload from the fetched transactions");
            let session_sig = signatures
                .iter()
                .find(|sig| sig.slot == session_slot)
                .expect("could not find the transaction for IDL upload at given slot")
                .clone();
            streams
                .into_iter()
                .map(|idl_data| (session_sig.clone(), idl_data))
                .collect::<Vec<_>>()
        })
        .collect();

    Ok((extracted, failed))
}

fn output_idls(
    idls: Vec<(RpcConfirmedTransactionStatusWithSignature, Vec<u8>)>,
    out_dir: Option<String>,
) -> Result<()> {
    let out_dir = resolve_idl_output_dir(out_dir.as_deref())?;
    std::fs::create_dir_all(&out_dir)?;

    for (i, (sig, idl_data)) in idls.iter().enumerate() {
        output_idl(
            idl_data,
            &historical_idl_output_path(i + 1, sig.slot, &out_dir),
        )?;
    }

    Ok(())
}

fn output_idl(idl_data: &[u8], path: &Path) -> Result<()> {
    std::fs::write(path, idl_data)?;
    println!("Saved IDL to: {}", path.display());
    Ok(())
}

// Resolve the output directory for the IDL.
// If no output directory is provided, use the current directory.
fn resolve_idl_output_dir(out_dir: Option<&str>) -> Result<PathBuf> {
    out_dir
        .map(PathBuf::from)
        .map_or_else(|| std::env::current_dir().map_err(Into::into), Ok)
}

// Generate the output path for a single IDL.
fn single_idl_output_path(slot: u64, out_dir: &std::path::Path) -> PathBuf {
    out_dir.join(format!("idl_{}.json", slot))
}

// Generate the output path for a historical IDL. Format: idl_v{version}_{slot}.json
fn historical_idl_output_path(version: usize, slot: u64, out_dir: &std::path::Path) -> PathBuf {
    out_dir.join(format!("idl_v{}_{}.json", version, slot))
}

fn extract_chunks_from_transaction(
    client: &RpcClient,
    signature: &Signature,
    tuning: &FetchTuning,
) -> Result<Vec<ChunkData>> {
    let transaction = fetch_transaction(client, signature, tuning)?;
    let ui_tx = parse_transaction_data(transaction)?;
    extract_chunks_from_message(ui_tx.message)
}

fn fetch_transaction(
    client: &RpcClient,
    signature: &Signature,
    tuning: &FetchTuning,
) -> Result<EncodedConfirmedTransactionWithStatusMeta> {
    let config = RpcTransactionConfig {
        encoding: Some(UiTransactionEncoding::Json),
        commitment: Some(CommitmentConfig::confirmed()),
        max_supported_transaction_version: Some(0),
    };

    let mut attempt: u32 = 0;
    loop {
        attempt += 1;
        match client.get_transaction_with_config(signature, config) {
            Ok(tx) => return Ok(tx),
            Err(e) => {
                let retryable = matches!(
                    e.kind(),
                    RpcClientErrorKind::Reqwest(error)
                        // Explicitly check for 429 responses as retryable
                        if error.status() == Some(StatusCode::TOO_MANY_REQUESTS)
                );
                if !retryable || attempt >= tuning.max_retries {
                    return Err(anyhow!("failed to fetch transaction {signature}: {e}"));
                }
                let backoff = tuning
                    .retry_backoff_ms
                    .saturating_mul(1u64 << (attempt - 1));
                std::thread::sleep(std::time::Duration::from_millis(backoff));
            }
        }
    }
}

fn parse_transaction_data(
    transaction: EncodedConfirmedTransactionWithStatusMeta,
) -> Result<UiTransaction> {
    match transaction.transaction {
        EncodedTransactionWithStatusMeta {
            transaction: EncodedTransaction::Json(ui_tx),
            ..
        } => Ok(ui_tx),
        _ => Err(anyhow!("Invalid transaction format")),
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
    let streams = decompress_all_streams(compressed_data);
    // When a session buffer contains several concatenated zlib streams (grouping
    // merged two adjacent uploads), the last complete stream is the newest IDL.
    // A single-stream session just returns its only entry.
    Ok(streams.into_iter().last())
}

fn decompress_all_streams(compressed_data: &[u8]) -> Vec<Vec<u8>> {
    const ZLIB_HEADER: u8 = 0x78;

    let mut streams = Vec::new();
    let mut cursor = compressed_data;

    while cursor.first() == Some(&ZLIB_HEADER) {
        let mut decoder = ZlibDecoder::new(cursor);
        let mut out = Vec::new();
        match decoder.read_to_end(&mut out) {
            Ok(_) => {
                let consumed = decoder.total_in() as usize;
                if is_complete_idl_json(&out) {
                    streams.push(out);
                }
                if consumed == 0 || consumed > cursor.len() {
                    break;
                }
                cursor = &cursor[consumed..];
            }
            Err(_) => break,
        }
    }

    streams
}

// A decompressed IDL stream may be truncated if the original upload was
// aborted mid-write but the zlib trailer happened to decode cleanly. Validate
// the output parses as JSON before accepting it — anything else is garbage.
fn is_complete_idl_json(data: &[u8]) -> bool {
    serde_json::from_slice::<serde_json::Value>(data).is_ok()
}

fn extract_compressed_chunk(data_str: &str) -> Result<Option<ChunkData>> {
    const IDL_HEADER_SIZE: usize = 13;

    let data = bs58::decode(data_str).into_vec()?;

    if !is_valid_idl_write_instruction(&data) {
        return Ok(None);
    }

    let vec_len = extract_payload_length(&data);

    if !has_complete_payload(&data, vec_len) {
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
