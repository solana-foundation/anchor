use steel::*;

declare_id!("B7ihZyoXZ1fwAY3TugkiFJ6SXkzJwMuQrxrekBaSmn32");

// --- Account discriminator ---
#[repr(u8)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum HelloWorldAccount {
    Counter = 1,
}

impl From<HelloWorldAccount> for u8 {
    fn from(v: HelloWorldAccount) -> u8 {
        v as u8
    }
}

// --- Counter state ---
#[repr(C)]
#[derive(Clone, Copy, Debug, Pod, Zeroable)]
pub struct Counter {
    pub value: u64,
    pub bump: u8,
    pub _pad: [u8; 7],
}

account!(HelloWorldAccount, Counter);

// --- Entrypoint ---
#[cfg(not(feature = "no-entrypoint"))]
entrypoint!(process_instruction);

pub fn process_instruction(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    _data: &[u8],
) -> ProgramResult {
    let [payer, counter_info, system_program] = accounts else {
        return Err(ProgramError::NotEnoughAccountKeys);
    };

    // Pre-derive the bump so `create_program_account_with_bump` doesn't re-search.
    let (_pda, bump) = Pubkey::find_program_address(&[b"counter"], program_id);

    create_program_account_with_bump::<Counter>(
        counter_info,
        system_program,
        payer,
        program_id,
        &[b"counter"],
        bump,
    )?;

    // Struct payload starts at offset 8 (after the 1-byte disc + alignment).
    let mut data = counter_info.data.borrow_mut();
    let counter = bytemuck::from_bytes_mut::<Counter>(
        &mut data[8..8 + core::mem::size_of::<Counter>()],
    );
    counter.value = 42;
    counter.bump = bump;
    Ok(())
}
