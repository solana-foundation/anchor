use quasar_lang::prelude::*;

#[account(discriminator = 1, set_inner)]
#[seeds(b"multisig", creator: Address)]
pub struct MultisigConfig {
    pub creator: Address,
    pub threshold: u8,
    pub bump: u8,
    pub label: String<32>,
    pub signers: Vec<Address, 10>,
}
