//! Fixed-capacity bit set implementation for allocation-free environments.
//!
//! This module provides [`FixedBitSet`], a space-efficient way to track boolean flags
//! at specific indices with compile-time capacity bounds.

/// A fixed-capacity bit set for tracking boolean flags at specific indices.
///
/// `FixedBitSet` provides an efficient way to track which indices in a range `[0, SIZE)`
/// are "included" or "set". It uses a boolean array internally for simplicity and clarity.
///
/// # Type Parameters
///
/// * `SIZE` - The maximum number of indices that can be tracked (compile-time constant).
///
/// # Examples
///
/// ```rust
/// use saturn_collections::generic::fixed_bitset::FixedBitSet;
///
/// let mut bitset: FixedBitSet<16> = FixedBitSet::new();
///
/// bitset.insert(3);
/// bitset.insert(7);
/// bitset.insert(15);
///
/// assert!(bitset.contains(3));
/// assert!(!bitset.contains(4));
/// assert_eq!(bitset.count(), 3);
///
/// // Iterate over set bits
/// let bits: Vec<_> = bitset.iter().collect();
/// assert_eq!(bits, vec![3, 7, 15]);
/// ```
///
/// # Memory Layout
///
/// The bit set stores a boolean array of size `SIZE` followed by a count field.
/// Each boolean represents whether the corresponding index is set.
#[derive(Debug)]
pub struct FixedBitSet<const SIZE: usize> {
    included: [bool; SIZE],
    count: usize,
}

impl<const SIZE: usize> Default for FixedBitSet<SIZE> {
    fn default() -> Self {
        Self::new()
    }
}

impl<const SIZE: usize> FixedBitSet<SIZE> {
    /// Creates a new, empty bit set.
    ///
    /// All bits are initially unset (false).
    ///
    /// # Examples
    ///
    /// ```rust
    /// use saturn_collections::generic::fixed_bitset::FixedBitSet;
    ///
    /// let bitset: FixedBitSet<10> = FixedBitSet::new();
    /// assert!(bitset.is_empty());
    /// assert_eq!(bitset.count(), 0);
    /// ```
    pub fn new() -> Self {
        Self {
            included: [false; SIZE],
            count: 0,
        }
    }

    /// Returns the number of set bits in the bit set.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use saturn_collections::generic::fixed_bitset::FixedBitSet;
    ///
    /// let mut bitset: FixedBitSet<10> = FixedBitSet::new();
    /// assert_eq!(bitset.count(), 0);
    ///
    /// bitset.insert(3);
    /// bitset.insert(7);
    /// assert_eq!(bitset.count(), 2);
    /// ```
    pub fn count(&self) -> usize {
        self.count
    }

    /// Returns `true` if no bits are set.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use saturn_collections::generic::fixed_bitset::FixedBitSet;
    ///
    /// let mut bitset: FixedBitSet<10> = FixedBitSet::new();
    /// assert!(bitset.is_empty());
    ///
    /// bitset.insert(5);
    /// assert!(!bitset.is_empty());
    /// ```
    pub fn is_empty(&self) -> bool {
        self.count == 0
    }

    /// Returns `true` if the bit at the given index is set.
    ///
    /// Returns `false` if the index is out of bounds.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use saturn_collections::generic::fixed_bitset::FixedBitSet;
    ///
    /// let mut bitset: FixedBitSet<10> = FixedBitSet::new();
    /// assert!(!bitset.contains(3));
    ///
    /// bitset.insert(3);
    /// assert!(bitset.contains(3));
    /// assert!(!bitset.contains(4));
    ///
    /// // Out of bounds
    /// assert!(!bitset.contains(10));
    /// ```
    pub fn contains(&self, index: usize) -> bool {
        index < SIZE && self.included[index]
    }

    /// Sets the bit at the given index.
    ///
    /// Returns `true` if the bit was newly set, `false` if it was already set
    /// or if the index is out of bounds.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use saturn_collections::generic::fixed_bitset::FixedBitSet;
    ///
    /// let mut bitset: FixedBitSet<10> = FixedBitSet::new();
    /// assert!(bitset.insert(3)); // Newly set
    /// assert!(!bitset.insert(3)); // Already set
    /// assert!(!bitset.insert(10)); // Out of bounds
    /// ```
    pub fn insert(&mut self, index: usize) -> bool {
        if index < SIZE && !self.included[index] {
            self.included[index] = true;
            self.count += 1;
            true
        } else {
            false
        }
    }

    /// Unsets the bit at the given index.
    ///
    /// Returns `true` if the bit was unset, `false` if it was already unset
    /// or if the index is out of bounds.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use saturn_collections::generic::fixed_bitset::FixedBitSet;
    ///
    /// let mut bitset: FixedBitSet<10> = FixedBitSet::new();
    /// bitset.insert(3);
    ///
    /// assert!(bitset.remove(3)); // Successfully removed
    /// assert!(!bitset.remove(3)); // Already unset
    /// assert!(!bitset.remove(10)); // Out of bounds
    /// ```
    pub fn remove(&mut self, index: usize) -> bool {
        if index < SIZE && self.included[index] {
            self.included[index] = false;
            self.count -= 1;
            true
        } else {
            false
        }
    }

    /// Sets multiple bits from a slice of indices.
    ///
    /// Indices that are out of bounds or already set are ignored.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use saturn_collections::generic::fixed_bitset::FixedBitSet;
    ///
    /// let mut bitset: FixedBitSet<10> = FixedBitSet::new();
    /// bitset.extend_from_slice(&[1, 3, 5, 3, 10]); // 3 is duplicate, 10 is out of bounds
    ///
    /// assert_eq!(bitset.count(), 3);
    /// assert!(bitset.contains(1));
    /// assert!(bitset.contains(3));
    /// assert!(bitset.contains(5));
    /// ```
    pub fn extend_from_slice(&mut self, indices: &[usize]) {
        for &index in indices {
            self.insert(index);
        }
    }

    /// Collects all set indices into a sorted array.
    ///
    /// Returns the actual count of set bits. The buffer must be at least as large
    /// as the bit set capacity.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use saturn_collections::generic::fixed_bitset::FixedBitSet;
    ///
    /// let mut bitset: FixedBitSet<8> = FixedBitSet::new();
    /// bitset.insert(4);
    /// bitset.insert(1);
    /// bitset.insert(7);
    ///
    /// let mut buffer = [0; 8];
    /// let count = bitset.collect_sorted(&mut buffer);
    /// assert_eq!(count, 3);
    /// assert_eq!(&buffer[..count], &[1, 4, 7]);
    /// ```
    pub fn collect_sorted(&self, buffer: &mut [usize; SIZE]) -> usize {
        let mut count = 0;
        for (index, &included) in self.included.iter().enumerate() {
            if included && count < SIZE {
                buffer[count] = index;
                count += 1;
            }
        }
        count
    }

    /// Returns an iterator over all set indices in ascending order.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use saturn_collections::generic::fixed_bitset::FixedBitSet;
    ///
    /// let mut bitset: FixedBitSet<8> = FixedBitSet::new();
    /// bitset.insert(2);
    /// bitset.insert(4);
    /// bitset.insert(6);
    ///
    /// let indices: Vec<_> = bitset.iter().collect();
    /// assert_eq!(indices, vec![2, 4, 6]);
    /// ```
    pub fn iter(&self) -> impl Iterator<Item = usize> + '_ {
        self.included
            .iter()
            .enumerate()
            .filter_map(|(index, &included)| if included { Some(index) } else { None })
    }

    /// Unsets all bits in the bit set.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use saturn_collections::generic::fixed_bitset::FixedBitSet;
    ///
    /// let mut bitset: FixedBitSet<8> = FixedBitSet::new();
    /// bitset.insert(0);
    /// bitset.insert(1);
    /// assert_eq!(bitset.count(), 2);
    ///
    /// bitset.clear();
    /// assert!(bitset.is_empty());
    /// assert_eq!(bitset.count(), 0);
    /// ```
    pub fn clear(&mut self) {
        self.included.fill(false);
        self.count = 0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SIZE: usize = 8;

    #[test]
    fn test_new_and_default_are_empty() {
        let set = FixedBitSet::<SIZE>::new();
        assert_eq!(set.count(), 0);
        assert!(set.is_empty());

        let default_set: FixedBitSet<SIZE> = Default::default();
        assert_eq!(default_set.count(), 0);
        assert!(default_set.is_empty());
    }

    #[test]
    fn test_insert_and_contains() {
        let mut set = FixedBitSet::<SIZE>::new();
        assert!(!set.contains(3));
        assert!(set.insert(3));
        assert!(set.contains(3));
        assert_eq!(set.count(), 1);
        assert!(!set.is_empty());

        // Duplicate insert
        assert!(!set.insert(3));
        assert_eq!(set.count(), 1);
    }

    #[test]
    fn test_insert_out_of_bounds() {
        let mut set = FixedBitSet::<SIZE>::new();
        assert!(!set.insert(SIZE));
        assert_eq!(set.count(), 0);
    }

    #[test]
    fn test_remove() {
        let mut set = FixedBitSet::<SIZE>::new();
        set.insert(2);
        set.insert(5);
        assert!(set.remove(2));
        assert!(!set.contains(2));
        assert_eq!(set.count(), 1);

        // Removing non-included index
        assert!(!set.remove(2));
        assert_eq!(set.count(), 1);
    }

    #[test]
    fn test_remove_out_of_bounds() {
        let mut set = FixedBitSet::<SIZE>::new();
        assert!(!set.remove(SIZE));
    }

    #[test]
    fn test_extend_from_slice() {
        let mut set = FixedBitSet::<SIZE>::new();
        set.extend_from_slice(&[1, 3, 5, 3]); // 3 is a duplicate
        assert!(set.contains(1));
        assert!(set.contains(3));
        assert!(set.contains(5));
        assert_eq!(set.count(), 3);
    }

    #[test]
    fn test_collect_sorted() {
        let mut set = FixedBitSet::<SIZE>::new();
        set.insert(4);
        set.insert(1);
        set.insert(7);

        let mut buffer = [0; SIZE];
        let count = set.collect_sorted(&mut buffer);
        assert_eq!(count, 3);
        assert_eq!(&buffer[..count], &[1, 4, 7]);
    }

    #[test]
    fn test_iter() {
        let mut set = FixedBitSet::<SIZE>::new();
        let indices = [2, 4, 6];
        set.extend_from_slice(&indices);

        let collected: Vec<_> = set.iter().collect();
        assert_eq!(collected, indices);
    }

    #[test]
    fn test_clear() {
        let mut set = FixedBitSet::<SIZE>::new();
        set.insert(0);
        set.insert(1);
        assert_eq!(set.count(), 2);
        set.clear();
        assert_eq!(set.count(), 0);
        assert!(set.is_empty());
        for i in 0..SIZE {
            assert!(!set.contains(i));
        }
    }
}
