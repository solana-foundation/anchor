use anchor_lang::prelude::*;

declare_id!("C1ient1nteractions1111111111111111111111111");

#[program]
pub mod client_interactions {
    use super::*;

    pub fn int(ctx: Context<Int>, i8: i8, i16: i16, i32: i32, i64: i64, i128: i128) -> Result<()> {
        ctx.accounts.account.i8 = i8;
        ctx.accounts.account.i16 = i16;
        ctx.accounts.account.i32 = i32;
        ctx.accounts.account.i64 = i64;
        ctx.accounts.account.i128 = i128;
        Ok(())
    }

    pub fn uint(
        ctx: Context<UnsignedInt>,
        u8: u8,
        u16: u16,
        u32: u32,
        u64: u64,
        u128: u128,
    ) -> Result<()> {
        ctx.accounts.account.u8 = u8;
        ctx.accounts.account.u16 = u16;
        ctx.accounts.account.u32 = u32;
        ctx.accounts.account.u64 = u64;
        ctx.accounts.account.u128 = u128;
        Ok(())
    }

    pub fn enm(ctx: Context<Enum>, enum_arg: MyEnum) -> Result<()> {
        ctx.accounts.account.enum_field = enum_arg;
        Ok(())
    }

    pub fn type_alias(
        ctx: Context<TypeAlias>,
        type_alias_u8: TypeAliasU8,
        type_alias_u8_array: TypeAliasU8Array,
        type_alias_struct: TypeAliasStruct,
    ) -> Result<()> {
        ctx.accounts.account.type_alias_u8 = type_alias_u8;
        ctx.accounts.account.type_alias_u8_array = type_alias_u8_array;
        ctx.accounts.account.type_alias_struct = type_alias_struct;
        Ok(())
    }
}

#[derive(Accounts)]
pub struct Int<'info> {
    #[account(zero)]
    pub account: Account<'info, IntAccount>,
}

#[account]
pub struct IntAccount {
    pub i8: i8,
    pub i16: i16,
    pub i32: i32,
    pub i64: i64,
    pub i128: i128,
}

#[derive(Accounts)]
pub struct UnsignedInt<'info> {
    #[account(zero)]
    pub account: Account<'info, UnsignedIntAccount>,
}

#[account]
pub struct UnsignedIntAccount {
    pub u8: u8,
    pub u16: u16,
    pub u32: u32,
    pub u64: u64,
    pub u128: u128,
}

#[derive(Accounts)]
pub struct Enum<'info> {
    #[account(zero)]
    pub account: Account<'info, EnumAccount>,
}

#[account]
pub struct EnumAccount {
    pub enum_field: MyEnum,
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy, Debug, Eq, PartialEq)]
pub enum MyEnum {
    Unit,
    Named { point_x: u64, point_y: u64 },
    Unnamed(u8, u8, u16, u16),
    UnnamedStruct(MyStruct),
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy, Debug, Eq, PartialEq)]
pub struct MyStruct {
    pub u8: u8,
    pub u16: u16,
    pub u32: u32,
    pub u64: u64,
}

#[derive(Accounts)]
pub struct TypeAlias<'info> {
    #[account(zero)]
    pub account: Account<'info, TypeAliasAccount>,
}

#[account]
pub struct TypeAliasAccount {
    pub type_alias_u8: TypeAliasU8,
    pub type_alias_u8_array: TypeAliasU8Array,
    pub type_alias_struct: TypeAliasStruct,
}

pub type TypeAliasU8 = u8;
pub type TypeAliasU8Array = [TypeAliasU8; 8];
pub type TypeAliasStruct = MyStruct;

/// Generic type aliases should not get included in the IDL
pub type TypeAliasNotSupported<'a, T> = NotSupported<'a, T>;
pub struct NotSupported<'a, T> {
    _t: T,
    _s: &'a str,
}
