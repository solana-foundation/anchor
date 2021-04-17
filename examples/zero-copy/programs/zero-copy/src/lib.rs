//! This example demonstrates the use of zero copy deserialization for accounts.
//! The main noticeable benefit one achieves using zero copy is the ability
//! to create accounts larger than the size of the stack or heap, as is
//! demonstrated by the event queue in this example.

use anchor_lang::prelude::*;

#[program]
pub mod zero_copy {
    use super::*;

    pub fn create_foo(ctx: Context<CreateFoo>) -> ProgramResult {
        let foo = &mut ctx.accounts.foo.load_init()?;
        foo.authority = *ctx.accounts.authority.key;
        foo.set_second_authority(ctx.accounts.authority.key);
        Ok(())
    }

    pub fn update_foo(ctx: Context<UpdateFoo>, data: u64) -> ProgramResult {
        let mut foo = ctx.accounts.foo.load_mut()?;
        foo.data = data;
        Ok(())
    }

    pub fn update_foo_second(ctx: Context<UpdateFooSecond>, second_data: u64) -> ProgramResult {
        let mut foo = ctx.accounts.foo.load_mut()?;
        foo.second_data = second_data;
        Ok(())
    }

    pub fn create_bar(ctx: Context<CreateBar>) -> ProgramResult {
        let bar = &mut ctx.accounts.bar.load_init()?;
        bar.authority = *ctx.accounts.authority.key;
        Ok(())
    }

    pub fn update_bar(ctx: Context<UpdateBar>, data: u64) -> ProgramResult {
        let bar = &mut ctx.accounts.bar.load_mut()?;
        bar.data = data;
        Ok(())
    }

    pub fn create_large_account(_ctx: Context<CreateLargeAccount>) -> ProgramResult {
        Ok(())
    }

    pub fn update_large_account(
        ctx: Context<UpdateLargeAccount>,
        idx: u32,
        data: u64,
    ) -> ProgramResult {
        let event_q = &mut ctx.accounts.event_q.load_mut()?;
        event_q.events[idx as usize] = Event {
            data,
            from: *ctx.accounts.from.key,
        };
        Ok(())
    }
}

#[derive(Accounts)]
pub struct CreateFoo<'info> {
    #[account(init)]
    foo: Loader<'info, Foo>,
    #[account(signer)]
    authority: AccountInfo<'info>,
    rent: Sysvar<'info, Rent>,
}

#[derive(Accounts)]
pub struct UpdateFoo<'info> {
    #[account(mut, has_one = authority)]
    foo: Loader<'info, Foo>,
    #[account(signer)]
    authority: AccountInfo<'info>,
}

#[derive(Accounts)]
pub struct UpdateFooSecond<'info> {
    #[account(mut, "&foo.load()?.get_second_authority() == second_authority.key")]
    foo: Loader<'info, Foo>,
    #[account(signer)]
    second_authority: AccountInfo<'info>,
}

#[derive(Accounts)]
pub struct CreateBar<'info> {
    #[account(associated = authority, with = foo)]
    bar: Loader<'info, Bar>,
    #[account(signer)]
    authority: AccountInfo<'info>,
    foo: Loader<'info, Foo>,
    rent: Sysvar<'info, Rent>,
    system_program: AccountInfo<'info>,
}

#[derive(Accounts)]
pub struct UpdateBar<'info> {
    #[account(mut, has_one = authority)]
    bar: Loader<'info, Bar>,
    #[account(signer)]
    authority: AccountInfo<'info>,
}

#[derive(Accounts)]
pub struct CreateLargeAccount<'info> {
    #[account(init)]
    event_q: Loader<'info, EventQ>,
    rent: Sysvar<'info, Rent>,
}

#[derive(Accounts)]
pub struct UpdateLargeAccount<'info> {
    #[account(mut)]
    event_q: Loader<'info, EventQ>,
    #[account(signer)]
    from: AccountInfo<'info>,
}

#[account(zero_copy)]
pub struct Foo {
    pub authority: Pubkey,
    pub data: u64,
    pub second_data: u64,
    #[accessor(Pubkey)] // The `accessor` api will likely be removed.
    pub second_authority: [u8; 32],
}

#[associated(zero_copy)]
pub struct Bar {
    pub authority: Pubkey,
    pub data: u64,
}

#[account(zero_copy)]
pub struct EventQ {
    pub events: [Event; 25000],
}

#[zero_copy]
pub struct Event {
    pub from: Pubkey,
    pub data: u64,
}
