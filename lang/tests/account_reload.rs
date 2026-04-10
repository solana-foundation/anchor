use {
    anchor_lang::{
        pinocchio_runtime::account::{RuntimeAccount, NOT_BORROWED},
        prelude::*,
        Owners, RefMut,
    },
    std::{mem::size_of, ptr},
};

declare_id!("Fg6PaFpoGXkYsidMpWTK6W2BeZ7FEfcYkg476zPFsLnS");

struct OwnedPinocchioAccount {
    #[allow(dead_code)]
    storage: Box<[u8]>,
    info: AccountInfo,
}

impl OwnedPinocchioAccount {
    fn new(key: Pubkey, owner: Pubkey, lamports: u64, data: &[u8]) -> Self {
        let header_len = size_of::<RuntimeAccount>();
        let total = header_len + data.len();
        let mut storage = vec![0u8; total].into_boxed_slice();
        let header_ptr = storage.as_mut_ptr().cast::<RuntimeAccount>();
        let acc = RuntimeAccount {
            borrow_state: NOT_BORROWED,
            is_signer: 0,
            is_writable: 1,
            executable: 0,
            padding: [0; 4],
            address: key,
            owner,
            lamports,
            data_len: data.len() as u64,
        };
        unsafe {
            ptr::write(header_ptr, acc);
            ptr::copy_nonoverlapping(
                data.as_ptr(),
                storage.as_mut_ptr().add(header_len),
                data.len(),
            );
        }
        let info = unsafe { AccountInfo::new_unchecked(header_ptr) };
        Self { storage, info }
    }
}

#[account]
#[derive(Default, Debug)]
struct Dummy {
    val: u64,
}

impl Owners for Dummy {
    fn owners() -> &'static [Pubkey] {
        const IDS: [Pubkey; 1] = [crate::ID_CONST];
        &IDS
    }
}

fn serialize_dummy(val: u64) -> Vec<u8> {
    let mut v: Vec<u8> = Vec::new();
    Dummy { val }.try_serialize(&mut v).unwrap();
    v
}

#[test]
fn reload_owner_unchanged_updates_data() {
    let init = serialize_dummy(10);
    let key: Pubkey = Pubkey::new_unique();
    let owned = OwnedPinocchioAccount::new(key, crate::ID, 1, &init);

    let mut acc: Account<Dummy> = Account::<Dummy>::try_from(&owned.info).unwrap();
    assert_eq!(acc.val, 10);

    let new_bytes = serialize_dummy(42);
    assert_eq!(new_bytes.len(), acc.to_account_info().data_len());

    {
        let mut info = acc.to_account_info();
        let mut d: RefMut<'_, [u8]> = info.try_borrow_mut().unwrap();
        d.copy_from_slice(&new_bytes);
    }

    acc.reload().unwrap();
    assert_eq!(acc.val, 42);
}

#[test]
fn reload_owner_changed_fails() {
    let init = serialize_dummy(1);
    let key: Pubkey = Pubkey::new_unique();
    let owned = OwnedPinocchioAccount::new(key, crate::ID, 1, &init);

    let mut acc: Account<Dummy> = Account::<Dummy>::try_from(&owned.info).unwrap();
    unsafe {
        acc.to_account_info().assign(&Pubkey::new_unique());
    }

    let err: anchor_lang::error::Error = acc.reload().unwrap_err();
    assert_eq!(
        err,
        anchor_lang::error::ErrorCode::AccountOwnedByWrongProgram.into()
    );
}

#[test]
fn interface_reload_owner_unchanged_updates_data() {
    use anchor_lang::accounts::interface_account::InterfaceAccount;

    let data = serialize_dummy(5);
    let key: Pubkey = Pubkey::new_unique();
    let owned = OwnedPinocchioAccount::new(key, crate::ID, 1, &data);

    let mut iface: InterfaceAccount<Dummy> =
        InterfaceAccount::<Dummy>::try_from(&owned.info).unwrap();
    assert_eq!(iface.val, 5);

    let new_bytes = serialize_dummy(6);
    {
        let mut info = iface.to_account_info();
        let mut d: RefMut<'_, [u8]> = info.try_borrow_mut().unwrap();
        d.copy_from_slice(&new_bytes);
    }

    iface.reload().unwrap();
    assert_eq!(iface.val, 6);
}

#[test]
fn reload_error_does_not_mutate_cached_state() {
    let data = serialize_dummy(7);
    let key: Pubkey = Pubkey::new_unique();
    let owned = OwnedPinocchioAccount::new(key, crate::ID, 1, &data);

    let mut acc: Account<Dummy> = Account::<Dummy>::try_from(&owned.info).unwrap();
    assert_eq!(acc.val, 7);

    unsafe {
        acc.to_account_info().assign(&Pubkey::new_unique());
    }

    let err: anchor_lang::error::Error = acc.reload().unwrap_err();
    assert_eq!(
        err,
        anchor_lang::error::ErrorCode::AccountOwnedByWrongProgram.into()
    );
    assert_eq!(acc.val, 7);
}
