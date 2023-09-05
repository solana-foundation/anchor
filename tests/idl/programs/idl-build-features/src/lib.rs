use anchor_lang::prelude::*;

declare_id!("id1Bui1dFeatures111111111111111111111111111");

#[program]
pub mod idl_build_features {
    use super::*;

    pub fn full_path(
        ctx: Context<FullPath>,
        my_struct: MyStruct,
        some_module_my_struct: some_module::MyStruct,
    ) -> Result<()> {
        ctx.accounts.account.my_struct = my_struct;
        ctx.accounts.account.some_module_my_struct = some_module_my_struct;
        Ok(())
    }
}

#[derive(Accounts)]
pub struct FullPath<'info> {
    #[account(zero)]
    pub account: Account<'info, FullPathAccount>,
}

#[account]
pub struct FullPathAccount {
    pub my_struct: MyStruct,
    pub some_module_my_struct: some_module::MyStruct,
}

mod some_module {
    use super::*;

    #[derive(AnchorSerialize, AnchorDeserialize, Clone)]
    pub struct MyStruct {
        pub data: u8,
    }
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone)]
pub struct MyStruct {
    pub u8: u8,
    pub u16: u16,
    pub u32: u32,
    pub u64: u64,
}
