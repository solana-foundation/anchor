//! Signer account alias backed by generic `Account`.

pub type Signer<'info> = crate::accounts::account::Account<'info, crate::accounts::account::Wallet>;
