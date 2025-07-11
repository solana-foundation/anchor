//! Host-side registry for fully-populated `UtxoInfo` structures used by unit tests.
//!
//! This module is **only** compiled when building for the host (`not(target_os = "solana")`).
//! Production BPF builds exclude it entirely.
#![cfg(not(target_os = "solana"))]

use once_cell::sync::Lazy;
use std::collections::HashMap;
use std::sync::Mutex;

use arch_program::utxo::UtxoMeta;
use saturn_bitcoin_transactions::utxo_info::UtxoInfo;

/// Global in-memory map keyed by the `UtxoMeta` that stores rich [`UtxoInfo`]s
/// for deterministic unit testing.
static TEST_INFO_REGISTRY: Lazy<Mutex<HashMap<UtxoMeta, UtxoInfo>>> =
    Lazy::new(|| Mutex::new(HashMap::new()));

/// Register a fully-populated [`UtxoInfo`] so that [`crate::meta_to_info`] can
/// return it instead of a stub during unit tests.
pub fn register_test_utxo_info(info: UtxoInfo) {
    TEST_INFO_REGISTRY
        .lock()
        .expect("registry poisoned")
        .insert(info.meta.clone(), info);
}

/// Look up a previously-registered [`UtxoInfo`] by its meta. Returns `None` if
/// the meta has not been registered.
pub fn lookup(meta: &UtxoMeta) -> Option<UtxoInfo> {
    TEST_INFO_REGISTRY
        .lock()
        .expect("registry poisoned")
        .get(meta)
        .cloned()
}
