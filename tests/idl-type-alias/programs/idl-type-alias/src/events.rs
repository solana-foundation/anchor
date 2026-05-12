use anchor_lang::prelude::*;

// A newtype wrapper — works fine.
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy, Debug, InitSpace)]
pub struct FooId(pub [u8; 32]);

// Type alias for a plain array — triggers issue #4052 when used in an event.
pub type BarId = [u8; 32];

#[event]
#[derive(Clone, Debug)]
pub struct MyEvent {
    pub foo_id: FooId,
    pub bar_id: BarId,
}
