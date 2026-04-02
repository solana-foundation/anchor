#[path = "support/mod.rs"]
mod support;
use {
    anchor_lang::{prelude::*, RefMut},
    support::OwnedPinocchioAccount,
};

declare_id!("Fg6PaFpoGXkYsidMpWTK6W2BeZ7FEfcYkg476zPFsLnS");

#[account]
#[derive(Default, Debug)]
struct Dummy {
    val: u64,
}

fn serialize_dummy(val: u64) -> Vec<u8> {
    let mut v: Vec<u8> = Vec::new();
    Dummy { val }.try_serialize(&mut v).unwrap();
    v
}

#[test]
fn reload_owner_unchanged_updates_data() {
    let init = serialize_dummy(10);
    let lamports: u64 = 1;
    let owner: Pubkey = crate::ID;

    let key: Pubkey = Pubkey::new_unique();
    let mut owned = OwnedPinocchioAccount::new(key, owner, lamports, &init);

    let mut acc: Account<Dummy> = Account::<Dummy>::try_from(owned.info).unwrap();
    assert_eq!(acc.val, 10);

    let new_bytes = serialize_dummy(42);
    assert_eq!(new_bytes.len(), acc.to_account_view().data_len());

    {
        let mut d: RefMut<'_, [u8]> = owned.info.try_borrow_mut().unwrap();
        d.copy_from_slice(&new_bytes);
    }

    acc.reload().unwrap();
    assert_eq!(acc.val, 42);
}

#[test]
fn reload_owner_changed_fails() {
    let init = serialize_dummy(1);
    let lamports: u64 = 1;
    let owner: Pubkey = crate::ID;

    let key: Pubkey = Pubkey::new_unique();
    let mut owned = OwnedPinocchioAccount::new(key, owner, lamports, &init);

    let mut acc: Account<Dummy> = Account::<Dummy>::try_from(owned.info).unwrap();

    owned.set_owner(Pubkey::new_unique());

    let err: anchor_lang::error::Error = acc.reload().unwrap_err();
    assert_eq!(
        err,
        anchor_lang::error::ErrorCode::AccountOwnedByWrongProgram.into()
    );
}

#[test]
fn interface_reload_owner_unchanged_updates_data() {
    let data = serialize_dummy(5);
    let lamports: u64 = 1;
    let owner: Pubkey = crate::ID;
    let key: Pubkey = Pubkey::new_unique();
    let mut owned = OwnedPinocchioAccount::new(key, owner, lamports, &data);

    let mut i_face: Account<Dummy> = Account::<Dummy>::try_from(owned.info).unwrap();
    assert_eq!(i_face.val, 5);

    let new_bytes = serialize_dummy(6);
    {
        let mut d: RefMut<'_, [u8]> = owned.info.try_borrow_mut().unwrap();
        d.copy_from_slice(&new_bytes);
    }

    i_face.reload().unwrap();
    assert_eq!(i_face.val, 6);
}

#[test]
fn reload_error_does_not_mutate_cached_state() {
    let data = serialize_dummy(7);
    let lamports: u64 = 1;

    let owner: Pubkey = crate::ID;
    let key: Pubkey = Pubkey::new_unique();
    let mut owned = OwnedPinocchioAccount::new(key, owner, lamports, &data);

    let mut acc: Account<Dummy> = Account::<Dummy>::try_from(owned.info).unwrap();
    assert_eq!(acc.val, 7);

    owned.set_owner(Pubkey::new_unique());

    let err: anchor_lang::error::Error = acc.reload().unwrap_err();
    assert_eq!(
        err,
        anchor_lang::error::ErrorCode::AccountOwnedByWrongProgram.into()
    );
    assert_eq!(acc.val, 7);
}
