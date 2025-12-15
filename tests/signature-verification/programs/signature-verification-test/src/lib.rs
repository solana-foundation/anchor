use anchor_lang::prelude::*;
use anchor_lang::signature_verification::{
    load_instruction, verify_ed25519_ix_with_instruction_index,
    verify_secp256k1_ix_with_instruction_index,
};

declare_id!("9P8zSbNRQkwDrjCmqsHHcU1GTk5npaKYgKHroAkupbLG");

#[program]
pub mod signature_verification_test {
    use super::*;

    pub fn verify_ed25519_signature(
        ctx: Context<VerifyEd25519Signature>,
        message: Vec<u8>,
        signature: [u8; 64],
    ) -> Result<()> {
        let ix = load_instruction(0, &ctx.accounts.ix_sysvar)?;
        verify_ed25519_ix_with_instruction_index(
            &ix,
            u16::MAX,
            &ctx.accounts.signer.key().to_bytes(),
            &message,
            &signature,
        )?;

        msg!("Ed25519 signature verified successfully using custom helper!");
        Ok(())
    }

    pub fn verify_secp(
        ctx: Context<VerifySecp256k1Signature>,
        message: Vec<u8>,
        signature: [u8; 64],
        recovery_id: u8,
        eth_address: [u8; 20],
    ) -> Result<()> {
        let ix = load_instruction(0, &ctx.accounts.ix_sysvar)?;
        verify_secp256k1_ix_with_instruction_index(
            &ix,
            0,
            &eth_address,
            &message,
            &signature,
            recovery_id,
        )?;

        msg!("Secp256k1 signature verified successfully using custom helper!");

        Ok(())
    }
}

#[derive(Accounts)]
pub struct VerifyEd25519Signature<'info> {
    /// CHECK: Signer account
    pub signer: AccountInfo<'info>,
    /// CHECK: Instructions sysvar account
    #[account(address = solana_sdk_ids::sysvar::instructions::ID)]
    pub ix_sysvar: AccountInfo<'info>,
}

#[derive(Accounts)]
pub struct VerifySecp256k1Signature<'info> {
    /// CHECK: Instructions sysvar account
    #[account(address = solana_sdk_ids::sysvar::instructions::ID)]
    pub ix_sysvar: AccountInfo<'info>,
}
