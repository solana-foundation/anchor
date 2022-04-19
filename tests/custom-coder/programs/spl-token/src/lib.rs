use anchor_lang::prelude::*;

declare_id!("FmpfPa1LHEYRbueNMnwNVd2JvyQ89GXGWdyZEXNNKV8w");

// This program is simply used to generate the IDL for the token program.
//
// Note that we manually add the COption<Pubkey> type to the IDL after
// compiling.
//
#[program]
pub mod spl_token {
    use super::*;

    pub fn initialize_mint(
        ctx: Context<InitializeMint>,
        decimals: u8,
        mint_authority: Pubkey,
        //        freeze_authority: COption<Pubkey>,
    ) -> Result<()> {
        Ok(())
    }

    pub fn initialize_account(ctx: Context<InitializeAccount>) -> Result<()> {
        Ok(())
    }

    pub fn initialize_multisig(ctx: Context<InitializeMultisig>, m: u8) -> Result<()> {
        Ok(())
    }

    pub fn transfer(ctx: Context<Transfer>, amount: u64) -> Result<()> {
        Ok(())
    }

    pub fn approve(ctx: Context<Approve>, amount: u64) -> Result<()> {
        Ok(())
    }

    pub fn revoke(ctx: Context<Revoke>) -> Result<()> {
        Ok(())
    }

    pub fn set_authority(
        ctx: Context<SetAuthority>,
        authority_type: u8,
        //        new_authority: COption<Pubkey>,
    ) -> Result<()> {
        Ok(())
    }

    pub fn mint_to(ctx: Context<MintTo>, amount: u64) -> Result<()> {
        Ok(())
    }

    pub fn burn(ctx: Context<Burn>, amount: u64) -> Result<()> {
        Ok(())
    }

    pub fn close_account(ctx: Context<CloseAccount>) -> Result<()> {
        Ok(())
    }

    pub fn freeze_account(ctx: Context<FreezeAccount>) -> Result<()> {
        Ok(())
    }

    pub fn thaw_account(ctx: Context<ThawAccount>) -> Result<()> {
        Ok(())
    }

    pub fn transfer_checked(
        ctx: Context<TransferChecked>,
        amount: u64,
        decimals: u8,
    ) -> Result<()> {
        Ok(())
    }

    pub fn approve_checked(
        ctx: Context<ApproveChecked>,
        amount: u64,
        decimals: u8,
    ) -> Result<()> {
        Ok(())
    }

    pub fn mint_to_checked(
        ctx: Context<MintToChecked>,
        amount: u64,
        decimals: u8,
    ) -> Result<()> {
        Ok(())
    }

    pub fn burn_checked(ctx: Context<BurnChecked>, amount: u64, decimals: u8) -> Result<()> {
        Ok(())
    }

    pub fn initialize_account_2(
        ctx: Context<InitializeAccount2>,
        authority: Pubkey,
    ) -> Result<()> {
        Ok(())
    }

    pub fn sync_native(ctx: Context<SyncNative>) -> Result<()> {
        Ok(())
    }

    pub fn initialize_account3(
        ctx: Context<InitializeAccount3>,
        authority: Pubkey,
    ) -> Result<()> {
        Ok(())
    }

    pub fn initialize_multisig_2(ctx: Context<InitializeMultisig2>, m: u8) -> Result<()> {
        Ok(())
    }

    pub fn initialize_mint_2(
        ctx: Context<InitializeMint2>,
        decimals: u8,
        mint_authority: Pubkey,
        //        freeze_authority: COption<Pubkey>,
    ) -> Result<()> {
        Ok(())
    }
}

#[derive(Accounts)]
pub struct InitializeMint<'info> {
    #[account(mut)]
    mint: AccountInfo<'info>,
    rent: AccountInfo<'info>,
}

#[derive(Accounts)]
pub struct InitializeAccount<'info> {
    #[account(mut)]
    account: AccountInfo<'info>,
    mint: AccountInfo<'info>,
    authority: AccountInfo<'info>,
    rent: AccountInfo<'info>,
}

#[derive(Accounts)]
pub struct InitializeMultisig<'info> {
    #[account(mut)]
    account: AccountInfo<'info>,
    rent: AccountInfo<'info>,
}

#[derive(Accounts)]
pub struct Transfer<'info> {
    #[account(mut)]
    source: AccountInfo<'info>,
    #[account(mut)]
    destination: AccountInfo<'info>,
    authority: Signer<'info>,
}

#[derive(Accounts)]
pub struct Approve<'info> {
    #[account(mut)]
    source: AccountInfo<'info>,
    delegate: AccountInfo<'info>,
    authority: Signer<'info>,
}

#[derive(Accounts)]
pub struct Revoke<'info> {
    #[account(mut)]
    source: AccountInfo<'info>,
    authority: Signer<'info>,
}

#[derive(Accounts)]
pub struct SetAuthority<'info> {
    #[account(mut)]
    pub mint: AccountInfo<'info>,
    pub authority: Signer<'info>,
}

#[derive(Accounts)]
pub struct MintTo<'info> {
    #[account(mut)]
    pub mint: AccountInfo<'info>,
    #[account(mut)]
    pub to: AccountInfo<'info>,
    pub authority: Signer<'info>,
}

#[derive(Accounts)]
pub struct Burn<'info> {
    #[account(mut)]
    source: AccountInfo<'info>,
    #[account(mut)]
    mint: AccountInfo<'info>,
    authority: Signer<'info>,
}

#[derive(Accounts)]
pub struct CloseAccount<'info> {
    #[account(mut)]
    account: AccountInfo<'info>,
    #[account(mut)]
    destination: AccountInfo<'info>,
    authority: AccountInfo<'info>,
}

#[derive(Accounts)]
pub struct FreezeAccount<'info> {
    #[account(mut)]
    account: AccountInfo<'info>,
    mint: AccountInfo<'info>,
    authority: Signer<'info>,
}

#[derive(Accounts)]
pub struct ThawAccount<'info> {
    #[account(mut)]
    account: AccountInfo<'info>,
    mint: AccountInfo<'info>,
    authority: Signer<'info>,
}

#[derive(Accounts)]
pub struct TransferChecked<'info> {
    #[account(mut)]
    source: AccountInfo<'info>,
    mint: AccountInfo<'info>,
    #[account(mut)]
    destination: AccountInfo<'info>,
    authority: Signer<'info>,
}

#[derive(Accounts)]
pub struct ApproveChecked<'info> {
    #[account(mut)]
    source: AccountInfo<'info>,
    mint: AccountInfo<'info>,
    delegate: AccountInfo<'info>,
    authority: Signer<'info>,
}

#[derive(Accounts)]
pub struct MintToChecked<'info> {
    #[account(mut)]
    mint: AccountInfo<'info>,
    #[account(mut)]
    to: AccountInfo<'info>,
    authority: Signer<'info>,
}

#[derive(Accounts)]
pub struct BurnChecked<'info> {
    #[account(mut)]
    source: AccountInfo<'info>,
    #[account(mut)]
    mint: AccountInfo<'info>,
    authority: Signer<'info>,
}

#[derive(Accounts)]
pub struct InitializeAccount2<'info> {
    #[account(mut)]
    account: AccountInfo<'info>,
    mint: AccountInfo<'info>,
    rent: AccountInfo<'info>,
}

#[derive(Accounts)]
pub struct SyncNative<'info> {
    #[account(mut)]
    account: AccountInfo<'info>,
}

#[derive(Accounts)]
pub struct InitializeAccount3<'info> {
    #[account(mut)]
    account: AccountInfo<'info>,
    mint: AccountInfo<'info>,
}

#[derive(Accounts)]
pub struct InitializeMultisig2<'info> {
    #[account(mut)]
    account: AccountInfo<'info>,
}

#[derive(Accounts)]
pub struct InitializeMint2<'info> {
    #[account(mut)]
    mint: AccountInfo<'info>,
}
