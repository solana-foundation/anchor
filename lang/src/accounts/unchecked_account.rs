//! Explicit no-check account alias backed by generic `Account`.

pub type UncheckedAccount<'info> = crate::accounts::account::Account<'info, ()>;
