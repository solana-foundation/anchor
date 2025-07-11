use anchor_lang::accounts::shards::Shards;
use anchor_lang::satellite_bitcoin::utxo_info::UtxoInfo;
use anchor_lang::shard_set::ShardSet;
use anchor_lang::{context::BtcContext, prelude::*, ZeroCopy};
use arch_program::utxo::UtxoMeta;
use bytemuck::{Pod, Zeroable};

declare_id!("11111111111111111111111111111111");

#[program(btc_tx(max_inputs_to_sign = 4, max_modified_accounts = 4, rune_capacity = 1))]
pub mod btc_tx_test_program {
    use super::*;

    pub fn demo(mut ctx: BtcContext<Demo>, utxos: Vec<UtxoMeta>) -> Result<()> {
        // Ensure we can access the builder (compile-time guarantee – no unwrap needed)
        // let builder_ref = &mut ctx.btc_tx_builder;

        let utxos = DemoUtxoParser::try_utxos(&mut ctx, &utxos)?;

        // Build an *unselected* ShardSet from all shard loaders.
        // The lifetime is inferred, so we only need to specify the shard type and
        // the compile-time limit for how many shards can be selected.
        let mut shard_set: ShardSet<BtcTxBuilder, 10> = ShardSet::from(&ctx.accounts.together);

        let mut selected = shard_set.select_with([0, 1]).unwrap();
        Ok(())
    }
}

#[derive(Debug, Default, Clone, Copy, Pod, Zeroable)]
#[repr(C)]
pub struct BtcTxBuilder {
    pub done: u8,
}

impl ZeroCopy for BtcTxBuilder {}
impl Owner for BtcTxBuilder {
    fn owner() -> Pubkey {
        Pubkey::default()
    }
}

impl Discriminator for BtcTxBuilder {
    const DISCRIMINATOR: &'static [u8] = b"btc_tx_builder";
}

#[derive(Accounts)]
pub struct Demo<'info> {
    pub signer: Signer<'info>,
    #[account(shards = "rest", seeds = [b"together"], bump)]
    pub together: Shards<'info, AccountLoader<'info, BtcTxBuilder>>,
}

#[derive(UtxoParser)]
#[utxo_accounts(Demo)]
pub struct DemoUtxoParser {
    #[utxo(anchor = together, value = 546, runes = "none")]
    pub lol: Vec<UtxoInfo>,
}

// No custom error codes needed – builder is always present now.

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn compile_test() {
        // Just ensure the program module compiles and the demo handler type-checks.
        assert_eq!(crate::ID, crate::id());
    }
}
