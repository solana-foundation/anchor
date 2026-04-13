use {
    crate::cursor::AccountCursor,
    pinocchio::{account::AccountView, address::Address},
};

/// Instruction-scoped context passed to every handler.
///
/// Holds the validated declared accounts (`accounts`), the program_id
/// (as a borrow to avoid the 32-byte copy into every frame), bumps for
/// PDA-backed fields, and a reference to the still-advancing cursor for
/// lazy `remaining_accounts()` access.
pub struct Context<'a, T: Bumps> {
    /// Program id as a reference — lives for the whole instruction
    /// since it comes from the entrypoint's input buffer.
    pub program_id: &'a Address,

    /// Declared accounts (the `#[derive(Accounts)]` struct).
    pub accounts: T,

    /// Bump seeds found during constraint validation. Provided as a
    /// convenience so handlers don't have to recalculate bump seeds or
    /// pass them in as arguments.
    pub bumps: T::Bumps,

    /// Cursor into the serialized input buffer, positioned at the
    /// *start* of the remaining-accounts region (after `try_accounts`
    /// has consumed exactly `T::HEADER_SIZE` declared accounts). Used
    /// by [`Self::remaining_accounts`] for on-demand walking.
    cursor: &'a mut AccountCursor,

    /// Number of accounts after the declared region. Decremented to
    /// zero once the remaining region is walked.
    remaining_num: u8,

    /// Cache of the parsed remaining accounts. Populated lazily on the
    /// first call to [`Self::remaining_accounts`]; subsequent calls
    /// return a clone of the cached vec so the caller receives an
    /// owned value (avoiding borrow conflicts with `self.accounts`).
    remaining_cache: Option<alloc::vec::Vec<AccountView>>,
}

impl<'a, T: Bumps> Context<'a, T> {
    #[inline(always)]
    pub fn new(
        program_id: &'a Address,
        accounts: T,
        bumps: T::Bumps,
        cursor: &'a mut AccountCursor,
        remaining_num: u8,
    ) -> Self {
        Self {
            program_id,
            accounts,
            bumps,
            cursor,
            remaining_num,
            remaining_cache: None,
        }
    }

    /// Walks any trailing accounts beyond the declared `T` fields and
    /// returns them as an owned `Vec<AccountView>`.
    ///
    /// On the first call this advances the cursor through the remaining
    /// region, populates an internal cache, and returns a clone. On
    /// subsequent calls the cached vec is cloned without re-walking the
    /// cursor. Returns an owned vec (rather than a slice tied to
    /// `&mut self`) so handlers can keep using `self.accounts` and
    /// `self.bumps` alongside the remaining list.
    pub fn remaining_accounts(&mut self) -> alloc::vec::Vec<AccountView> {
        if self.remaining_cache.is_none() {
            let mut v = alloc::vec::Vec::with_capacity(self.remaining_num as usize);
            for _ in 0..self.remaining_num {
                // SAFETY: cursor is positioned at the start of the
                // remaining region and `remaining_num` is the exact
                // number of accounts to walk.
                v.push(unsafe { self.cursor.next() });
            }
            self.remaining_cache = Some(v);
        }
        self.remaining_cache.as_ref().unwrap().clone()
    }
}

/// Trait linking an accounts struct to its generated bumps struct.
pub trait Bumps {
    type Bumps;
}
