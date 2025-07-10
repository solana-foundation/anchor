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

// Blanket Accounts implementation that simply consumes *all* remaining accounts and
// groups them into shards, relying on the inner `T` to decide how many accounts it
// needs.  The macro-generated code for the account struct will typically build the
// `Shards` value directly, but having this impl makes the type usable on its own.
impl<'info, B, T> crate::Accounts<'info, B> for Shards<'info, T>
where
    T: crate::Accounts<'info, B>,
{
    fn try_accounts(
        program_id: &arch_program::pubkey::Pubkey,
        accounts: &mut &'info [arch_program::account::AccountInfo<'info>],
        ix_data: &[u8],
        bumps: &mut B,
        reallocs: &mut std::collections::BTreeSet<arch_program::pubkey::Pubkey>,
    ) -> crate::Result<Self> {
        let mut vec = Vec::new();
        while !accounts.is_empty() {
            let item = T::try_accounts(program_id, accounts, ix_data, bumps, reallocs)?;
            vec.push(item);
        }
        Ok(Self {
            shards: vec,
            _marker: core::marker::PhantomData,
        })
    }
}
