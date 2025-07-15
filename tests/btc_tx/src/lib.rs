use arch_program::utxo::UtxoMeta;
use bytemuck::{Pod, Zeroable};
use satellite_lang::accounts::shards::Shards;
use satellite_lang::satellite_bitcoin::utxo_info::UtxoInfo;
use satellite_lang::shard_set::ShardSet;
use satellite_lang::{context::BtcContext, prelude::*, ZeroCopy};

declare_id!("11111111111111111111111111111111");

#[program(btc_tx(max_inputs_to_sign = 4, max_modified_accounts = 4, rune_capacity = 1))]
pub mod btc_tx_test_program {
    use super::*;

    pub fn demo<'info>(
        mut ctx: BtcContext<'_, '_, '_, '_, 'info, Demo<'info>>,
        utxos: Vec<UtxoMeta>,
    ) -> Result<()> {
        let result = DemoUtxoParser::try_utxos(&mut ctx, &utxos)?;

        ctx.btc_tx_builder
            .add_state_transition(&ctx.accounts.together[0].to_account_info())?;

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

// pub struct DemoUtxoParser {
//     pub lol: Vec<UtxoInfo>,
// }
// impl<'info> satellite_lang::utxo_parser::TryFromUtxos<'info, Demo<'info>>
//     for DemoUtxoParser
// {
//     fn try_utxos(
//         ctx: &mut BtcContext<'_, '_, '_, '_, 'info, Demo<'info>>,
//         utxos: &[satellite_lang::arch_program::utxo::UtxoMeta],
//     ) -> core::result::Result<Self, satellite_lang::arch_program::program_error::ProgramError> {
//         let mut idx: usize = 0;
//         let total: usize = utxos.len();
//         for i in 0..total {
//             for j in (i + 1)..total {
//                 if utxos[i] == utxos[j] {
//                     return Err(ProgramError::Custom(ErrorCode::DuplicateUtxoMeta.into()));
//                 }
//             }
//         }
//         // let _ = {
//         //     fn _assert_indexable<T: core::ops::Index<usize>>(_t: &T) {}
//         //     _assert_indexable(&accounts.together);
//         // };
//         let target_len = ctx.accounts.together.len();
//         let mut lol: Vec<satellite_bitcoin::utxo_info::UtxoInfo> = Vec::with_capacity(target_len);
//         for i in 0..target_len {
//             if idx >= total {
//                 return Err(ProgramError::Custom(ErrorCode::MissingRequiredUtxo.into()));
//             }
//             let utxo = satellite_lang::utxo_parser::meta_to_info(&utxos[idx])?;
//             if !(utxo.value == (546) && utxo.rune_entry_count() == 0) {
//                 return Err(ProgramError::Custom(ErrorCode::InvalidRunesPresence.into()));
//             }
//             let account_info =
//                 satellite_lang::ToAccountInfo::to_account_info(&ctx.accounts.together[i]);
//             let _anchor_ix = arch_program::system_instruction::anchor(
//                 account_info.key,
//                 utxo.meta.txid_big_endian(),
//                 utxo.meta.vout(),
//             );
//             lol.push(utxo);
//             let together = satellite_lang::ToAccountInfo::to_account_info(&ctx.accounts.together[i]);
//             ctx.btc_tx_builder.add_state_transition(&together)?;
//             idx += 1;
//         }
//         if idx < total {
//             return Err(ProgramError::Custom(ErrorCode::UnexpectedExtraUtxos.into()));
//         }
//         Ok(Self { lol })
//     }
// }
