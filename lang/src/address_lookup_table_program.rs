use crate::prelude::*;
use crate::solana_program::address_lookup_table;
use solana_program::address_lookup_table::state::AddressLookupTable as SolanaAddressLookupTable;
use solana_program::pubkey::Pubkey;
use std::ops::{Deref, DerefMut};

#[derive(Debug, Clone)]
pub struct AddressLookupTable;
impl anchor_lang::Id for AddressLookupTable {
    fn id() -> Pubkey {
        address_lookup_table::program::ID
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AddressLookupTableAccount<'info>(SolanaAddressLookupTable<'info>);

impl AccountSerialize for AddressLookupTableAccount<'_> {}

impl<'info> AccountDeserialize for AddressLookupTableAccount<'info> {
    fn try_deserialize_unchecked(buf: &mut &[u8]) -> Result<Self> {
        // Deserialize into the temporary struct
        let table = SolanaAddressLookupTable::deserialize(buf)
            .map_err(|_| ProgramError::InvalidAccountData)?;

        // Construct a new AddressLookupTable with a lifetime tied to 'info
        let new_table = SolanaAddressLookupTable {
            meta: table.meta,
            addresses: std::borrow::Cow::Owned(table.addresses.into_owned()),
        };

        Ok(Self(new_table))
    }
}

impl<'info> Deref for AddressLookupTableAccount<'info> {
    type Target = SolanaAddressLookupTable<'info>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<'info> DerefMut for AddressLookupTableAccount<'info> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl Owner for AddressLookupTableAccount<'_> {
    fn owner() -> Pubkey {
        address_lookup_table::program::ID
    }
}

 #[cfg(feature = "idl-build")]
 mod idl_build {
     use super::*;

     impl crate::IdlBuild for AddressLookupTableAccount<'_> {}
     impl crate::Discriminator for AddressLookupTableAccount<'_> {
         const DISCRIMINATOR: &'static [u8] = &[];
     }
 }
