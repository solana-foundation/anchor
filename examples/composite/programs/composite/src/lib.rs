//! This example demonstrates the ability to compose together multiple
//! structs deriving `Accounts`. See `CompositeUpdate`, below.

use anchor_lang::prelude::*;

#[program]
mod composite {
    use super::*;
    pub fn initialize(_ctx: Context<Initialize>) -> ProgramResult {
        Ok(())
    }

    pub fn composite_update(
        ctx: Context<CompositeUpdate>,
        dummy_a: u64,
        dummy_b: u64,
    ) -> ProgramResult {
        let a = &mut ctx.accounts.foo.dummy_a;
        let b = &mut ctx.accounts.bar.dummy_b;

        a.data = dummy_a;
        b.data = dummy_b;

        Ok(())
    }
}

#[derive(Accounts)]
pub struct Initialize<'info> {
    #[account(init)]
    pub dummy_a: ProgramAccount<'info, DummyA>,
    #[account(init)]
    pub dummy_b: ProgramAccount<'info, DummyB>,
}

#[derive(Accounts)]
pub struct CompositeUpdate<'info> {
    foo: Foo<'info>,
    bar: Bar<'info>,
}

#[derive(Accounts)]
pub struct Foo<'info> {
    #[account(mut)]
    pub dummy_a: ProgramAccount<'info, DummyA>,
}

#[derive(Accounts)]
pub struct Bar<'info> {
    #[account(mut)]
    pub dummy_b: ProgramAccount<'info, DummyB>,
}

#[account]
pub struct DummyA {
    pub data: u64,
}

#[account]
pub struct DummyB {
    pub data: u64,
}
