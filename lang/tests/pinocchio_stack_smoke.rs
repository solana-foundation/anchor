//! Regression smoke tests for the Pinocchio stack: `CpiContext` lifetimes (task2) and
//! lifetime-free `Account` / `InterfaceAccount` usage (task3).
#[path = "support/mod.rs"]
mod support;

use anchor_lang::prelude::*;
use anchor_lang::system_program::{self, CreateAccount, Transfer};

use support::OwnedPinocchioAccount;

declare_id!("Fg6PaFpoGXkYsidMpWTK6W2BeZ7FEfcYkg476zPFsLnS");

#[account]
#[derive(Default)]
struct DataAcc {
    v: u64,
}

#[test]
fn cpi_context_transfer_construct_and_meta_roundtrip() {
    let owner = crate::ID;
    let lamports = 1u64;
    let from_k = Pubkey::new_unique();
    let to_k = Pubkey::new_unique();

    let from_o = OwnedPinocchioAccount::new(from_k, owner, lamports, &[]);
    let to_o = OwnedPinocchioAccount::new(to_k, owner, lamports, &[]);

    let accs = Transfer {
        from: from_o.info,
        to: to_o.info,
    };
    let ctx = CpiContext::new(system_program::ID, accs);
    let metas = ctx.accounts.to_account_metas(None);
    assert_eq!(metas.len(), 2);

    let infos = ctx.accounts.to_account_views();
    assert_eq!(infos.len(), 2);
}

#[test]
fn cpi_context_create_account_triple_lifetime() {
    let owner = crate::ID;
    let lamports = 10u64;
    let from_k = Pubkey::new_unique();
    let to_k = Pubkey::new_unique();

    let from_o = OwnedPinocchioAccount::new(from_k, owner, lamports, &[]);
    let to_o = OwnedPinocchioAccount::new(to_k, owner, 0, &[]);

    let accs = CreateAccount {
        from: from_o.info,
        to: to_o.info,
    };
    let _ctx: CpiContext<'_, '_, CreateAccount> = CpiContext::new(system_program::ID, accs);
}

#[test]
fn account_try_from_set_inner_key_matches_view() {
    let owner = crate::ID;
    let key = Pubkey::new_unique();
    let mut buf = vec![];
    DataAcc { v: 9 }.try_serialize(&mut buf).unwrap();

    let owned = OwnedPinocchioAccount::new(key, owner, 1, &buf);
    let mut acc: Account<DataAcc> = Account::try_from(owned.info).unwrap();
    assert_eq!(acc.v, 9);
    assert_eq!(Key::key(&acc), key);

    acc.set_inner(DataAcc { v: 3 });
    assert_eq!(acc.v, 3);
}

#[test]
fn interface_account_deref_and_key() {
    use anchor_lang::accounts::interface_account::InterfaceAccount;

    let owner = crate::ID;
    let key = Pubkey::new_unique();
    let mut buf = vec![];
    DataAcc { v: 11 }.try_serialize(&mut buf).unwrap();

    let owned = OwnedPinocchioAccount::new(key, owner, 1, &buf);
    let iface: InterfaceAccount<DataAcc> = InterfaceAccount::try_from(owned.info).unwrap();
    assert_eq!(iface.v, 11);
    assert_eq!(Key::key(&iface), key);
}
