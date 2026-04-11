use quasar_lang::prelude::*;

#[account(discriminator = 1)]
#[seeds(b"multisig", creator: Address)]
pub struct MultisigConfig<'a> {
    pub creator: Address,
    pub threshold: u8,
    pub bump: u8,
    pub label: String<'a, 32>,
    pub signers: Vec<'a, Address, 10>,
}
