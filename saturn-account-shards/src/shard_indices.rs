use saturn_collections::generic::fixed_list::{FixedList, FixedListError};

/// Abstraction over "something that can be turned into a list of shard indices".
///
/// The const generic `N` is the upper bound enforced by `ShardSet` on how many
/// shards may be selected at once.  Implementors should **panic** (matching the
/// behaviour of `FixedList::copy_from_slice`) when more than `N` indices are
/// supplied – this is considered a programmer error that should be caught
/// during testing.
pub trait IntoShardIndices<const N: usize> {
    /// Converts `self` into the canonical `FixedList` representation that
    /// `ShardSet` expects internally.
    fn into_indices(self) -> Result<FixedList<usize, N>, FixedListError>;
}

// --- Blanket implementations for the most common callers -------------------------- //

impl<const N: usize> IntoShardIndices<N> for usize {
    #[inline]
    fn into_indices(self) -> Result<FixedList<usize, N>, FixedListError> {
        let mut list = FixedList::new();
        list.push(self)?;
        Ok(list)
    }
}

impl<'b, const N: usize> IntoShardIndices<N> for &'b [usize] {
    #[inline]
    fn into_indices(self) -> Result<FixedList<usize, N>, FixedListError> {
        Ok(FixedList::from_slice(self))
    }
}

impl<const N: usize> IntoShardIndices<N> for Vec<usize> {
    #[inline]
    fn into_indices(self) -> Result<FixedList<usize, N>, FixedListError> {
        Ok(FixedList::from_slice(&self))
    }
}

impl<const N: usize> IntoShardIndices<N> for FixedList<usize, N> {
    #[inline]
    fn into_indices(self) -> Result<FixedList<usize, N>, FixedListError> {
        Ok(self)
    }
}

impl<const N: usize, const M: usize> IntoShardIndices<N> for [usize; M] {
    #[inline]
    fn into_indices(self) -> Result<FixedList<usize, N>, FixedListError> {
        Ok(FixedList::from_slice(&self))
    }
}

// Any-sized FixedList – copies elements, panics if it would overflow `N`.
impl<const N: usize, const M: usize> IntoShardIndices<N> for &FixedList<usize, M> {
    #[inline]
    fn into_indices(self) -> Result<FixedList<usize, N>, FixedListError> {
        Ok(FixedList::from_slice(self.as_slice()))
    }
}
