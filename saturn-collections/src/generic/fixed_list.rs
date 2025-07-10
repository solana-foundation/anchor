//! Fixed-capacity list implementation for allocation-free environments.
//!
//! This module provides [`FixedList`], a contiguous array-backed list with stack-like
//! `push`/`pop` semantics and compile-time capacity bounds.

use crate::generic::push_pop::{PushPopCollection, PushPopError};

/// Error type for [`FixedList`] operations.
#[derive(Debug)]
pub enum FixedListError {
    /// The list has reached its maximum capacity.
    Full,
}

/// A fixed-capacity list backed by a contiguous array.
///
/// `FixedList` provides stack-like operations (`push`/`pop`) with compile-time capacity bounds.
/// It maintains insertion order and allows efficient element access via slices.
///
/// # Type Parameters
///
/// * `T` - The element type. Must implement `Default + Copy` for initialization.
/// * `SIZE` - The maximum number of elements the list can hold (compile-time constant).
///
/// # Examples
///
/// ```rust
/// use saturn_collections::generic::fixed_list::FixedList;
///
/// let mut list: FixedList<u32, 4> = FixedList::new();
///
/// list.push(10).unwrap();
/// list.push(20).unwrap();
/// assert_eq!(list.len(), 2);
/// assert_eq!(list.as_slice(), &[10, 20]);
///
/// assert_eq!(list.pop(), Some(20));
/// assert_eq!(list.len(), 1);
/// ```
///
/// # Memory Layout
///
/// The list stores elements in a contiguous array followed by a length field.
/// Only the first `len` elements are considered valid; the rest contain default values.
#[derive(Debug)]
pub struct FixedList<T, const SIZE: usize> {
    items: [T; SIZE],
    len: usize,
}

impl<T: Default + Copy, const SIZE: usize> Default for FixedList<T, SIZE> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T: Default + Copy, const SIZE: usize> FixedList<T, SIZE> {
    /// Creates a new, empty `FixedList`.
    ///
    /// All elements are initialized to their default values, but only the first `len`
    /// elements are considered valid.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use saturn_collections::generic::fixed_list::FixedList;
    ///
    /// let list: FixedList<u32, 10> = FixedList::new();
    /// assert!(list.is_empty());
    /// assert_eq!(list.len(), 0);
    /// ```
    pub fn new() -> Self {
        Self {
            items: [T::default(); SIZE],
            len: 0,
        }
    }

    /// Creates a `FixedList` from a slice, copying elements up to the capacity.
    ///
    /// If the slice is longer than the capacity, only the first `SIZE` elements are copied.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use saturn_collections::generic::fixed_list::FixedList;
    ///
    /// let data = [1, 2, 3];
    /// let list: FixedList<i32, 5> = FixedList::from_slice(&data);
    /// assert_eq!(list.len(), 3);
    /// assert_eq!(list.as_slice(), &[1, 2, 3]);
    /// ```
    pub fn from_slice(slice: &[T]) -> Self {
        let mut list = Self::new();
        list.copy_from_slice(slice);
        list
    }

    /// Creates a `FixedList` from an iterator, taking up to `SIZE` elements.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use saturn_collections::generic::fixed_list::FixedList;
    ///
    /// let list: FixedList<u32, 3> = FixedList::from_iter(0..10);
    /// assert_eq!(list.len(), 3);
    /// assert_eq!(list.as_slice(), &[0, 1, 2]);
    /// ```
    pub fn from_iter<I: Iterator<Item = T>>(iter: I) -> Self {
        let mut list = Self::new();

        for item in iter.take(SIZE) {
            let _ = list.push(item);
        }

        list
    }

    /// Returns the number of elements in the list.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use saturn_collections::generic::fixed_list::FixedList;
    ///
    /// let mut list: FixedList<u32, 5> = FixedList::new();
    /// assert_eq!(list.len(), 0);
    ///
    /// list.push(42).unwrap();
    /// assert_eq!(list.len(), 1);
    /// ```
    pub fn len(&self) -> usize {
        self.len
    }

    /// Returns `true` if the list contains no elements.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use saturn_collections::generic::fixed_list::FixedList;
    ///
    /// let mut list: FixedList<u32, 5> = FixedList::new();
    /// assert!(list.is_empty());
    ///
    /// list.push(42).unwrap();
    /// assert!(!list.is_empty());
    /// ```
    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    /// Returns an iterator over the elements in the list.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use saturn_collections::generic::fixed_list::FixedList;
    ///
    /// let mut list: FixedList<u32, 5> = FixedList::new();
    /// list.push(1).unwrap();
    /// list.push(2).unwrap();
    ///
    /// let collected: Vec<_> = list.iter().copied().collect();
    /// assert_eq!(collected, vec![1, 2]);
    /// ```
    pub fn iter(&self) -> impl Iterator<Item = &T> + '_ {
        self.items[..self.len].iter()
    }

    /// Returns a mutable iterator over the elements in the list.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use saturn_collections::generic::fixed_list::FixedList;
    ///
    /// let mut list: FixedList<u32, 5> = FixedList::new();
    /// list.push(1).unwrap();
    /// list.push(2).unwrap();
    ///
    /// for item in list.iter_mut() {
    ///     *item *= 10;
    /// }
    /// assert_eq!(list.as_slice(), &[10, 20]);
    /// ```
    pub fn iter_mut(&mut self) -> impl Iterator<Item = &mut T> + '_ {
        self.items[..self.len].iter_mut()
    }

    /// Appends an element to the back of the list.
    ///
    /// # Errors
    ///
    /// Returns [`FixedListError::Full`] if the list is at capacity.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use saturn_collections::generic::fixed_list::FixedList;
    ///
    /// let mut list: FixedList<u32, 2> = FixedList::new();
    /// assert!(list.push(1).is_ok());
    /// assert!(list.push(2).is_ok());
    /// assert!(list.push(3).is_err()); // List is full
    /// ```
    pub fn push(&mut self, item: T) -> Result<(), FixedListError> {
        if self.len >= SIZE {
            return Err(FixedListError::Full);
        }

        self.items[self.len] = item;
        self.len += 1;

        Ok(())
    }

    /// Removes and returns the last element, or `None` if the list is empty.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use saturn_collections::generic::fixed_list::FixedList;
    ///
    /// let mut list: FixedList<u32, 5> = FixedList::new();
    /// assert_eq!(list.pop(), None);
    ///
    /// list.push(42).unwrap();
    /// assert_eq!(list.pop(), Some(42));
    /// ```
    pub fn pop(&mut self) -> Option<T> {
        if self.len > 0 {
            self.len -= 1;
            Some(self.items[self.len])
        } else {
            None
        }
    }

    /// Returns a slice containing all elements in the list.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use saturn_collections::generic::fixed_list::FixedList;
    ///
    /// let mut list: FixedList<u32, 5> = FixedList::new();
    /// list.push(1).unwrap();
    /// list.push(2).unwrap();
    ///
    /// let slice = list.as_slice();
    /// assert_eq!(slice, &[1, 2]);
    /// ```
    pub fn as_slice(&self) -> &[T] {
        &self.items[..self.len]
    }

    /// Returns a mutable slice containing all elements in the list.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use saturn_collections::generic::fixed_list::FixedList;
    ///
    /// let mut list: FixedList<u32, 5> = FixedList::new();
    /// list.push(1).unwrap();
    /// list.push(2).unwrap();
    ///
    /// let slice = list.as_mut_slice();
    /// slice[0] = 10;
    /// assert_eq!(list.as_slice(), &[10, 2]);
    /// ```
    pub fn as_mut_slice(&mut self) -> &mut [T] {
        &mut self.items[..self.len]
    }

    /// Copies elements from a slice into the list, replacing existing contents.
    ///
    /// The list length is updated to match the slice length (up to capacity).
    ///
    /// # Examples
    ///
    /// ```rust
    /// use saturn_collections::generic::fixed_list::FixedList;
    ///
    /// let mut list: FixedList<u32, 5> = FixedList::new();
    /// let data = [10, 20, 30];
    /// list.copy_from_slice(&data);
    ///
    /// assert_eq!(list.len(), 3);
    /// assert_eq!(list.as_slice(), &[10, 20, 30]);
    /// ```
    pub fn copy_from_slice(&mut self, slice: &[T])
    where
        T: Clone,
    {
        self.len = slice.len();
        self.items[..self.len].clone_from_slice(slice);
    }
}

impl<T: Default + Copy, const SIZE: usize> PushPopCollection<T> for FixedList<T, SIZE> {
    fn push(&mut self, item: T) -> Result<(), PushPopError> {
        self.push(item).map_err(|_| PushPopError::Full)
    }

    fn pop(&mut self) -> Option<T> {
        self.pop()
    }

    fn as_slice(&self) -> &[T] {
        self.as_slice()
    }

    fn len(&self) -> usize {
        self.len
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_is_empty() {
        let list = FixedList::<u32, 4>::default();
        assert_eq!(list.len(), 0);
        assert!(list.is_empty());
    }

    #[test]
    fn test_push_and_len() {
        let mut list = FixedList::<u32, 3>::new();
        list.push(10).unwrap();
        list.push(20).unwrap();
        assert_eq!(list.len(), 2);
        assert!(!list.is_empty());
        assert_eq!(list.as_slice(), &[10, 20]);
    }

    #[test]
    fn test_push_past_capacity() {
        let mut list = FixedList::<u32, 2>::new();
        list.push(1).unwrap();
        list.push(2).unwrap();
        list.push(3).unwrap_err(); // should be ignored
        assert_eq!(list.len(), 2);
        assert_eq!(list.as_slice(), &[1, 2]);
    }

    #[test]
    fn test_pop() {
        let mut list = FixedList::<u32, 2>::new();
        assert_eq!(list.pop(), None);

        list.push(100).unwrap();
        list.push(200).unwrap();
        assert_eq!(list.pop(), Some(200));
        assert_eq!(list.pop(), Some(100));
        assert_eq!(list.pop(), None);
        assert!(list.is_empty());
    }

    #[test]
    fn test_iter() {
        let mut list = FixedList::<u32, 3>::new();
        list.push(1).unwrap();
        list.push(2).unwrap();
        let collected: Vec<_> = list.iter().copied().collect();
        assert_eq!(collected, vec![1, 2]);
    }

    #[test]
    fn test_iter_mut() {
        let mut list = FixedList::<u32, 3>::new();
        list.push(1).unwrap();
        list.push(2).unwrap();
        for x in list.iter_mut() {
            *x *= 10;
        }
        assert_eq!(list.as_slice(), &[10, 20]);
    }

    #[test]
    fn test_as_slice_and_mut_slice() {
        let mut list = FixedList::<u32, 4>::new();
        list.push(42).unwrap();
        list.push(99).unwrap();
        let slice = list.as_slice();
        assert_eq!(slice, &[42, 99]);

        let mut_slice = list.as_mut_slice();
        mut_slice[0] = 123;
        assert_eq!(list.as_slice(), &[123, 99]);
    }

    #[test]
    fn test_copy_from_slice() {
        let mut list = FixedList::<u32, 5>::new();
        let data = [9, 8, 7];
        list.copy_from_slice(&data);
        assert_eq!(list.len(), 3);
        assert_eq!(list.as_slice(), &data);
    }

    #[test]
    fn test_push_pop_collection_trait() {
        let mut list = FixedList::<u8, 2>::new();
        PushPopCollection::push(&mut list, 1).unwrap();
        PushPopCollection::push(&mut list, 2).unwrap();
        PushPopCollection::push(&mut list, 3).unwrap_err(); // should be ignored
        assert_eq!(PushPopCollection::len(&list), 2);
        assert_eq!(PushPopCollection::as_slice(&list), &[1, 2]);
        assert_eq!(PushPopCollection::pop(&mut list), Some(2));
        assert_eq!(PushPopCollection::pop(&mut list), Some(1));
        assert_eq!(PushPopCollection::pop(&mut list), None);
    }
}
