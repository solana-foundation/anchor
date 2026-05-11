use {
    super::serialized_account::{AnchorAccountSerialize, SerializedAccount},
    borsh::{BorshDeserialize, BorshSerialize},
    solana_program_error::ProgramError,
};

/// Borsh codec tag for [`SerializedAccount`]. Zero-sized; selected at the
/// use site via the [`BorshAccount`] alias.
pub struct BorshSerializer;

impl<T> AnchorAccountSerialize<T> for BorshSerializer
where
    T: BorshSerialize + BorshDeserialize,
{
    #[inline(always)]
    fn serialize(value: &T, buf: &mut &mut [u8]) -> Result<(), ProgramError> {
        value
            .serialize(buf)
            .map_err(|_| ProgramError::InvalidAccountData)
    }

    #[inline(always)]
    fn deserialize(buf: &mut &[u8]) -> Result<T, ProgramError> {
        T::deserialize(buf).map_err(|_| ProgramError::InvalidAccountData)
    }
}

/// Borsh-serialized account type.
///
/// Validates owner, checks discriminator, deserializes via borsh. Holds a
/// pinocchio borrow guard (`Ref` for `load`, `RefMut` for `load_mut`);
/// `exit()` serializes through the held `RefMut`.
///
/// Type alias over [`SerializedAccount<T, BorshSerializer>`]; all inherent
/// methods (`address`, `release_borrow`, `reacquire_borrow_mut`,
/// `reacquire_guard_only`) and trait impls live on `SerializedAccount`.
///
/// ## `#[account(owner = X @ MyErr)]` does NOT surface `MyErr`
///
/// Owner/discriminator validation runs inside `load`/`load_mut`, before
/// derive-level constraints. A mismatch is `ProgramError::IllegalOwner`,
/// not the user's `@ MyErr`. For a custom error, use `UncheckedAccount`
/// with derive-level `owner = X @ MyErr` (you lose the built-in disc/borsh
/// checks).
pub type BorshAccount<T> = SerializedAccount<T, BorshSerializer>;
