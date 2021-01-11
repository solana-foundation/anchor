#![feature(proc_macro_hygiene)]

use anchor::prelude::*;

// Define the program's RPC handlers.

#[program]
mod basic_2 {
    use super::*;

    #[access_control(not_zero(authority))]
    pub fn create_root(ctx: Context<CreateRoot>, authority: Pubkey, data: u64) -> ProgramResult {
        let root = &mut ctx.accounts.root;
        root.authority = authority;
        root.data = data;
        root.initialized = true;
        Ok(())
    }

    pub fn update_root(ctx: Context<UpdateRoot>, data: u64) -> ProgramResult {
        let root = &mut ctx.accounts.root;
        root.data = data;
        Ok(())
    }

    pub fn create_leaf(ctx: Context<CreateLeaf>, data: u64, custom: MyCustomType) -> ProgramResult {
        let leaf = &mut ctx.accounts.leaf;
        leaf.initialized = true;
        leaf.root = *ctx.accounts.root.info.key;
        leaf.data = data;
        leaf.custom = custom;
        Ok(())
    }

    pub fn update_leaf(
        ctx: Context<UpdateLeaf>,
        data: u64,
        custom: Option<MyCustomType>,
    ) -> ProgramResult {
        let leaf = &mut ctx.accounts.leaf;
        leaf.data = data;
        if let Some(custom) = custom {
            leaf.custom = custom;
        }
        Ok(())
    }
}

// Define the validated accounts for each handler.

#[derive(Accounts)]
pub struct CreateRoot<'info> {
    #[account(init)]
    pub root: ProgramAccount<'info, Root>,
    pub rent: Rent,
}

#[derive(Accounts)]
pub struct UpdateRoot<'info> {
    #[account(signer)]
    pub authority: AccountInfo<'info>,
    #[account(mut, "&root.authority == authority.key")]
    pub root: ProgramAccount<'info, Root>,
}

#[derive(Accounts)]
pub struct CreateLeaf<'info> {
    pub root: ProgramAccount<'info, Root>,
    #[account(init)]
    pub leaf: ProgramAccount<'info, Leaf>,
    pub rent: Rent,
}

#[derive(Accounts)]
pub struct UpdateLeaf<'info> {
    #[account(signer)]
    pub authority: AccountInfo<'info>,
    #[account("&root.authority == authority.key")]
    pub root: ProgramAccount<'info, Root>,
    #[account(mut, belongs_to = root)]
    pub leaf: ProgramAccount<'info, Leaf>,
}

// Define the program owned accounts.

#[account]
pub struct Root {
    pub initialized: bool,
    pub authority: Pubkey,
    pub data: u64,
}

#[account]
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
    if authority == Pubkey::new_from_array([0; 32]) {
        return Err(ProgramError::InvalidInstructionData);
    }
    Ok(())
}
