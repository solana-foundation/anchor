use crate::generic::fixed_set::{FixedSet, FixedSetError};
use bytemuck::{Pod, Zeroable};

#[macro_export]
macro_rules! declare_fixed_set {
    ($Name:ident, $T:ty, $SIZE:expr) => {
        #[repr(C)]
        #[derive(Clone, Copy, Debug, bytemuck::Pod, bytemuck::Zeroable)]
        pub struct $Name {
            items: [$T; $SIZE],
            len: u8,
            _padding: [u8; 15],
        }

        const _: () = {
            use core::mem::{align_of, size_of};

            const _ALIGN: usize = align_of::<$Name>();
            const _SIZE: usize = size_of::<$Name>();
            const _ITEM_SIZE: usize = size_of::<$T>() * $SIZE;
            // 1 byte for len + 15 bytes padding
            const _EXPECTED_SIZE: usize = _ITEM_SIZE + 1 + 15;
            const _: () = assert!(_SIZE == _EXPECTED_SIZE, "Size mismatch in FixedSet struct!");
        };

        impl $Name {
            /// Creates an empty fixed set.
            #[allow(clippy::declare_interior_mutable_const)]
            pub fn new() -> Self {
                Self {
                    items: core::array::from_fn(|_| <$T>::default()),
                    len: 0,
                    _padding: [0; 15],
                }
            }

            /// Number of elements in the set.
            pub fn len(&self) -> usize {
                self.len as usize
            }

            /// Capacity of the set.
            pub fn capacity(&self) -> usize {
                $SIZE
            }

            /// True if set is empty.
            pub fn is_empty(&self) -> bool {
                self.len == 0
            }

            /// True if set is full.
            pub fn is_full(&self) -> bool {
                self.len as usize == $SIZE
            }

            /// Iterator over items.
            pub fn iter(&self) -> impl Iterator<Item = &$T> + '_ {
                self.items[..self.len as usize].iter()
            }

            /// Mutable iterator over items.
            pub fn iter_mut(&mut self) -> impl Iterator<Item = &mut $T> + '_ {
                let len = self.len as usize;
                self.items[..len].iter_mut()
            }

            /// Returns true if set contains `item`.
            pub fn contains<Q>(&self, item: &Q) -> bool
            where
                $T: PartialEq<Q>,
            {
                self.iter().any(|i| i == item)
            }

            /// Returns the first item equal to `item` in the set.
            pub fn find<Q>(&self, item: &Q) -> Option<&$T>
            where
                $T: PartialEq<Q>,
            {
                self.iter().find(|i| *i == item)
            }

            /// Returns the first item equal to `item` in the set.
            pub fn find_mut<Q>(&mut self, item: &Q) -> Option<&mut $T>
            where
                $T: PartialEq<Q>,
            {
                self.iter_mut().find(|i| *i == item)
            }

            /// Insert item into set.
            pub fn insert(&mut self, item: $T) -> Result<(), FixedSetError>
            where
                $T: Copy + PartialEq + Default,
            {
                if self.contains(&item) {
                    return Err(FixedSetError::Duplicate);
                }
                if self.is_full() {
                    return Err(FixedSetError::Full);
                }
                self.items[self.len as usize] = item;
                self.len += 1;
                Ok(())
            }

            /// Insert `item` into the set; if it already exists, call `modify` to update it.
            pub fn insert_or_modify<E, F>(&mut self, item: $T, mut modify: F) -> Result<(), E>
            where
                $T: Copy + PartialEq + Default,
                F: FnMut(&mut $T) -> Result<(), E>,
                E: From<FixedSetError>,
            {
                if self.contains(&item) {
                    let pos = self.iter().position(|i| *i == item).unwrap();
                    modify(&mut self.items[pos])?;
                } else {
                    self.insert(item).map_err(E::from)?;
                }
                Ok(())
            }

            /// Remove item from set, returning it if present.
            pub fn remove<Q>(&mut self, item: &Q) -> Option<$T>
            where
                $T: PartialEq<Q> + Copy,
            {
                let pos_opt = (0..self.len as usize).find(|&i| self.items[i] == *item);
                if let Some(pos) = pos_opt {
                    let removed = self.items[pos];
                    self.len -= 1;
                    if pos != self.len as usize {
                        self.items[pos] = self.items[self.len as usize];
                    }
                    Some(removed)
                } else {
                    None
                }
            }

            /// Convert to slice.
            pub fn as_slice(&self) -> &[$T] {
                &self.items[..self.len as usize]
            }

            /// Convert to mutable slice.
            pub fn as_mut_slice(&mut self) -> &mut [$T] {
                &mut self.items[..self.len as usize]
            }
        }

        impl Default for $Name {
            fn default() -> Self {
                Self::new()
            }
        }

        impl From<$Name> for FixedSet<$T, $SIZE>
        where
            $T: Copy + PartialEq + Default,
        {
            fn from(custom: $Name) -> Self {
                let mut set = FixedSet::<$T, $SIZE>::default();
                for item in custom.as_slice().iter().copied() {
                    // Safe to unwrap; capacity matches
                    let _ = set.insert(item);
                }
                set
            }
        }

        impl From<FixedSet<$T, $SIZE>> for $Name
        where
            $T: Copy + PartialEq + Default,
        {
            fn from(set: FixedSet<$T, $SIZE>) -> Self {
                let mut custom = Self::new();
                for item in set.as_slice() {
                    // Safe to unwrap; capacity matches
                    let _ = custom.insert(*item);
                }
                custom
            }
        }

        // Implement the `FixedCapacitySet` trait for interoperability.
        impl crate::generic::fixed_set::FixedCapacitySet for $Name {
            type Item = $T;

            fn capacity(&self) -> usize {
                $SIZE
            }

            fn len(&self) -> usize {
                self.len()
            }

            fn contains<Q>(&self, item: &Q) -> bool
            where
                Self::Item: PartialEq<Q>,
            {
                self.contains(item)
            }

            fn find<Q>(&self, item: &Q) -> Option<&Self::Item>
            where
                Self::Item: PartialEq<Q>,
            {
                self.find(item)
            }

            fn find_mut<Q>(&mut self, item: &Q) -> Option<&mut Self::Item>
            where
                Self::Item: PartialEq<Q>,
            {
                self.find_mut(item)
            }

            fn insert(
                &mut self,
                item: Self::Item,
            ) -> Result<(), crate::generic::fixed_set::FixedSetError> {
                self.insert(item)
            }

            fn insert_or_modify<E, F>(&mut self, item: Self::Item, modify: F) -> Result<(), E>
            where
                F: FnMut(&mut Self::Item) -> Result<(), E>,
                E: From<crate::generic::fixed_set::FixedSetError>,
            {
                self.insert_or_modify(item, modify)
            }

            fn remove<Q>(&mut self, item: &Q) -> Option<Self::Item>
            where
                Self::Item: PartialEq<Q>,
            {
                self.remove(item)
            }

            fn as_slice(&self) -> &[Self::Item] {
                self.as_slice()
            }

            fn as_mut_slice(&mut self) -> &mut [Self::Item] {
                self.as_mut_slice()
            }

            fn iter(&self) -> impl Iterator<Item = &Self::Item> {
                self.iter()
            }

            fn iter_mut(&mut self) -> impl Iterator<Item = &mut Self::Item> {
                self.iter_mut()
            }
        }
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    // Use a primitive type (u32) to avoid external dependencies.
    declare_fixed_set!(TestFixedSet2, u32, 2);

    #[test]
    fn test_basic_operations() {
        let mut set = TestFixedSet2::new();

        // Initially empty
        assert!(set.is_empty());

        // Insert first element
        assert!(set.insert(10).is_ok());
        assert_eq!(set.len(), 1);

        // Duplicate insertion returns Duplicate error
        assert_eq!(set.insert(10), Err(FixedSetError::Duplicate));

        // Insert second element
        assert!(set.insert(20).is_ok());
        assert_eq!(set.len(), 2);

        // Now full – further insertions fail with Full error
        assert_eq!(set.insert(30), Err(FixedSetError::Full));

        // Contains checks
        assert!(set.contains(&10));
        assert!(set.contains(&20));

        // Remove an element
        assert_eq!(set.remove(&10), Some(10));
        assert!(!set.contains(&10));
        assert_eq!(set.len(), 1);
    }

    #[test]
    fn test_generic_search() {
        // Test with simple wrapper type that implements PartialEq with the wrapped type
        use bytemuck::{Pod, Zeroable};

        #[repr(transparent)]
        #[derive(Clone, Copy, Debug, PartialEq, Pod, Zeroable, Default)]
        pub struct Value(u32);

        #[derive(Debug, Clone, Copy, PartialEq)]
        struct ValueId(u32);

        impl PartialEq<ValueId> for Value {
            fn eq(&self, other: &ValueId) -> bool {
                self.0 == other.0
            }
        }

        declare_fixed_set!(ValueSet, Value, 3);

        let mut set = ValueSet::new();
        let value1 = Value(1);
        let value2 = Value(2);

        set.insert(value1).unwrap();
        set.insert(value2).unwrap();

        let search_id = ValueId(1);

        // Test that we can search using ValueId when the set contains Value
        assert!(set.contains(&search_id));

        // Test find_mut with different type
        let found_mut = set.find_mut(&search_id);
        assert!(found_mut.is_some());

        // Test remove with different type
        let removed = set.remove(&ValueId(2));
        assert_eq!(removed, Some(value2));
        assert!(!set.contains(&ValueId(2)));
    }
}
