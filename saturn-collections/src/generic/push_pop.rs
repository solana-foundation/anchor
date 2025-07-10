//! Common trait for collections supporting push/pop operations.
//!
//! This module provides [`PushPopCollection`], a trait that abstracts over collections
//! that support stack-like operations, allowing generic code to work with both
//! fixed-capacity collections and standard library types like `Vec`.

/// Error type for push/pop operations.
#[derive(Debug)]
pub enum PushPopError {
    /// The collection has reached its maximum capacity.
    Full,
}

/// A trait for collections that support push/pop operations.
///
/// This trait provides a common interface for collections that can grow and shrink
/// by adding/removing elements at one end (typically the back). It's implemented
/// for both fixed-capacity collections (like [`FixedList`]) and standard library
/// collections (like [`Vec`]).
///
/// # Examples
///
/// ```rust
/// use saturn_collections::generic::{fixed_list::FixedList, push_pop::PushPopCollection};
///
/// fn work_with_collection<T: PushPopCollection<i32>>(collection: &mut T) {
///     collection.push(42).unwrap();
///     collection.push(100).unwrap();
///     assert_eq!(collection.len(), 2);
///     assert_eq!(collection.pop(), Some(100));
///     assert_eq!(collection.as_slice(), &[42]);
/// }
///
/// // Works with fixed-capacity collections
/// let mut list = FixedList::<i32, 10>::new();
/// work_with_collection(&mut list);
///
/// // Works with standard library collections
/// let mut vec = Vec::new();
/// work_with_collection(&mut vec);
/// ```
///
/// [`FixedList`]: crate::generic::fixed_list::FixedList
/// [`Vec`]: std::vec::Vec
pub trait PushPopCollection<T> {
    /// Adds an element to the collection.
    ///
    /// Returns [`PushPopError::Full`] if the collection is at capacity.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use saturn_collections::generic::{fixed_list::FixedList, push_pop::PushPopCollection};
    ///
    /// let mut list = FixedList::<i32, 2>::new();
    /// assert!(list.push(1).is_ok());
    /// assert!(list.push(2).is_ok());
    /// assert!(list.push(3).is_err()); // Full
    /// ```
    fn push(&mut self, item: T) -> Result<(), PushPopError>;

    /// Removes and returns the last element, or `None` if the collection is empty.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use saturn_collections::generic::{fixed_list::FixedList, push_pop::PushPopCollection};
    ///
    /// let mut list = FixedList::<i32, 5>::new();
    /// assert_eq!(list.pop(), None);
    ///
    /// list.push(42).unwrap();
    /// assert_eq!(list.pop(), Some(42));
    /// ```
    fn pop(&mut self) -> Option<T>;

    /// Returns a slice view of all elements in the collection.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use saturn_collections::generic::{fixed_list::FixedList, push_pop::PushPopCollection};
    ///
    /// let mut list = FixedList::<i32, 5>::new();
    /// list.push(1).unwrap();
    /// list.push(2).unwrap();
    ///
    /// assert_eq!(list.as_slice(), &[1, 2]);
    /// ```
    fn as_slice(&self) -> &[T];

    /// Returns the number of elements in the collection.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use saturn_collections::generic::{fixed_list::FixedList, push_pop::PushPopCollection};
    ///
    /// let mut list = FixedList::<i32, 5>::new();
    /// assert_eq!(list.len(), 0);
    ///
    /// list.push(42).unwrap();
    /// assert_eq!(list.len(), 1);
    /// ```
    fn len(&self) -> usize;
}

/// Implementation for standard library [`Vec`].
///
/// This implementation never returns [`PushPopError::Full`] since `Vec` can grow dynamically.
///
/// [`Vec`]: std::vec::Vec
impl<T> PushPopCollection<T> for Vec<T> {
    fn push(&mut self, item: T) -> Result<(), PushPopError> {
        self.push(item);
        Ok(())
    }

    fn pop(&mut self) -> Option<T> {
        self.pop()
    }

    fn as_slice(&self) -> &[T] {
        self.as_slice()
    }

    fn len(&self) -> usize {
        self.len()
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_push_and_len() {
        let mut collection: Vec<i32> = Vec::new();
        collection.push(1);
        collection.push(2);
        assert_eq!(collection.len(), 2);
    }

    #[test]
    fn test_pop() {
        let mut collection: Vec<i32> = vec![10, 20];
        let item = collection.pop();
        assert_eq!(item, Some(20));
        assert_eq!(collection.len(), 1);
    }

    #[test]
    fn test_pop_empty() {
        let mut collection: Vec<i32> = Vec::new();
        assert_eq!(collection.pop(), None);
    }

    #[test]
    fn test_as_slice() {
        let mut collection: Vec<i32> = Vec::new();
        collection.push(5);
        collection.push(10);
        collection.push(15);

        let slice = collection.as_slice();
        assert_eq!(slice, &[5, 10, 15]);
    }

    #[test]
    fn test_combined_operations() {
        let mut collection: Vec<i32> = Vec::new();
        collection.push(100);
        collection.push(200);
        collection.pop();
        collection.push(300);

        let slice = collection.as_slice();
        assert_eq!(slice, &[100, 300]);
        assert_eq!(collection.len(), 2);
    }
}
