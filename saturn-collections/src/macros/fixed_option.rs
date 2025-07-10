use bytemuck::{Pod, Zeroable};

#[macro_export]
macro_rules! declare_fixed_option {
    ($Name:ident, $T:ty, $padding:expr) => {
        #[repr(C)]
        #[derive(
            Clone,
            Copy,
            Debug,
            // PartialEq,
            // Eq,
            bytemuck::Pod,
            bytemuck::Zeroable,
        )]
        pub struct $Name {
            item: $T,
            present: u8,
            _padding: [u8; $padding],
        }

        // Compile-time layout check
        const _: () = {
            use core::mem::{align_of, size_of};

            const _ALIGN: usize = align_of::<$Name>();
            const _SIZE: usize = size_of::<$Name>();
            const _ITEM_SIZE: usize = size_of::<$T>();
            const _EXPECTED_SIZE: usize = _ITEM_SIZE + 1 + $padding;
            const _: () = assert!(
                _SIZE == _EXPECTED_SIZE,
                "Size mismatch in FixedOption struct!"
            );
        };

        impl $Name {
            pub fn none() -> Self {
                Self {
                    item: <$T as bytemuck::Zeroable>::zeroed(),
                    present: 0,
                    _padding: [0; $padding],
                }
            }

            pub fn some(data: $T) -> Self {
                Self {
                    item: data,
                    present: 1,
                    _padding: [0; $padding],
                }
            }

            pub fn is_some(&self) -> bool {
                self.present != 0
            }

            pub fn is_none(&self) -> bool {
                self.present == 0
            }

            pub fn get(&self) -> Option<$T> {
                if self.is_some() {
                    Some(self.item.clone())
                } else {
                    None
                }
            }

            pub fn as_ref(&self) -> Option<&$T> {
                if self.is_some() {
                    Some(&self.item)
                } else {
                    None
                }
            }

            pub fn as_mut(&mut self) -> Option<&mut $T> {
                if self.is_some() {
                    Some(&mut self.item)
                } else {
                    None
                }
            }

            pub fn unwrap(self) -> $T {
                let option: Option<_> = self.into();

                option.unwrap()
            }
        }

        impl Default for $Name {
            fn default() -> Self {
                Self::none()
            }
        }

        impl From<$Name> for Option<$T> {
            fn from(item: $Name) -> Option<$T> {
                if item.is_some() {
                    Some(item.item)
                } else {
                    None
                }
            }
        }

        impl From<Option<$T>> for $Name {
            fn from(item: Option<$T>) -> $Name {
                match item {
                    Some(data) => $Name::some(data),
                    None => $Name::none(),
                }
            }
        }

        impl From<Option<&$T>> for $Name {
            fn from(item: Option<&$T>) -> $Name {
                match item {
                    Some(data) => $Name::some(data.clone()),
                    None => $Name::none(),
                }
            }
        }
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    // Dummy UtxoInfo for testing
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Pod, Zeroable)]
    #[repr(C)]
    pub struct DummyUtxoInfo {
        pub value: u64,
    }

    declare_fixed_option!(TestFixedOptionUtxoInfo, DummyUtxoInfo, 15);

    #[test]
    fn test_fixed_option_none() {
        let none_utxo = TestFixedOptionUtxoInfo::none();
        assert!(none_utxo.is_none());
        assert_eq!(none_utxo.get(), None);
    }

    #[test]
    fn test_fixed_option_some_and_get() {
        let utxo = DummyUtxoInfo { value: 999 };
        let opt = TestFixedOptionUtxoInfo::some(utxo);
        assert!(opt.is_some());
        assert_eq!(opt.get(), Some(utxo));
    }

    #[test]
    fn test_fixed_option_default() {
        let def: TestFixedOptionUtxoInfo = Default::default();
        assert!(def.is_none());
    }
}
