//! Public aliases and helper macro for `ShardSet` ergonomics.
//!
//! Import this as `use saturn_account_shards::prelude::*` to gain the
//! user-friendly façade without pulling in the full type machinery.

use crate::shard_set::{Selected, ShardSet, Unselected};
use anchor_lang::prelude::AccountLoader;
use anchor_lang::Owner;
use anchor_lang::ZeroCopy;

/// Convenience alias for an **unselected** `ShardSet` in which the typestate
/// parameter is hidden.
pub type Shards<'slice, 'info, S, const MAX_SELECTED_SHARDS: usize> =
    ShardSet<'slice, 'info, S, MAX_SELECTED_SHARDS, Unselected>;

/// Convenience alias for a `ShardSet` that already carries an active shard
/// selection.
pub type SelectedShards<'slice, 'info, S, const MAX_SELECTED_SHARDS: usize> =
    ShardSet<'slice, 'info, S, MAX_SELECTED_SHARDS, Selected>;

/// Allows constructing a `ShardSet` via `slice.into()` instead of calling
/// `ShardSet::new(slice)` explicitly.
impl<'slice, 'info, S, const MAX_SELECTED_SHARDS: usize> From<&'slice [AccountLoader<'info, S>]>
    for ShardSet<'slice, 'info, S, MAX_SELECTED_SHARDS, Unselected>
where
    S: ZeroCopy + Owner,
    'slice: 'info,
{
    #[inline]
    fn from(slice: &'slice [AccountLoader<'info, S>]) -> Self {
        ShardSet::from_loaders(slice)
    }
}
