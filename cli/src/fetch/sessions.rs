use {
    super::{FetchTuning, SessionChunks, SlotChunk},
    std::thread,
};

const DEFAULT_PARALLEL_FETCH_SIGNATURE_THRESHOLD: usize = 10;
const DEFAULT_MAX_PARALLEL_FETCH_WORKERS: usize = 4;
const SESSION_SLOT_GAP_THRESHOLD: u64 = 5_000;

// Groups ordered chunks into upload sessions using chunk-size transitions and slot gaps.
pub(super) fn group_chunks_into_sessions(all_chunks: &[SlotChunk]) -> Vec<SessionChunks> {
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

// Decides whether historical transaction fetches should fan out across worker threads.
pub(super) fn should_parallelize_historical_fetch(
    signature_count: usize,
    tuning: &FetchTuning,
) -> bool {
    if tuning.no_parallel {
        return false;
    }
    if matches!(tuning.workers, Some(1)) {
        return false;
    }
    signature_count > DEFAULT_PARALLEL_FETCH_SIGNATURE_THRESHOLD
}

// Chooses the worker count for parallel fetches, capped by runtime and CLI limits.
pub(super) fn historical_fetch_worker_count(signature_count: usize, tuning: &FetchTuning) -> usize {
    if !should_parallelize_historical_fetch(signature_count, tuning) {
        return 1;
    }

    let available = thread::available_parallelism()
        .map(|count| count.get())
        .unwrap_or(DEFAULT_MAX_PARALLEL_FETCH_WORKERS);
    let cap = tuning.workers.unwrap_or(DEFAULT_MAX_PARALLEL_FETCH_WORKERS);

    signature_count.min(available).min(cap).max(1)
}
