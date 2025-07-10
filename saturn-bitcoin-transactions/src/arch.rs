use arch_program::{
    account::{AccountInfo, MIN_ACCOUNT_LAMPORTS},
    program::{get_runes_from_output, invoke_signed},
    program_error::ProgramError,
    pubkey::Pubkey,
    rune::RuneAmount,
    system_instruction::create_account_with_anchor as create_account_instruction,
    utxo::UtxoMeta,
};
use saturn_collections::generic::fixed_set::FixedCapacitySet;

use crate::error::BitcoinTxError;

pub fn create_account<'a>(
    utxo: &UtxoMeta,
    account: &AccountInfo<'a>,
    system_program_id: &AccountInfo<'a>,
    fee_payer: &AccountInfo<'a>,
    program_id: &Pubkey,
    signer_seeds: &[&[u8]],
) -> Result<(), ProgramError> {
    let cpi_signer_seeds: &[&[&[u8]]] = &[signer_seeds];

    let instruction = create_account_instruction(
        fee_payer.key,
        account.key,
        MIN_ACCOUNT_LAMPORTS,
        0,
        program_id,
        utxo.txid_big_endian(),
        utxo.vout(),
    );

    invoke_signed(
        &instruction,
        &[
            account.clone(),
            fee_payer.clone(),
            system_program_id.clone(),
        ],
        cpi_signer_seeds,
    )?;

    Ok(())
}

pub fn get_runes<RS>(utxo: &UtxoMeta) -> Result<RS, ProgramError>
where
    RS: FixedCapacitySet<Item = RuneAmount> + Default,
{
    let txid = utxo.txid_big_endian();

    let runes = get_runes_from_output(txid, utxo.vout()).ok_or(ProgramError::Custom(
        BitcoinTxError::RuneOutputNotFound.into(),
    ))?;

    let mut rune_set = RS::default();
    for rune in runes.iter() {
        let rune_amount = RuneAmount {
            amount: rune.amount,
            id: rune.id,
        };

        rune_set
            .insert(rune_amount)
            .map_err(|_| BitcoinTxError::MoreRunesInUtxoThanMax)?;
    }

    Ok(rune_set)
}
