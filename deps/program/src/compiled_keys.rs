use {
    crate::{instruction::Instruction, pubkey::Pubkey, sanitized::MessageHeader},
    std::collections::BTreeMap,
    thiserror::Error,
};

/// A helper struct to collect pubkeys compiled for a set of instructions
#[derive(Default, Debug, Clone, PartialEq, Eq)]
pub(crate) struct CompiledKeys {
    payer: Option<Pubkey>,
    key_meta_map: BTreeMap<Pubkey, CompiledKeyMeta>,
}

#[cfg_attr(target_os = "solana", allow(dead_code))]
#[derive(PartialEq, Debug, Error, Eq, Clone)]
pub enum CompileError {
    #[error("account index overflowed during compilation")]
    AccountIndexOverflow,
    #[error("address lookup table index overflowed during compilation")]
    AddressTableLookupIndexOverflow,
    #[error("encountered unknown account key `{0}` during instruction compilation")]
    UnknownInstructionKey(Pubkey),
}

#[derive(Default, Debug, Clone, PartialEq, Eq)]
struct CompiledKeyMeta {
    is_signer: bool,
    is_writable: bool,
    is_invoked: bool,
}

impl CompiledKeys {
    /// Compiles the pubkeys referenced by a list of instructions and organizes by
    /// signer/non-signer and writable/readonly.
    pub(crate) fn compile(instructions: &[Instruction], payer: Option<Pubkey>) -> Self {
        let mut key_meta_map = BTreeMap::<Pubkey, CompiledKeyMeta>::new();
        for ix in instructions {
            let meta = key_meta_map.entry(ix.program_id).or_default();
            meta.is_invoked = true;
            for account_meta in &ix.accounts {
                let meta = key_meta_map.entry(account_meta.pubkey).or_default();
                meta.is_signer |= account_meta.is_signer;
                meta.is_writable |= account_meta.is_writable;
            }
        }
        if let Some(payer) = &payer {
            let meta = key_meta_map.entry(*payer).or_default();
            meta.is_signer = true;
            meta.is_writable = true;
        }
        Self {
            payer,
            key_meta_map,
        }
    }

    pub(crate) fn try_into_message_components(
        self,
    ) -> Result<(MessageHeader, Vec<Pubkey>), CompileError> {
        let try_into_u8 = |num: usize| -> Result<u8, CompileError> {
            u8::try_from(num).map_err(|_| CompileError::AccountIndexOverflow)
        };

        let Self {
            payer,
            mut key_meta_map,
        } = self;

        if let Some(payer) = &payer {
            key_meta_map.remove_entry(payer);
        }

        let writable_signer_keys: Vec<Pubkey> = payer
            .into_iter()
            .chain(
                key_meta_map
                    .iter()
                    .filter_map(|(key, meta)| (meta.is_signer && meta.is_writable).then_some(*key)),
            )
            .collect();
        let readonly_signer_keys: Vec<Pubkey> = key_meta_map
            .iter()
            .filter_map(|(key, meta)| (meta.is_signer && !meta.is_writable).then_some(*key))
            .collect();
        let writable_non_signer_keys: Vec<Pubkey> = key_meta_map
            .iter()
            .filter_map(|(key, meta)| (!meta.is_signer && meta.is_writable).then_some(*key))
            .collect();
        let readonly_non_signer_keys: Vec<Pubkey> = key_meta_map
            .iter()
            .filter_map(|(key, meta)| (!meta.is_signer && !meta.is_writable).then_some(*key))
            .collect();

        let signers_len = writable_signer_keys
            .len()
            .saturating_add(readonly_signer_keys.len());

        let header = MessageHeader {
            num_required_signatures: try_into_u8(signers_len)?,
            num_readonly_signed_accounts: try_into_u8(readonly_signer_keys.len())?,
            num_readonly_unsigned_accounts: try_into_u8(readonly_non_signer_keys.len())?,
        };

        let static_account_keys = std::iter::empty()
            .chain(writable_signer_keys)
            .chain(readonly_signer_keys)
            .chain(writable_non_signer_keys)
            .chain(readonly_non_signer_keys)
            .collect();

        Ok((header, static_account_keys))
    }
}

#[cfg(test)]
mod tests {
    use {super::*, crate::account::AccountMeta, bitflags::bitflags};

    bitflags! {
        #[derive(Clone, Copy)]
        pub struct KeyFlags: u8 {
            const SIGNER   = 0b00000001;
            const WRITABLE = 0b00000010;
            const INVOKED  = 0b00000100;
        }
    }

    impl From<KeyFlags> for CompiledKeyMeta {
        fn from(flags: KeyFlags) -> Self {
            Self {
                is_signer: flags.contains(KeyFlags::SIGNER),
                is_writable: flags.contains(KeyFlags::WRITABLE),
                is_invoked: flags.contains(KeyFlags::INVOKED),
            }
        }
    }

    #[test]
    fn test_compile_with_dups() {
        let program_id0 = Pubkey::new_unique();
        let program_id1 = Pubkey::new_unique();
        let program_id2 = Pubkey::new_unique();
        let program_id3 = Pubkey::new_unique();
        let id0 = Pubkey::new_unique();
        let id1 = Pubkey::new_unique();
        let id2 = Pubkey::new_unique();
        let id3 = Pubkey::new_unique();
        let compiled_keys = CompiledKeys::compile(
            &[
                Instruction::new(
                    program_id0,
                    vec![0],
                    vec![
                        AccountMeta::new_readonly(id0, false),
                        AccountMeta::new_readonly(id1, true),
                        AccountMeta::new(id2, false),
                        AccountMeta::new(id3, true),
                        // duplicate the account inputs
                        AccountMeta::new_readonly(id0, false),
                        AccountMeta::new_readonly(id1, true),
                        AccountMeta::new(id2, false),
                        AccountMeta::new(id3, true),
                        // reference program ids
                        AccountMeta::new_readonly(program_id0, false),
                        AccountMeta::new_readonly(program_id1, true),
                        AccountMeta::new(program_id2, false),
                        AccountMeta::new(program_id3, true),
                    ],
                ),
                Instruction::new(program_id1, vec![0], vec![]),
                Instruction::new(program_id2, vec![0], vec![]),
                Instruction::new(program_id3, vec![0], vec![]),
            ],
            None,
        );

        assert_eq!(
            compiled_keys,
            CompiledKeys {
                key_meta_map: BTreeMap::from([
                    (id0, KeyFlags::empty().into()),
                    (id1, KeyFlags::SIGNER.into()),
                    (id2, KeyFlags::WRITABLE.into()),
                    (id3, (KeyFlags::SIGNER | KeyFlags::WRITABLE).into()),
                    (program_id0, KeyFlags::INVOKED.into()),
                    (program_id1, (KeyFlags::INVOKED | KeyFlags::SIGNER).into()),
                    (program_id2, (KeyFlags::INVOKED | KeyFlags::WRITABLE).into()),
                    (program_id3, KeyFlags::all().into()),
                ]),
                payer: None,
            }
        );
    }

    #[test]
    fn test_compile_with_dup_signer_mismatch() {
        let program_id = Pubkey::new_unique();
        let id0 = Pubkey::new_unique();
        let compiled_keys = CompiledKeys::compile(
            &[Instruction::new(
                program_id,
                vec![0],
                vec![AccountMeta::new(id0, false), AccountMeta::new(id0, true)],
            )],
            None,
        );

        // Ensure the dup writable key is a signer
        assert_eq!(
            compiled_keys,
            CompiledKeys {
                key_meta_map: BTreeMap::from([
                    (program_id, KeyFlags::INVOKED.into()),
                    (id0, (KeyFlags::SIGNER | KeyFlags::WRITABLE).into())
                ]),
                payer: None,
            }
        );
    }

    #[test]
    fn test_compile_with_dup_signer_writable_mismatch() {
        let program_id = Pubkey::new_unique();
        let id0 = Pubkey::new_unique();
        let compiled_keys = CompiledKeys::compile(
            &[Instruction::new(
                program_id,
                vec![0],
                vec![
                    AccountMeta::new_readonly(id0, true),
                    AccountMeta::new(id0, true),
                ],
            )],
            None,
        );

        // Ensure the dup signer key is writable
        assert_eq!(
            compiled_keys,
            CompiledKeys {
                key_meta_map: BTreeMap::from([
                    (id0, (KeyFlags::SIGNER | KeyFlags::WRITABLE).into()),
                    (program_id, KeyFlags::INVOKED.into()),
                ]),
                payer: None,
            }
        );
    }

    #[test]
    fn test_compile_with_dup_nonsigner_writable_mismatch() {
        let program_id = Pubkey::new_unique();
        let id0 = Pubkey::new_unique();
        let compiled_keys = CompiledKeys::compile(
            &[
                Instruction::new(
                    program_id,
                    vec![0],
                    vec![
                        AccountMeta::new_readonly(id0, false),
                        AccountMeta::new(id0, false),
                    ],
                ),
                Instruction::new(program_id, vec![0], vec![AccountMeta::new(id0, false)]),
            ],
            None,
        );

        // Ensure the dup nonsigner key is writable
        assert_eq!(
            compiled_keys,
            CompiledKeys {
                key_meta_map: BTreeMap::from([
                    (id0, KeyFlags::WRITABLE.into()),
                    (program_id, KeyFlags::INVOKED.into()),
                ]),
                payer: None,
            }
        );
    }

    #[test]
    fn test_try_into_message_components() {
        let keys = vec![
            Pubkey::new_unique(),
            Pubkey::new_unique(),
            Pubkey::new_unique(),
            Pubkey::new_unique(),
        ];

        let compiled_keys = CompiledKeys {
            key_meta_map: BTreeMap::from([
                (keys[0], (KeyFlags::SIGNER | KeyFlags::WRITABLE).into()),
                (keys[1], KeyFlags::SIGNER.into()),
                (keys[2], KeyFlags::WRITABLE.into()),
                (keys[3], KeyFlags::empty().into()),
            ]),
            payer: None,
        };

        let result = compiled_keys.try_into_message_components();
        assert_eq!(result.as_ref().err(), None);
        let (header, static_keys) = result.unwrap();

        assert_eq!(static_keys, keys);
        assert_eq!(
            header,
            MessageHeader {
                num_required_signatures: 2,
                num_readonly_signed_accounts: 1,
                num_readonly_unsigned_accounts: 1,
            }
        );
    }

    #[test]
    fn test_try_into_message_components_with_too_many_keys() {
        const TOO_MANY_KEYS: usize = 257;

        for key_flags in [
            KeyFlags::WRITABLE | KeyFlags::SIGNER,
            KeyFlags::SIGNER,
            // skip writable_non_signer_keys because it isn't used for creating header values
            KeyFlags::empty(),
        ] {
            let key_meta_map = BTreeMap::from_iter(
                (0..TOO_MANY_KEYS).map(|_| (Pubkey::new_unique(), key_flags.into())),
            );
            let payer = key_meta_map.keys().next().unwrap().clone();
            let test_keys = CompiledKeys {
                key_meta_map,
                payer: Some(payer),
            };

            assert_eq!(
                test_keys.try_into_message_components(),
                Err(CompileError::AccountIndexOverflow)
            );
        }
    }
}
