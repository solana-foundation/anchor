#![feature(proc_macro_hygiene)]

use anchor::prelude::*;

// Define the program's RPC handlers.

#[program]
mod example {
    use super::*;

    #[access_control(not_zero(authority))]
    pub fn create_root(ctx: Context<CreateRoot>, authority: Pubkey, data: u64) -> ProgramResult {
        let root = &mut ctx.accounts.root;
        root.account.authority = authority;
        root.account.data = data;
        root.account.initialized = true;
        Ok(())
    }

    pub fn update_root(ctx: Context<UpdateRoot>, data: u64) -> ProgramResult {
        let root = &mut ctx.accounts.root;
        root.account.data = data;
        Ok(())
    }

    pub fn create_leaf(ctx: Context<CreateLeaf>, data: u64, custom: MyCustomType) -> ProgramResult {
        let leaf = &mut ctx.accounts.leaf;
        leaf.account.data = data;
        leaf.account.custom = custom;
        Ok(())
    }

    pub fn update_leaf(ctx: Context<UpdateLeaf>, data: u64) -> ProgramResult {
        let leaf = &mut ctx.accounts.leaf;
        leaf.account.data = data;
        Ok(())
    }
}

// Define the validated accounts for each handler.

#[derive(Accounts)]
pub struct CreateRoot<'info> {
    #[account(signer)]
    pub authority: AccountInfo<'info>,
    #[account(mut, "!root.initialized")]
    pub root: ProgramAccount<'info, Root>,
}

#[derive(Accounts)]
pub struct UpdateRoot<'info> {
    #[account(signer)]
    pub authority: AccountInfo<'info>,
    #[account(mut, "root.initialized", "&root.authority == authority.key")]
    pub root: ProgramAccount<'info, Root>,
}

#[derive(Accounts)]
pub struct CreateLeaf<'info> {
    #[account("root.initialized")]
    pub root: ProgramAccount<'info, Root>,
    #[account(mut, "!leaf.initialized")]
    pub leaf: ProgramAccount<'info, Leaf>,
}

#[derive(Accounts)]
pub struct UpdateLeaf<'info> {
    #[account(signer)]
    pub authority: AccountInfo<'info>,
    #[account("root.initialized", "&root.authority == authority.key")]
    pub root: ProgramAccount<'info, Root>,
    #[account(mut, belongs_to = root, "!leaf.initialized")]
    pub leaf: ProgramAccount<'info, Leaf>,
}

// Define the program owned accounts.

#[derive(AnchorSerialize, AnchorDeserialize)]
pub struct Root {
    pub initialized: bool,
    pub authority: Pubkey,
    pub data: u64,
}

#[derive(AnchorSerialize, AnchorDeserialize)]
pub struct Leaf {
    pub initialized: bool,
    pub root: Pubkey,
    pub data: u64,
    pub custom: MyCustomType,
}

// Define custom types.

#[derive(AnchorSerialize, AnchorDeserialize)]
pub struct MyCustomType {
    pub my_data: u64,
    pub key: Pubkey,
}

// Define any auxiliary access control checks.

fn not_zero(authority: Pubkey) -> ProgramResult {
    if authority != Pubkey::new_from_array([0; 32]) {
        return Err(ProgramError::InvalidInstructionData);
    }
    Ok(())
}
