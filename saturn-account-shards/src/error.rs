use saturn_collections::generic::fixed_set::FixedSetError;
use anchor_lang::error_code;
use arch_program::program_error::ProgramError;

/// Errors that can occur when manipulating a set of `StateShard` instances.
///
/// These variants intentionally do **not** hold additional data so that the
/// type can be marked `Copy` and embedded into other error types with zero-cost.
#[derive(PartialEq, Eq)]
#[error_code(offset = 300)]
pub enum StateShardError {
    /// A rune transfer required more runes than were actually present across
    /// the shards involved in the operation.
    #[msg("Not enough rune in shards")]
    NotEnoughRuneInShards,
    /// A runestone edict refers to an output index that is **not** part of the
    /// transaction we are processing.
    #[msg("Output edict is not in transaction")]
    OutputEdictIsNotInTransaction,

    #[msg("Math error in btc amount")]
    MathErrorInBalanceAmountAcrossShards,

    /// Too many runes in utxo
    ///
    /// This error is returned when the total amount of runes in the utxo is
    /// greater than the maximum allowed amount of runes in a utxo.
    #[msg("Too many runes in utxo")]
    TooManyRunesInUtxo,

    #[msg("Rune amount addition overflow")]
    RuneAmountAdditionOverflow,

    #[msg("Shards are full of btc utxos")]
    ShardsAreFullOfBtcUtxos,

    #[msg("Removing more runes than are present in the shards")]
    RemovingMoreRunesThanPresentInShards,

    /// The runestone did not contain the mandatory pointer field.
    #[msg("Missing pointer in runestone")]
    MissingPointerInRunestone,

    /// The pointer specified inside the runestone does not correspond to any
    /// output created by the transaction.
    #[msg("Runestone pointer is not in transaction")]
    RunestonePointerIsNotInTransaction,

    /// The caller attempted to select the same shard index more than once.
    #[msg("Duplicate shard index in selection")]
    DuplicateShardSelection,

    #[msg("Shard index out of bounds")]
    OutOfBounds,

    #[msg("Too many shards selected")]
    TooManyShardsSelected,

    #[msg("Too many rune UTXOs for the selected shards")]
    ExcessRuneUtxos,
}

impl From<FixedSetError> for StateShardError {
    fn from(error: FixedSetError) -> Self {
        match error {
            FixedSetError::Full => StateShardError::TooManyRunesInUtxo,
            FixedSetError::Duplicate => {
                panic!("unreachable. we couldn't have a duplicate rune input")
            }
        }
    }
}

// Allow using `?` to convert `StateShardError` into `ProgramError` directly.
impl From<StateShardError> for ProgramError {
    fn from(e: StateShardError) -> Self {
        anchor_lang::error::Error::from(e).into()
    }
}

/// Convenience alias used throughout the crate so functions can simply return
/// `Result<T>` instead of writing out the full type every time.
pub type Result<T> = core::result::Result<T, StateShardError>;
