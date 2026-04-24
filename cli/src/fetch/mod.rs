use {
    crate::config::ConfigOverride,
    anyhow::{anyhow, Result},
    indicatif::{ProgressBar, ProgressStyle},
    solana_pubkey::Pubkey,
    solana_rpc_client::rpc_client::RpcClient,
    solana_rpc_client_api::response::RpcConfirmedTransactionStatusWithSignature,
    solana_signature::Signature,
    std::{path::PathBuf, str::FromStr, thread, time::Duration},
};

mod chunks;
mod decompress;
mod output;
mod rpc;
mod sessions;

use self::{
    chunks::extract_chunks_from_transaction,
    decompress::decompress_all_streams,
    output::{save_historical_idls, write_idl_file},
    rpc::{create_rpc_client, fetch_idl_signatures},
    sessions::{
        group_chunks_into_sessions, historical_fetch_worker_count,
        should_parallelize_historical_fetch,
    },
};

const DEFAULT_MAX_RETRIES: u32 = 5;
const DEFAULT_RETRY_BACKOFF_MS: u64 = 500;
const PROGRESS_TICK_INTERVAL_MS: u64 = 80;

type ChunkData = Vec<u8>;
type SlotChunk = (u64, ChunkData);
type SessionChunks = Vec<SlotChunk>;
type ExtractedIdl = (RpcConfirmedTransactionStatusWithSignature, Vec<u8>);

struct DecompressedSessions {
    extracted_idls: Vec<ExtractedIdl>,
    skipped_sessions: usize,
}

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
    // Binds a fetch tuning profile to a shared RPC client for one fetch run.
    fn new(client: &'a RpcClient, tuning: FetchTuning) -> Self {
        Self { client, tuning }
    }

    // Rejects slot queries that point past the cluster's current slot.
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

    // Collects IDL chunks for a borrowed slice of signatures on the current thread.
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

    // Chooses sequential or parallel collection for an owned signature page.
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

    // Splits the signature list across worker threads and merges recovered chunks.
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

// Extracts slot-tagged chunks for one transaction and reports per-signature failures
// through the shared progress bar.
fn collect_signature_chunks(
    client: &RpcClient,
    sig: &RpcConfirmedTransactionStatusWithSignature,
    tuning: &FetchTuning,
    pb: &ProgressBar,
) -> Option<Vec<SlotChunk>> {
    let signature = Signature::from_str(&sig.signature).ok()?;
    let chunks = match extract_chunks_from_transaction(client, &signature, tuning) {
        Ok(chunks) => chunks,
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

// Parses a CLI date filter into the UTC timestamp used by signature pagination.
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

// Fetches the newest IDL visible at or before a target slot and writes it to disk.
pub fn idl_fetch_at_slot(
    client: &RpcClient,
    all_signatures: &[RpcConfirmedTransactionStatusWithSignature],
    target_slot: u64,
    out_dir: Option<PathBuf>,
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
        if let Some(idl_data) = decompress_all_streams(&combined).into_iter().last() {
            let last_slot = session.last().map(|(s, _)| *s).unwrap_or(target_slot);
            println!(
                "Decompressed IDL from session ending at slot {} ({} bytes)",
                last_slot,
                idl_data.len()
            );
            return write_idl_file(
                &idl_data,
                &PathBuf::from(format!("idl_{}.json", target_slot)),
                out_dir.as_deref(),
            );
        }
        if idx + 1 < total {
            continue;
        }
    }

    Err(anyhow!(
        "No decompressable IDL session found at or before slot {}. Try fetching without --slot to \
         see recoverable versions.",
        target_slot
    ))
}

// Concatenates all chunk payload bytes for a reconstructed upload session.
fn combine_chunks(chunks: &[SlotChunk]) -> Vec<u8> {
    chunks
        .iter()
        .flat_map(|(_, chunk)| chunk.iter())
        .copied()
        .collect()
}

// Fetches all historical IDL uploads matching the requested slot/date filters.
pub fn idl_fetch_historical(
    cfg_override: &ConfigOverride,
    address: Pubkey,
    slot: Option<u64>,
    before: Option<String>,
    after: Option<String>,
    out_dir: Option<PathBuf>,
    tuning: FetchTuning,
) -> Result<()> {
    let before_timestamp = before.as_deref().map(parse_date_to_timestamp).transpose()?;
    let after_timestamp = after.as_deref().map(parse_date_to_timestamp).transpose()?;
    if let Some((after_ts, before_ts)) = after_timestamp.zip(before_timestamp) {
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

    let (filter_before, filter_after) = if slot.is_some() {
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

    let decompressed = decompress_sessions(&sessions, &signatures, &decompress_pb)?;
    decompress_pb.finish_with_message("Decompression complete");

    if decompressed.skipped_sessions > 0 {
        println!(
            "Skipped {}/{} session(s): no zlib streams found (partial uploads)",
            decompressed.skipped_sessions,
            sessions.len()
        );
    }

    if decompressed.extracted_idls.is_empty() {
        println!("\nNo IDL data could be fetched from historical slots.");
        return Ok(());
    }

    println!(
        "\nSuccessfully extracted {} IDL version(s)",
        decompressed.extracted_idls.len()
    );
    save_historical_idls(decompressed.extracted_idls, out_dir)
}

// Collects all chunks for a signature set and sorts them by slot for session reconstruction.
fn collect_and_process_chunks(
    fetcher: &IdlFetcher,
    signatures: &[RpcConfirmedTransactionStatusWithSignature],
    pb: &ProgressBar,
) -> Vec<SlotChunk> {
    let mut all_chunks = fetcher.collect_chunks_owned(signatures, pb);
    all_chunks.sort_by_key(|(slot, _)| *slot);
    all_chunks
}

// Decompresses every reconstructed session and pairs recovered IDLs with their source transaction.
fn decompress_sessions(
    sessions: &[SessionChunks],
    signatures: &[RpcConfirmedTransactionStatusWithSignature],
    pb: &ProgressBar,
) -> Result<DecompressedSessions> {
    let mut failed = 0usize;
    let mut extracted = Vec::new();

    for session in sessions {
        pb.inc(1);
        let combined_data = combine_chunks(session);
        let streams = decompress_all_streams(&combined_data);
        if streams.is_empty() {
            failed += 1;
            continue;
        }

        let session_slot = session.first().map(|(slot, _)| *slot).ok_or_else(|| {
            anyhow!("could not reconstruct an IDL upload from the fetched transactions")
        })?;
        let session_sig = signatures
            .iter()
            .find(|sig| sig.slot == session_slot)
            .ok_or_else(|| {
                anyhow!(
                    "could not find the transaction for IDL upload at given slot {session_slot}"
                )
            })?
            .clone();

        extracted.extend(
            streams
                .into_iter()
                .map(|idl_data| (session_sig.clone(), idl_data)),
        );
    }

    Ok(DecompressedSessions {
        extracted_idls: extracted,
        skipped_sessions: failed,
    })
}
