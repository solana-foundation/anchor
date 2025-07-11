pub struct Shards<'info, T> {
    /// Vector of shards – each shard is itself an account container (Account, AccountLoader, …).
    pub shards: Vec<T>,
    // Tie the lifetime parameter `'info` to the struct without storing an actual reference.
    _marker: core::marker::PhantomData<&'info ()>,
}

impl<'info, T> Shards<'info, T> {
    /// Creates a new `Shards` value from the given vector.
    pub fn new(shards: Vec<T>) -> Self {
        Self {
            shards,
            _marker: core::marker::PhantomData,
        }
    }
}

impl<'info, T> core::ops::Deref for Shards<'info, T> {
    type Target = [T];

    fn deref(&self) -> &Self::Target {
        &self.shards
    }
}

impl<'info, T> core::ops::DerefMut for Shards<'info, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.shards
    }
}

impl<'info, T> IntoIterator for Shards<'info, T> {
    type Item = T;
    type IntoIter = std::vec::IntoIter<T>;

    fn into_iter(self) -> Self::IntoIter {
        self.shards.into_iter()
    }
}

impl<'info, T: crate::ToAccountInfos<'info>> crate::ToAccountInfos<'info> for Shards<'info, T> {
    fn to_account_infos(&self) -> Vec<arch_program::account::AccountInfo<'info>> {
        self.shards
            .iter()
            .flat_map(|s| s.to_account_infos())
            .collect()
    }
}

impl<'info, T: crate::ToAccountMetas> crate::ToAccountMetas for Shards<'info, T> {
    fn to_account_metas(&self, is_signer: Option<bool>) -> Vec<arch_program::account::AccountMeta> {
        self.shards
            .iter()
            .flat_map(|s| s.to_account_metas(is_signer))
            .collect()
    }
}

impl<'info, T> core::ops::Index<usize> for Shards<'info, T> {
    type Output = T;

    fn index(&self, index: usize) -> &Self::Output {
        &self.shards[index]
    }
}

impl<'info, T> core::ops::IndexMut<usize> for Shards<'info, T> {
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        &mut self.shards[index]
    }
}

pub trait ShardIndexBumps {
    /// Pushes the given shard index (as a little-endian `u64`) onto the seed stack.
    fn push_shard_index(&mut self, idx: u64);
    /// Pops the most recently pushed shard index from the stack.
    fn pop_shard_index(&mut self);
}

// Blanket Accounts implementation that simply consumes *all* remaining accounts and
// groups them into shards, relying on the inner `T` to decide how many accounts it
// needs.  The macro-generated code for the account struct will typically build the
// `Shards` value directly, but having this impl makes the type usable on its own.
impl<'info, B, T> crate::Accounts<'info, B> for Shards<'info, T>
where
    T: crate::Accounts<'info, B>,
    B: ShardIndexBumps,
{
    fn try_accounts(
        program_id: &arch_program::pubkey::Pubkey,
        accounts: &mut &'info [arch_program::account::AccountInfo<'info>],
        ix_data: &[u8],
        bumps: &mut B,
        reallocs: &mut std::collections::BTreeSet<arch_program::pubkey::Pubkey>,
    ) -> crate::Result<Self> {
        let mut vec = Vec::new();
        let mut shard_idx: u64 = 0;
        while !accounts.is_empty() {
            let before_len = accounts.len();

            // Push the current index so inner `try_accounts` can include it in PDA seeds.
            bumps.push_shard_index(shard_idx);
            let item = T::try_accounts(program_id, accounts, ix_data, bumps, reallocs)?;
            bumps.pop_shard_index();

            // Ensure progress – `T::try_accounts` must shrink the slice.
            if accounts.len() == before_len {
                return Err(crate::error::ErrorCode::ShardsInnerDidNotConsume.into());
            }

            vec.push(item);
            shard_idx += 1;
        }
        Ok(Self {
            shards: vec,
            _marker: core::marker::PhantomData,
        })
    }
}

impl<'info, T: core::fmt::Debug> core::fmt::Debug for Shards<'info, T> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("Shards")
            .field("shards", &self.shards)
            .finish()
    }
}

impl<'info, T> core::default::Default for Shards<'info, T> {
    fn default() -> Self {
        Self {
            shards: Vec::new(),
            _marker: core::marker::PhantomData,
        }
    }
}

impl<'info, T> core::convert::AsRef<[T]> for Shards<'info, T> {
    fn as_ref(&self) -> &[T] {
        &self.shards
    }
}

impl<'info, T> core::convert::AsMut<[T]> for Shards<'info, T> {
    fn as_mut(&mut self) -> &mut [T] {
        &mut self.shards
    }
}

impl<'info, T> core::convert::From<Vec<T>> for Shards<'info, T> {
    fn from(shards: Vec<T>) -> Self {
        Self::new(shards)
    }
}

impl<'info, T> core::iter::FromIterator<T> for Shards<'info, T> {
    fn from_iter<I: IntoIterator<Item = T>>(iter: I) -> Self {
        Self::new(iter.into_iter().collect())
    }
}

impl<'info, T> core::iter::Extend<T> for Shards<'info, T> {
    fn extend<I: IntoIterator<Item = T>>(&mut self, iter: I) {
        self.shards.extend(iter);
    }
}

impl<'a, 'info, T> core::iter::IntoIterator for &'a Shards<'info, T> {
    type Item = &'a T;
    type IntoIter = core::slice::Iter<'a, T>;

    #[inline]
    fn into_iter(self) -> Self::IntoIter {
        self.shards.iter()
    }
}

impl<'a, 'info, T> core::iter::IntoIterator for &'a mut Shards<'info, T> {
    type Item = &'a mut T;
    type IntoIter = core::slice::IterMut<'a, T>;

    #[inline]
    fn into_iter(self) -> Self::IntoIter {
        self.shards.iter_mut()
    }
}
