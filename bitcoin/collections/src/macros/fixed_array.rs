use bytemuck::{Pod, Zeroable};

/// This macro declares a new array type for a specified type
#[macro_export]
macro_rules! declare_fixed_array {
    ($Name:ident, $T:ty, $SIZE:expr) => {
        #[derive(Clone, Copy, Debug, PartialEq, Eq, bytemuck::Pod, bytemuck::Zeroable)]
        #[repr(C)]
        pub struct $Name {
            items: [$T; $SIZE],
            count: u16, // Using u16; ensure N <= u16::MAX
            _padding: [u8; 14],
        }

        const _: () = {
            use core::mem::{align_of, size_of};

            const _ALIGN: usize = align_of::<$Name>();
            const _SIZE: usize = size_of::<$Name>();
            const _ITEM_SIZE: usize = size_of::<$T>() * $SIZE;
            const _EXPECTED_SIZE: usize = _ITEM_SIZE + 2 + 14;
            const _: () = assert!(
                _SIZE == _EXPECTED_SIZE,
                "Size mismatch in FixedArray struct!"
            );
        };

        impl $Name {
            /// Creates a new, empty `FixedArray`.
            ///
            /// Panics if `N` (the capacity) is greater than `u16::MAX`.
            pub fn new() -> Self {
                assert!(
                    $SIZE <= u16::MAX as usize,
                    "Capacity N exceeds u16 for count field"
                );
                Self {
                    items: core::array::from_fn(|_| <$T>::default()),
                    count: 0,
                    _padding: [0; 14],
                }
            }

            /// Creates a new `FixedArray` from a slice.
            ///
            /// Copies elements from the `input_slice` into the new `FixedArray`
            /// up to the capacity `N` of the `FixedArray` or the length of the slice,
            /// whichever is smaller.
            pub fn from_slice(input_slice: &[$T]) -> Self {
                let mut fa = Self::new(); // Initialize with default (empty with capacity N)
                let num_to_copy = std::cmp::min(input_slice.len(), $SIZE);

                for i in 0..num_to_copy {
                    // This clone is necessary because `input_slice` elements are borrowed,
                    // and `fa.items` owns its elements.
                    // The `add` method handles putting the item into `fa.items[fa.count]`
                    // and incrementing `fa.count`.
                    // Since `fa` is new and empty, `add` should not return Err here,
                    // so we can `.ok()` it (or `.unwrap()` if absolutely certain).
                    fa.add(input_slice[i].clone()).unwrap();
                }
                fa
            }

            /// Returns the number of active elements in the array.
            pub fn len(&self) -> usize {
                self.count as usize
            }

            /// Returns `true` if the array contains no active elements.
            pub fn is_empty(&self) -> bool {
                self.count == 0
            }

            /// Returns the total capacity of the array.
            pub fn capacity(&self) -> usize {
                $SIZE
            }

            /// Returns `true` if the array is at full capacity.
            pub fn is_full(&self) -> bool {
                (self.count as usize) == $SIZE
            }

            /// Returns a reference to the element at `index`, or `None` if out of bounds.
            pub fn get(&self, index: usize) -> Option<&$T> {
                if index < self.len() {
                    Some(&self.items[index])
                } else {
                    None
                }
            }

            /// Returns a mutable reference to the element at `index`, or `None` if out of bounds.
            pub fn get_mut(&mut self, index: usize) -> Option<&mut $T> {
                if index < self.len() {
                    Some(&mut self.items[index])
                } else {
                    None
                }
            }

            /// Adds an item to the end of the collection.
            ///
            /// Returns `Ok(usize)` with the index of the added item if successful.
            /// Returns `Err(T)` containing the item back if the collection is full.
            pub fn add(&mut self, item: $T) -> Option<usize> {
                if self.is_full() {
                    None
                } else {
                    let index = self.count as usize;
                    self.items[index] = item;
                    self.count += 1;
                    Some(index)
                }
            }

            /// Removes an item from the specified `index` and returns it.
            ///
            /// Elements after the removed item are shifted to fill the gap.
            /// The slot at the end of the (now shorter) active list is reset to `T::default()`.
            /// Returns `None` if `index` is out of bounds.
            pub fn remove_at(&mut self, index: usize) -> Option<$T> {
                if index >= self.len() {
                    return None;
                }

                let removed_item = self.items[index].clone();

                for i in index..(self.len() - 1) {
                    self.items[i] = self.items[i + 1].clone();
                }

                self.count -= 1; // Decrement count first

                // Reset the slot that is no longer in use at the new end of active items
                if $SIZE > 0 {
                    // Check N to prevent panic on N=0 (though items array implies N > 0)
                    self.items[self.len()] = <$T>::default(); // self.len() is the new count
                }

                Some(removed_item)
            }

            /// Removes the first occurrence of `item_to_remove`.
            ///
            /// Returns `true` if an item was removed, `false` otherwise.
            /// Requires `T: PartialEq`.
            pub fn remove_item(&mut self, item_to_remove: &$T) -> bool {
                let mut found_index: Option<usize> = None;
                for i in 0..self.len() {
                    if self.items[i] == *item_to_remove {
                        found_index = Some(i);
                        break;
                    }
                }

                if let Some(index) = found_index {
                    self.remove_at(index);
                    true
                } else {
                    false
                }
            }

            /// Returns an iterator over the active elements.
            pub fn iter(&self) -> impl Iterator<Item = &$T> + '_ {
                self.items.iter().take(self.len())
            }

            /// Returns a mutable iterator over the active elements.
            pub fn iter_mut(&mut self) -> impl Iterator<Item = &mut $T> + '_ {
                let len = self.len();
                self.items.iter_mut().take(len)
            }

            /// Clears the array, removing all active elements and resetting them to `T::default()`.
            pub fn clear(&mut self) {
                for i in 0..self.len() {
                    self.items[i] = <$T>::default();
                }
                self.count = 0;
            }

            /// Returns a slice containing all active elements.
            pub fn as_slice(&self) -> &[$T] {
                &self.items[0..self.len()]
            }

            /// Returns a mutable slice containing all active elements.
            pub fn as_mut_slice(&mut self) -> &mut [$T] {
                let len = self.len();
                &mut self.items[0..len]
            }

            /// Retains only the elements specified by the predicate.
            ///
            /// In other words, remove all elements `e` such that `f(&e)` returns `false`.
            /// This method operates in place, visiting each element exactly once in the
            /// original order, and preserves the order of the retained elements.
            ///
            /// Because this function requires reading from and writing to the same
            /// locations, the `T: Clone` bound is used when elements are shifted.
            pub fn retain<F>(&mut self, mut f: F)
            where
                F: FnMut(&$T) -> bool,
            {
                let original_len = self.len();
                let mut write_idx = 0;
                let mut read_idx = 0;

                // Iterate through the active part of the array
                while read_idx < original_len {
                    // Decide whether to keep the element at read_idx
                    if f(&self.items[read_idx]) {
                        // If keeping it and it's not already in the correct place, move it.
                        if read_idx != write_idx {
                            self.items[write_idx] = self.items[read_idx].clone();
                        }
                        write_idx += 1;
                    }
                    read_idx += 1;
                }

                // Reset the slots for elements that were removed (from new_len to original_len)
                for i in write_idx..original_len {
                    self.items[i] = <$T>::default();
                }

                self.count = write_idx as u16;
            }

            pub fn as_vec(&self) -> Vec<$T> {
                self.items
                    .iter()
                    .take(self.count as usize)
                    .cloned()
                    .collect()
            }
        }

        impl Default for $Name {
            fn default() -> Self {
                Self::new()
            }
        }
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Clone, Copy, Debug, Default, PartialEq, Eq, bytemuck::Pod, bytemuck::Zeroable)]
    #[repr(C)]
    struct TestItem {
        id: u32,
        data: [u8; 4], // Example field
    }

    const TEST_CAPACITY: usize = 3;
    const LARGE_CAPACITY: usize = 50; // For serde N > 32 test
    const OVER_U16_MAX_CAPACITY: usize = (u16::MAX as usize) + 1;

    declare_fixed_array!(FixedArrayTestItemNone, TestItem, 0);
    declare_fixed_array!(FixedArrayTestItemSingle, TestItem, 1);
    declare_fixed_array!(FixedArrayTestItemThree, TestItem, 3);
    declare_fixed_array!(FixedArrayTestItemFive, TestItem, 5);
    declare_fixed_array!(FixedArrayTestItemTest, TestItem, TEST_CAPACITY);
    declare_fixed_array!(FixedArrayTestItemLarge, TestItem, LARGE_CAPACITY);
    declare_fixed_array!(FixedArrayTestItemOversized, TestItem, OVER_U16_MAX_CAPACITY);

    #[test]
    fn test_new_empty_full_capacity() {
        let fa = FixedArrayTestItemTest::new();
        assert_eq!(fa.len(), 0);
        assert!(fa.is_empty());
        assert!(!fa.is_full());
        assert_eq!(fa.capacity(), TEST_CAPACITY);
    }

    #[test]
    fn test_add_and_get() {
        let mut fa = FixedArrayTestItemTest::new();
        let item1 = TestItem {
            id: 1,
            data: [1; 4],
        };
        let item2 = TestItem {
            id: 2,
            data: [2; 4],
        };

        assert_eq!(fa.add(item1.clone()), Some(0));
        assert_eq!(fa.len(), 1);
        assert!(!fa.is_empty());
        assert_eq!(fa.get(0), Some(&item1));
        assert_eq!(fa.get(1), None); // Out of bounds for active items

        assert_eq!(fa.add(item2.clone()), Some(1));
        assert_eq!(fa.len(), 2);
        assert_eq!(fa.get(1), Some(&item2));
    }

    #[test]
    fn test_add_to_full_array() {
        let mut fa = FixedArrayTestItemSingle::new();
        let item1 = TestItem {
            id: 1,
            data: [1; 4],
        };
        let item2 = TestItem {
            id: 2,
            data: [2; 4],
        };

        assert_eq!(fa.add(item1.clone()), Some(0));
        assert!(fa.is_full());
        assert_eq!(fa.len(), 1);

        match fa.add(item2.clone()) {
            None => {}
            Some(_) => panic!("Should not be able to add to a full array."),
        }
        assert_eq!(fa.len(), 1); // Length should remain unchanged
    }

    #[test]
    fn test_remove_at_and_shifting() {
        let mut fa = FixedArrayTestItemTest::new();
        let item1 = TestItem {
            id: 1,
            data: [1; 4],
        };
        let item2 = TestItem {
            id: 2,
            data: [2; 4],
        };
        let item3 = TestItem {
            id: 3,
            data: [3; 4],
        };

        fa.add(item1.clone()).unwrap();
        fa.add(item2.clone()).unwrap();
        fa.add(item3.clone()).unwrap();
        assert!(fa.is_full());

        // Remove middle item
        assert_eq!(fa.remove_at(1), Some(item2.clone()));
        assert_eq!(fa.len(), 2);
        assert!(!fa.is_full());
        assert_eq!(fa.get(0), Some(&item1));
        assert_eq!(fa.get(1), Some(&item3)); // item3 should shift to index 1
        assert_eq!(fa.items[2], TestItem::default()); // Last slot reset

        // Remove first item
        assert_eq!(fa.remove_at(0), Some(item1.clone()));
        assert_eq!(fa.len(), 1);
        assert_eq!(fa.get(0), Some(&item3));
        assert_eq!(fa.items[1], TestItem::default()); // Slot reset

        // Remove last remaining item
        assert_eq!(fa.remove_at(0), Some(item3.clone()));
        assert_eq!(fa.len(), 0);
        assert!(fa.is_empty());
        assert_eq!(fa.items[0], TestItem::default()); // Slot reset

        // Try removing from empty or out of bounds
        assert_eq!(fa.remove_at(0), None);
    }

    #[test]
    fn test_remove_at_clears_slot() {
        let mut fa = FixedArrayTestItemTest::new();
        let item1 = TestItem {
            id: 1,
            data: [1; 4],
        };
        fa.add(item1.clone()).unwrap();

        // Check internal state before remove
        assert_eq!(fa.items[0], item1);
        assert_eq!(fa.items[1], TestItem::default());

        fa.remove_at(0).unwrap();
        assert_eq!(fa.len(), 0);
        assert_eq!(
            fa.items[0],
            TestItem::default(),
            "Slot should be cleared to default"
        );
    }

    #[test]
    fn test_remove_item() {
        let mut fa = FixedArrayTestItemTest::new();
        let item1 = TestItem {
            id: 1,
            data: [1; 4],
        };
        let item2 = TestItem {
            id: 2,
            data: [2; 4],
        };
        let item3 = TestItem {
            id: 3,
            data: [3; 4],
        }; // Not added initially

        fa.add(item1.clone()).unwrap();
        fa.add(item2.clone()).unwrap();

        assert!(fa.remove_item(&item1));
        assert_eq!(fa.len(), 1);
        assert_eq!(fa.get(0), Some(&item2));
        assert_eq!(fa.items[1], TestItem::default()); // Check slot cleared

        assert!(!fa.remove_item(&item1)); // Already removed
        assert!(!fa.remove_item(&item3)); // Not present
        assert_eq!(fa.len(), 1);
    }

    #[test]
    fn test_iter_and_iter_mut() {
        let mut fa = FixedArrayTestItemTest::new();
        let item1 = TestItem {
            id: 1,
            data: [1; 4],
        };
        let item2 = TestItem {
            id: 2,
            data: [2; 4],
        };

        fa.add(item1.clone()).unwrap();
        fa.add(item2.clone()).unwrap();

        let collected_items: Vec<&TestItem> = fa.iter().collect();
        assert_eq!(collected_items, vec![&item1, &item2]);

        for item_ref in fa.iter_mut() {
            item_ref.id += 10;
        }

        assert_eq!(fa.get(0).unwrap().id, 11);
        assert_eq!(fa.get(1).unwrap().id, 12);
    }

    #[test]
    fn test_clear() {
        let mut fa = FixedArrayTestItemTest::new();
        fa.add(TestItem {
            id: 1,
            data: [1; 4],
        })
        .unwrap();
        fa.add(TestItem {
            id: 2,
            data: [2; 4],
        })
        .unwrap();
        assert_eq!(fa.len(), 2);

        fa.clear();
        assert_eq!(fa.len(), 0);
        assert!(fa.is_empty());
        // Check that underlying array slots are defaulted
        assert_eq!(fa.items[0], TestItem::default());
        assert_eq!(fa.items[1], TestItem::default());
    }

    #[test]
    fn test_as_slice() {
        let mut fa = FixedArrayTestItemTest::new();
        let item1 = TestItem {
            id: 1,
            data: [1; 4],
        };
        let item2 = TestItem {
            id: 2,
            data: [2; 4],
        };

        fa.add(item1.clone()).unwrap();
        fa.add(item2.clone()).unwrap();

        let slice = fa.as_slice();
        assert_eq!(slice.len(), 2);
        assert_eq!(slice[0], item1);
        assert_eq!(slice[1], item2);

        fa.as_mut_slice()[0].id = 100;
        assert_eq!(fa.get(0).unwrap().id, 100);
    }

    #[test]
    fn test_default_impl() {
        let fa = FixedArrayTestItemTest::default();
        assert_eq!(fa.len(), 0);
        assert!(fa.is_empty());
        assert_eq!(fa.capacity(), TEST_CAPACITY);
    }

    #[test]
    fn test_from_slice() {
        let item1 = TestItem {
            id: 1,
            data: [1; 4],
        };
        let item2 = TestItem {
            id: 2,
            data: [2; 4],
        };
        let item3 = TestItem {
            id: 3,
            data: [3; 4],
        };
        let item4 = TestItem {
            id: 4,
            data: [4; 4],
        };

        // Test 1: Empty slice
        let slice_empty: &[TestItem] = &[];
        let fa_empty = FixedArrayTestItemTest::from_slice(slice_empty);
        assert_eq!(fa_empty.len(), 0);
        assert!(fa_empty.is_empty());

        // Test 2: Slice smaller than capacity
        let slice_small = &[item1.clone(), item2.clone()];
        let fa_small = FixedArrayTestItemTest::from_slice(slice_small);
        assert_eq!(fa_small.len(), 2);
        assert_eq!(fa_small.get(0), Some(&item1));
        assert_eq!(fa_small.get(1), Some(&item2));
        assert_eq!(fa_small.get(2), None);

        // Test 3: Slice equal to capacity
        let slice_exact = &[item1.clone(), item2.clone(), item3.clone()];
        let fa_exact = FixedArrayTestItemTest::from_slice(slice_exact);
        assert_eq!(fa_exact.len(), TEST_CAPACITY);
        assert!(fa_exact.is_full());
        assert_eq!(fa_exact.get(0), Some(&item1));
        assert_eq!(fa_exact.get(1), Some(&item2));
        assert_eq!(fa_exact.get(2), Some(&item3));

        // Test 4: Slice larger than capacity
        let slice_large = &[item1.clone(), item2.clone(), item3.clone(), item4.clone()];
        let fa_large = FixedArrayTestItemTest::from_slice(slice_large);
        assert_eq!(fa_large.len(), TEST_CAPACITY);
        assert!(fa_large.is_full());
        assert_eq!(fa_large.get(0), Some(&item1));
        assert_eq!(fa_large.get(1), Some(&item2));
        assert_eq!(fa_large.get(2), Some(&item3));
        assert_eq!(fa_large.get(3), None); // Element 4 should not be included

        // Test 5: Zero capacity array with non-empty slice (should remain empty)
        let fa_zero_cap = FixedArrayTestItemNone::from_slice(slice_small);
        assert_eq!(fa_zero_cap.len(), 0);
        assert!(fa_zero_cap.is_empty());
        assert!(fa_zero_cap.is_full()); // A zero-capacity array is always full

        // Test 6: Zero capacity array with empty slice
        let fa_zero_cap_empty_slice = FixedArrayTestItemNone::from_slice(slice_empty);
        assert_eq!(fa_zero_cap_empty_slice.len(), 0);
        assert!(fa_zero_cap_empty_slice.is_empty());
        assert!(fa_zero_cap_empty_slice.is_full());
    }

    #[test]
    fn test_retain() {
        let item1 = TestItem {
            id: 1,
            data: [1; 4],
        };
        let item2 = TestItem {
            id: 2,
            data: [2; 4],
        };
        let item3 = TestItem {
            id: 3,
            data: [3; 4],
        };
        let item4 = TestItem {
            id: 4,
            data: [4; 4],
        };
        let item5 = TestItem {
            id: 5,
            data: [5; 4],
        };

        // Test 1: Retain all
        let mut fa1 =
            FixedArrayTestItemFive::from_slice(&[item1.clone(), item2.clone(), item3.clone()]);
        fa1.retain(|item| item.id > 0); // Keep all
        assert_eq!(fa1.len(), 3);
        assert_eq!(
            fa1.as_slice(),
            &[item1.clone(), item2.clone(), item3.clone()]
        );

        // Test 2: Retain none
        let mut fa2 =
            FixedArrayTestItemFive::from_slice(&[item1.clone(), item2.clone(), item3.clone()]);
        fa2.retain(|item| item.id > 10); // Keep none
        assert_eq!(fa2.len(), 0);
        assert!(fa2.is_empty());
        assert_eq!(fa2.items[0], TestItem::default()); // Check slots are cleared
        assert_eq!(fa2.items[1], TestItem::default());
        assert_eq!(fa2.items[2], TestItem::default());

        // Test 3: Retain some (even numbers)
        let mut fa3 = FixedArrayTestItemFive::from_slice(&[
            item1.clone(),
            item2.clone(),
            item3.clone(),
            item4.clone(),
            item5.clone(),
        ]);
        fa3.retain(|item| item.id % 2 == 0); // Keep 2, 4
        assert_eq!(fa3.len(), 2);
        assert_eq!(fa3.as_slice(), &[item2.clone(), item4.clone()]);
        assert_eq!(fa3.items[2], TestItem::default()); // Check slots are cleared
        assert_eq!(fa3.items[3], TestItem::default());
        assert_eq!(fa3.items[4], TestItem::default());

        // Test 4: Retain from an empty array
        let mut fa4 = FixedArrayTestItemFive::new();
        fa4.retain(|item| item.id > 0);
        assert_eq!(fa4.len(), 0);
        assert!(fa4.is_empty());

        // Test 5: Retain that removes from the beginning
        let mut fa5 =
            FixedArrayTestItemThree::from_slice(&[item1.clone(), item2.clone(), item3.clone()]);
        fa5.retain(|item| item.id > 1); // Keep 2, 3
        assert_eq!(fa5.len(), 2);
        assert_eq!(fa5.as_slice(), &[item2.clone(), item3.clone()]);
        assert_eq!(fa5.items[2], TestItem::default());

        // Test 6: Retain that removes from the end
        let mut fa6 =
            FixedArrayTestItemThree::from_slice(&[item1.clone(), item2.clone(), item3.clone()]);
        fa6.retain(|item| item.id < 3); // Keep 1, 2
        assert_eq!(fa6.len(), 2);
        assert_eq!(fa6.as_slice(), &[item1.clone(), item2.clone()]);
        assert_eq!(fa6.items[2], TestItem::default());

        // Test 7: Retain that removes from the middle
        let mut fa7 =
            FixedArrayTestItemThree::from_slice(&[item1.clone(), item2.clone(), item3.clone()]);
        fa7.retain(|item| item.id == 1 || item.id == 3); // Keep 1, 3
        assert_eq!(fa7.len(), 2);
        assert_eq!(fa7.as_slice(), &[item1.clone(), item3.clone()]);
        assert_eq!(fa7.items[2], TestItem::default());

        // Test 8: Retain on a full array resulting in a partially full array
        let mut fa8 =
            FixedArrayTestItemThree::from_slice(&[item1.clone(), item2.clone(), item3.clone()]);
        assert!(fa8.is_full());
        fa8.retain(|item| item.id < 2); // Keep 1
        assert_eq!(fa8.len(), 1);
        assert_eq!(fa8.as_slice(), &[item1.clone()]);
        assert!(!fa8.is_full());
        assert_eq!(fa8.items[1], TestItem::default());
        assert_eq!(fa8.items[2], TestItem::default());
    }

    // #[test]
    // fn test_borsh_serialization_large_array() {
    //     let mut fa = FixedArrayTestItemLarge::new();
    //     fa.add(TestItem {
    //         id: 10,
    //         data: [1; 4],
    //     })
    //     .unwrap();
    //     fa.add(TestItem {
    //         id: 20,
    //         data: [2; 4],
    //     })
    //     .unwrap();

    //     let encoded = borsh::to_vec(&fa).expect("Borsh serialization failed");
    //     let decoded: FixedArray<TestItem, LARGE_CAPACITY> =
    //         borsh::from_slice(&encoded).expect("Borsh deserialization failed");

    //     assert_eq!(
    //         fa, decoded,
    //         "Original and Borsh deserialized FixedArray differ"
    //     );
    //     assert_eq!(decoded.len(), 2);
    //     assert_eq!(decoded.get(1).unwrap().id, 20);
    // }

    #[test]
    #[should_panic(expected = r#"Capacity N exceeds u16 for count field"#)]
    fn test_new_panics_on_too_large_n() {
        // This test will only work if u16::MAX is small enough to actually allocate
        // For most systems, usize will be larger, so this specific test might not reflect
        // a practical memory limit, but tests the assertion.
        // const HUGE_CAPACITY: usize = (u16::MAX as usize) + 1;
        // FixedArray::<TestItem, HUGE_CAPACITY>::new();
        // Re-evaluating how to test this panic safely.
        // The assertion is `N <= u16::MAX as usize`.
        // If usize is 64-bit, this is fine. The issue would be if N was truly huge.
        // For now, the assertion is a safeguard for `count` field type.
        // Let's assume the typical N will be much smaller.
        // For a direct test of the panic, we'd need a const N > u16::MAX.
        // This can be simulated if we reduce u16::MAX for the test or use a very large N.
        // Since N is a const generic, this is tricky to test dynamically for the panic.
        // The assertion itself is the primary check.
        // For the purpose of this unit test, we'll assume N is within reasonable bounds.
        // If we want to force the panic for testing:
        struct MockU16MaxTest; // Create a dummy type
                               // If we could define `FixedArray<MockU16MaxTest, 65537>` here, it would test.
                               // However, this needs to be a compile-time constant.

        // This kind of test is hard to do for const generics within a single test function
        // without conditional compilation or specific build setups.
        // The assertion is clear enough: `assert!(N <= u16::MAX as usize, ...)`
        // Let's assume the assertion itself serves its purpose.
        // If a user tries `FixedArray<T, 70000>`, it will panic at `new()`.
        // For now, we trust the assertion and skip a direct panic test for this.
        // To actually test it, one would need to instantiate with N > 65535.
        // E.g. `FixedArray::<u8, 70000>::new();` would trigger it.
        // We can add a specific test case if required, but it will only compile if N can be that large.

        // Forcing a panic for the test with a specific large N value:
        FixedArrayTestItemOversized::new(); // This line should panic
    }
}
