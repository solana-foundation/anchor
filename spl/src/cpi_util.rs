use anchor_lang::{
    pinocchio_runtime::{
        account_info::AccountInfo,
        instruction::{InstructionAccount, InstructionView},
        program::{invoke_signed_with_bounds, Signer, MAX_STATIC_CPI_ACCOUNTS},
    },
    Instruction,
};

/// Bridge function that converts a `solana_instruction::Instruction` into a
/// pinocchio `InstructionView` and invokes it via pinocchio's CPI.
pub fn invoke_signed_solana_instruction(
    ix: Instruction,
    account_infos: &[AccountInfo],
    signer_seeds: &[Signer],
) -> Result<(), anchor_lang::pinocchio_runtime::error::ProgramError> {
    let accounts: Vec<InstructionAccount> = ix
        .accounts
        .iter()
        .map(|meta| InstructionAccount::new(&meta.pubkey, meta.is_writable, meta.is_signer))
        .collect();

    let instruction_view = InstructionView {
        program_id: &ix.program_id,
        data: &ix.data,
        accounts: &accounts,
    };

    invoke_signed_with_bounds::<MAX_STATIC_CPI_ACCOUNTS, AccountInfo>(
        &instruction_view,
        account_infos,
        signer_seeds,
    )
}
