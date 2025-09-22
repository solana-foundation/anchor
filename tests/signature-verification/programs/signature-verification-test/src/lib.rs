use anchor_lang::prelude::*;
use anchor_lang::signature_verification::{
    load_instruction, verify_ed25519_ix, verify_secp256k1_ix,
};

declare_id!("9q9StGMtVHQtz14vH8YhPtED9MxnBj2mnVpBhYeTeRhj");

#[program]
pub mod signature_verification_test {
    use super::*;

    pub fn verify_ed25519_signature(
        ctx: Context<VerifyEd25519Signature>,
        message: Vec<u8>,
        signature: [u8; 64],
    ) -> Result<()> {
        let ix = load_instruction(0, &ctx.accounts.ix_sysvar)?;
        verify_ed25519_ix(
            &ix,
            &ctx.accounts.signer.key().to_bytes(),
            &message,
            &signature,
        )?;

        msg!("Ed25519 signature verified successfully using custom helper!");
        msg!("Signer: {}", ctx.accounts.signer.key());
        msg!("Message length: {}", message.len());

        Ok(())
    }

    pub fn verify_secp256k1_signature(
        ctx: Context<VerifySecp256k1Signature>,
        message_hash: [u8; 32],
        signature: [u8; 64],
        recovery_id: u8,
        eth_address: [u8; 20],
    ) -> Result<()> {
        let ix = load_instruction(0, &ctx.accounts.ix_sysvar)?;
        verify_secp256k1_ix(&ix, &eth_address, &message_hash, &signature, recovery_id)?;

        msg!("Secp256k1 signature verified successfully using custom helper!");
        msg!("Eth address: {:?}", eth_address);
        msg!("Message hash: {:?}", message_hash);

        Ok(())
    }
}

#[derive(Accounts)]
pub struct VerifyEd25519Signature<'info> {
    /// CHECK: This account represents the signer's public key
    pub signer: AccountInfo<'info>,
    /// CHECK: Instructions sysvar account
    #[account(address = anchor_lang::solana_program::sysvar::instructions::ID)]
    pub ix_sysvar: AccountInfo<'info>,
}

#[derive(Accounts)]
pub struct VerifySecp256k1Signature<'info> {
    /// CHECK: Instructions sysvar account
    #[account(address = anchor_lang::solana_program::sysvar::instructions::ID)]
    pub ix_sysvar: AccountInfo<'info>,
}

#[error_code]
pub enum ErrorCode {
    #[msg("Signature verification failed")]
    SignatureVerificationFailed,
}
