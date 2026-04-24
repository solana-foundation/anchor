use {
    super::FetchTuning,
    crate::{
        cluster_url,
        config::{get_solana_cfg_url, Config, ConfigOverride},
    },
    anyhow::{anyhow, Result},
    solana_commitment_config::CommitmentConfig,
    solana_pubkey::Pubkey,
    solana_rpc_client::rpc_client::{GetConfirmedSignaturesForAddress2Config, RpcClient},
    solana_rpc_client_api::{
        client_error::{reqwest::StatusCode, ErrorKind as RpcClientErrorKind},
        config::RpcTransactionConfig,
        response::RpcConfirmedTransactionStatusWithSignature,
    },
    solana_signature::Signature,
    solana_transaction_status_client_types::{
        EncodedConfirmedTransactionWithStatusMeta, UiTransactionEncoding,
    },
    std::str::FromStr,
};

const IDL_SIGNATURE_PAGE_SIZE: usize = 100;

// Builds the RPC client used for historical IDL fetches from CLI cluster overrides.
pub(super) fn create_rpc_client(cfg_override: &ConfigOverride) -> Result<RpcClient> {
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

// Paginates the IDL account history and applies optional date bounds during collection.
pub(super) fn fetch_idl_signatures(
    client: &RpcClient,
    address: &Pubkey,
    before_timestamp: Option<i64>,
    after_timestamp: Option<i64>,
    target_slot: Option<u64>,
) -> Result<Vec<RpcConfirmedTransactionStatusWithSignature>> {
    let program_signer = Pubkey::find_program_address(&[], address).0;
    let idl_account_address = Pubkey::create_with_seed(&program_signer, "anchor:idl", address)
        .map_err(|e| anyhow!("Failed to derive IDL account address: {e}"))?;

    let mut signatures = Vec::new();
    let mut cursor: Option<Signature> = None;

    loop {
        let config = GetConfirmedSignaturesForAddress2Config {
            before: cursor,
            until: None,
            limit: Some(IDL_SIGNATURE_PAGE_SIZE),
            commitment: None,
        };
        let page = client.get_signatures_for_address_with_config(&idl_account_address, config)?;

        if page.is_empty() {
            break;
        }

        let reached_target_slot =
            target_slot.is_some_and(|slot| page.iter().any(|sig| sig.slot <= slot));

        let next_cursor = page
            .last()
            .and_then(|sig| Signature::from_str(&sig.signature).ok());

        let has_date_filter = before_timestamp.is_some() || after_timestamp.is_some();
        let mut crossed_after_bound = false;
        for sig in page {
            if sig.err.is_some() {
                continue;
            }
            if has_date_filter {
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

        if crossed_after_bound || reached_target_slot {
            break;
        }
        match next_cursor {
            Some(sig) => cursor = Some(sig),
            None => break,
        }
    }

    Ok(signatures)
}

// Fetches one transaction with retry/backoff handling for rate-limited RPC responses.
pub(super) fn fetch_transaction(
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
