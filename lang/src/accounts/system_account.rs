//! System account alias backed by generic `Account`.

pub type SystemAccount<'info> =
    crate::accounts::account::Account<'info, crate::accounts::account::System>;
