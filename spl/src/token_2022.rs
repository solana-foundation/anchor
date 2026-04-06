// Avoiding AccountInfo deprecated msg in anchor context
#![allow(deprecated)]
use anchor_lang::{
    context::CpiContext,
    pinocchio_runtime::{account_view::AccountView, pubkey::Pubkey},
    Accounts, Result,
};
pub use {spl_token_2022::ID, spl_token_2022_interface as spl_token_2022};

#[deprecated(
    since = "0.28.0",
    note = "please use `transfer_checked` or `transfer_checked_with_fee` instead"
)]
pub fn transfer(ctx: CpiContext<'_, '_, Transfer>, amount: u64) -> Result<()> {
    #[allow(deprecated)]
    let ix = spl_token_2022::instruction::transfer(
        &ctx.program_id,
        ctx.accounts.from.address(),
        ctx.accounts.to.address(),
        ctx.accounts.authority.address(),
        &[],
        amount,
    )?;
    anchor_lang::pinocchio_runtime::program::invoke_signed(
        &ix,
        &[ctx.accounts.from, ctx.accounts.to, ctx.accounts.authority],
        ctx.signer_seeds,
    )
    .map_err(Into::into)
}

pub fn transfer_checked(
    ctx: CpiContext<'_, '_, TransferChecked>,
    amount: u64,
    decimals: u8,
) -> Result<()> {
    let ix = spl_token_2022::instruction::transfer_checked(
        &ctx.program_id,
        ctx.accounts.from.address(),
        ctx.accounts.mint.address(),
        ctx.accounts.to.address(),
        ctx.accounts.authority.address(),
        &[],
        amount,
        decimals,
    )?;
    anchor_lang::pinocchio_runtime::program::invoke_signed(
        &ix,
        &[
            ctx.accounts.from,
            ctx.accounts.mint,
            ctx.accounts.to,
            ctx.accounts.authority,
        ],
        ctx.signer_seeds,
    )
    .map_err(Into::into)
}

pub fn mint_to(ctx: CpiContext<'_, '_, MintTo>, amount: u64) -> Result<()> {
    let ix = spl_token_2022::instruction::mint_to(
        &ctx.program_id,
        ctx.accounts.mint.address(),
        ctx.accounts.to.address(),
        ctx.accounts.authority.address(),
        &[],
        amount,
    )?;
    anchor_lang::pinocchio_runtime::program::invoke_signed(
        &ix,
        &[ctx.accounts.to, ctx.accounts.mint, ctx.accounts.authority],
        ctx.signer_seeds,
    )
    .map_err(Into::into)
}

pub fn mint_to_checked(
    ctx: CpiContext<'_, '_, MintToChecked>,
    amount: u64,
    decimals: u8,
) -> Result<()> {
    let ix = spl_token_2022::instruction::mint_to_checked(
        &ctx.program_id,
        ctx.accounts.mint.address(),
        ctx.accounts.to.address(),
        ctx.accounts.authority.address(),
        &[],
        amount,
        decimals,
    )?;
    anchor_lang::pinocchio_runtime::program::invoke_signed(
        &ix,
        &[ctx.accounts.to, ctx.accounts.mint, ctx.accounts.authority],
        ctx.signer_seeds,
    )
    .map_err(Into::into)
}

pub fn burn(ctx: CpiContext<'_, '_, Burn>, amount: u64) -> Result<()> {
    let ix = spl_token_2022::instruction::burn(
        &ctx.program_id,
        ctx.accounts.from.address(),
        ctx.accounts.mint.address(),
        ctx.accounts.authority.address(),
        &[],
        amount,
    )?;
    anchor_lang::pinocchio_runtime::program::invoke_signed(
        &ix,
        &[ctx.accounts.from, ctx.accounts.mint, ctx.accounts.authority],
        ctx.signer_seeds,
    )
    .map_err(Into::into)
}

pub fn burn_checked(ctx: CpiContext<'_, '_, BurnChecked>, amount: u64, decimals: u8) -> Result<()> {
    let ix = spl_token_2022::instruction::burn_checked(
        &ctx.program_id,
        ctx.accounts.from.address(),
        ctx.accounts.mint.address(),
        ctx.accounts.authority.address(),
        &[],
        amount,
        decimals,
    )?;
    anchor_lang::pinocchio_runtime::program::invoke_signed(
        &ix,
        &[ctx.accounts.from, ctx.accounts.mint, ctx.accounts.authority],
        ctx.signer_seeds,
    )
    .map_err(Into::into)
}

pub fn approve(ctx: CpiContext<'_, '_, Approve>, amount: u64) -> Result<()> {
    let ix = spl_token_2022::instruction::approve(
        &ctx.program_id,
        ctx.accounts.to.address(),
        ctx.accounts.delegate.address(),
        ctx.accounts.authority.address(),
        &[],
        amount,
    )?;
    anchor_lang::pinocchio_runtime::program::invoke_signed(
        &ix,
        &[
            ctx.accounts.to,
            ctx.accounts.delegate,
            ctx.accounts.authority,
        ],
        ctx.signer_seeds,
    )
    .map_err(Into::into)
}

pub fn approve_checked(
    ctx: CpiContext<'_, '_, ApproveChecked>,
    amount: u64,
    decimals: u8,
) -> Result<()> {
    let ix = spl_token_2022::instruction::approve_checked(
        &ctx.program_id,
        ctx.accounts.to.address(),
        ctx.accounts.mint.address(),
        ctx.accounts.delegate.address(),
        ctx.accounts.authority.address(),
        &[],
        amount,
        decimals,
    )?;
    anchor_lang::pinocchio_runtime::program::invoke_signed(
        &ix,
        &[
            ctx.accounts.to,
            ctx.accounts.mint,
            ctx.accounts.delegate,
            ctx.accounts.authority,
        ],
        ctx.signer_seeds,
    )
    .map_err(Into::into)
}

pub fn revoke(ctx: CpiContext<'_, '_, Revoke>) -> Result<()> {
    let ix = spl_token_2022::instruction::revoke(
        &ctx.program_id,
        ctx.accounts.source.address(),
        ctx.accounts.authority.address(),
        &[],
    )?;
    anchor_lang::pinocchio_runtime::program::invoke_signed(
        &ix,
        &[ctx.accounts.source, ctx.accounts.authority],
        ctx.signer_seeds,
    )
    .map_err(Into::into)
}

pub fn initialize_account(ctx: CpiContext<'_, '_, InitializeAccount>) -> Result<()> {
    let ix = spl_token_2022::instruction::initialize_account(
        &ctx.program_id,
        ctx.accounts.account.address(),
        ctx.accounts.mint.address(),
        ctx.accounts.authority.address(),
    )?;
    anchor_lang::pinocchio_runtime::program::invoke(
        &ix,
        &[
            ctx.accounts.account,
            ctx.accounts.mint,
            ctx.accounts.authority,
            ctx.accounts.rent,
        ],
    )
    .map_err(Into::into)
}

pub fn initialize_account3(ctx: CpiContext<'_, '_, InitializeAccount3>) -> Result<()> {
    let ix = spl_token_2022::instruction::initialize_account3(
        &ctx.program_id,
        ctx.accounts.account.address(),
        ctx.accounts.mint.address(),
        ctx.accounts.authority.address(),
    )?;
    anchor_lang::pinocchio_runtime::program::invoke(&ix, &[ctx.accounts.account, ctx.accounts.mint])
        .map_err(Into::into)
}

pub fn close_account(ctx: CpiContext<'_, '_, CloseAccount>) -> Result<()> {
    let ix = spl_token_2022::instruction::close_account(
        &ctx.program_id,
        ctx.accounts.account.address(),
        ctx.accounts.destination.address(),
        ctx.accounts.authority.address(),
        &[], // TODO: support multisig
    )?;
    anchor_lang::pinocchio_runtime::program::invoke_signed(
        &ix,
        &[
            ctx.accounts.account,
            ctx.accounts.destination,
            ctx.accounts.authority,
        ],
        ctx.signer_seeds,
    )
    .map_err(Into::into)
}

pub fn freeze_account(ctx: CpiContext<'_, '_, FreezeAccount>) -> Result<()> {
    let ix = spl_token_2022::instruction::freeze_account(
        &ctx.program_id,
        ctx.accounts.account.address(),
        ctx.accounts.mint.address(),
        ctx.accounts.authority.address(),
        &[], // TODO: Support multisig signers.
    )?;
    anchor_lang::pinocchio_runtime::program::invoke_signed(
        &ix,
        &[
            ctx.accounts.account,
            ctx.accounts.mint,
            ctx.accounts.authority,
        ],
        ctx.signer_seeds,
    )
    .map_err(Into::into)
}

pub fn thaw_account(ctx: CpiContext<'_, '_, ThawAccount>) -> Result<()> {
    let ix = spl_token_2022::instruction::thaw_account(
        &ctx.program_id,
        ctx.accounts.account.address(),
        ctx.accounts.mint.address(),
        ctx.accounts.authority.address(),
        &[], // TODO: Support multisig signers.
    )?;
    anchor_lang::pinocchio_runtime::program::invoke_signed(
        &ix,
        &[
            ctx.accounts.account,
            ctx.accounts.mint,
            ctx.accounts.authority,
        ],
        ctx.signer_seeds,
    )
    .map_err(Into::into)
}

pub fn initialize_mint(
    ctx: CpiContext<'_, '_, InitializeMint>,
    decimals: u8,
    authority: &Pubkey,
    freeze_authority: Option<&Pubkey>,
) -> Result<()> {
    let ix = spl_token_2022::instruction::initialize_mint(
        &ctx.program_id,
        ctx.accounts.mint.address(),
        authority,
        freeze_authority,
        decimals,
    )?;
    anchor_lang::pinocchio_runtime::program::invoke(&ix, &[ctx.accounts.mint, ctx.accounts.rent])
        .map_err(Into::into)
}

pub fn initialize_mint2(
    ctx: CpiContext<'_, '_, InitializeMint2>,
    decimals: u8,
    authority: &Pubkey,
    freeze_authority: Option<&Pubkey>,
) -> Result<()> {
    let ix = spl_token_2022::instruction::initialize_mint2(
        &ctx.program_id,
        ctx.accounts.mint.address(),
        authority,
        freeze_authority,
        decimals,
    )?;
    anchor_lang::pinocchio_runtime::program::invoke(&ix, &[ctx.accounts.mint]).map_err(Into::into)
}

pub fn set_authority(
    ctx: CpiContext<'_, '_, SetAuthority>,
    authority_type: spl_token_2022::instruction::AuthorityType,
    new_authority: Option<Pubkey>,
) -> Result<()> {
    let ix = spl_token_2022::instruction::set_authority(
        &ctx.program_id,
        ctx.accounts.account_or_mint.address(),
        new_authority.as_ref(),
        authority_type,
        ctx.accounts.current_authority.address(),
        &[], // TODO: Support multisig signers.
    )?;
    anchor_lang::pinocchio_runtime::program::invoke_signed(
        &ix,
        &[ctx.accounts.account_or_mint, ctx.accounts.current_authority],
        ctx.signer_seeds,
    )
    .map_err(Into::into)
}

pub fn sync_native(ctx: CpiContext<'_, '_, SyncNative>) -> Result<()> {
    let ix =
        spl_token_2022::instruction::sync_native(&ctx.program_id, ctx.accounts.account.address())?;
    anchor_lang::pinocchio_runtime::program::invoke(&ix, &[ctx.accounts.account])
        .map_err(Into::into)
}

pub fn get_account_data_size(
    ctx: CpiContext<'_, '_, GetAccountDataSize>,
    extension_types: &[spl_token_2022::extension::ExtensionType],
) -> Result<u64> {
    let ix = spl_token_2022::instruction::get_account_data_size(
        &ctx.program_id,
        ctx.accounts.mint.address(),
        extension_types,
    )?;
    anchor_lang::pinocchio_runtime::program::invoke(&ix, &[ctx.accounts.mint])?;
    anchor_lang::pinocchio_runtime::program::get_return_data()
        .ok_or(anchor_lang::pinocchio_runtime::program_error::ProgramError::InvalidInstructionData)
        .and_then(|(key, data)| {
            if key != ctx.program_id {
                Err(anchor_lang::pinocchio_runtime::program_error::ProgramError::IncorrectProgramId)
            } else {
                data.try_into().map(u64::from_le_bytes).map_err(|_| {
                    anchor_lang::pinocchio_runtime::program_error::ProgramError::InvalidInstructionData
                })
            }
        })
        .map_err(Into::into)
}

pub fn initialize_mint_close_authority(
    ctx: CpiContext<'_, '_, InitializeMintCloseAuthority>,
    close_authority: Option<&Pubkey>,
) -> Result<()> {
    let ix = spl_token_2022::instruction::initialize_mint_close_authority(
        &ctx.program_id,
        ctx.accounts.mint.address(),
        close_authority,
    )?;
    anchor_lang::pinocchio_runtime::program::invoke(&ix, &[ctx.accounts.mint]).map_err(Into::into)
}

pub fn initialize_immutable_owner(ctx: CpiContext<'_, '_, InitializeImmutableOwner>) -> Result<()> {
    let ix = spl_token_2022::instruction::initialize_immutable_owner(
        &ctx.program_id,
        ctx.accounts.account.address(),
    )?;
    anchor_lang::pinocchio_runtime::program::invoke(&ix, &[ctx.accounts.account])
        .map_err(Into::into)
}

pub fn amount_to_ui_amount(
    ctx: CpiContext<'_, '_, AmountToUiAmount>,
    amount: u64,
) -> Result<String> {
    let ix = spl_token_2022::instruction::amount_to_ui_amount(
        &ctx.program_id,
        ctx.accounts.account.address(),
        amount,
    )?;
    anchor_lang::pinocchio_runtime::program::invoke(&ix, &[ctx.accounts.account])?;
    anchor_lang::pinocchio_runtime::program::get_return_data()
        .ok_or(anchor_lang::pinocchio_runtime::program_error::ProgramError::InvalidInstructionData)
        .and_then(|(key, data)| {
            if key != ctx.program_id {
                Err(anchor_lang::pinocchio_runtime::program_error::ProgramError::IncorrectProgramId)
            } else {
                String::from_utf8(data).map_err(|_| {
                    anchor_lang::pinocchio_runtime::program_error::ProgramError::InvalidInstructionData
                })
            }
        })
        .map_err(Into::into)
}

pub fn ui_amount_to_amount(
    ctx: CpiContext<'_, '_, UiAmountToAmount>,
    ui_amount: &str,
) -> Result<u64> {
    let ix = spl_token_2022::instruction::ui_amount_to_amount(
        &ctx.program_id,
        ctx.accounts.account.address(),
        ui_amount,
    )?;
    anchor_lang::pinocchio_runtime::program::invoke(&ix, &[ctx.accounts.account])?;
    anchor_lang::pinocchio_runtime::program::get_return_data()
        .ok_or(anchor_lang::pinocchio_runtime::program_error::ProgramError::InvalidInstructionData)
        .and_then(|(key, data)| {
            if key != ctx.program_id {
                Err(anchor_lang::pinocchio_runtime::program_error::ProgramError::IncorrectProgramId)
            } else {
                data.try_into().map(u64::from_le_bytes).map_err(|_| {
                    anchor_lang::pinocchio_runtime::program_error::ProgramError::InvalidInstructionData
                })
            }
        })
        .map_err(Into::into)
}

#[derive(Accounts)]
pub struct Transfer {
    pub from: AccountView,
    pub to: AccountView,
    pub authority: AccountView,
}

#[derive(Accounts)]
pub struct TransferChecked {
    pub from: AccountView,
    pub mint: AccountView,
    pub to: AccountView,
    pub authority: AccountView,
}

#[derive(Accounts)]
pub struct MintTo {
    pub mint: AccountView,
    pub to: AccountView,
    pub authority: AccountView,
}

#[derive(Accounts)]
pub struct MintToChecked {
    pub mint: AccountView,
    pub to: AccountView,
    pub authority: AccountView,
}

#[derive(Accounts)]
pub struct Burn {
    pub mint: AccountView,
    pub from: AccountView,
    pub authority: AccountView,
}

#[derive(Accounts)]
pub struct BurnChecked {
    pub mint: AccountView,
    pub from: AccountView,
    pub authority: AccountView,
}

#[derive(Accounts)]
pub struct Approve {
    pub to: AccountView,
    pub delegate: AccountView,
    pub authority: AccountView,
}

#[derive(Accounts)]
pub struct ApproveChecked {
    pub to: AccountView,
    pub mint: AccountView,
    pub delegate: AccountView,
    pub authority: AccountView,
}

#[derive(Accounts)]
pub struct Revoke {
    pub source: AccountView,
    pub authority: AccountView,
}

#[derive(Accounts)]
pub struct InitializeAccount {
    pub account: AccountView,
    pub mint: AccountView,
    pub authority: AccountView,
    pub rent: AccountView,
}

#[derive(Accounts)]
pub struct InitializeAccount3 {
    pub account: AccountView,
    pub mint: AccountView,
    pub authority: AccountView,
}

#[derive(Accounts)]
pub struct CloseAccount {
    pub account: AccountView,
    pub destination: AccountView,
    pub authority: AccountView,
}

#[derive(Accounts)]
pub struct FreezeAccount {
    pub account: AccountView,
    pub mint: AccountView,
    pub authority: AccountView,
}

#[derive(Accounts)]
pub struct ThawAccount {
    pub account: AccountView,
    pub mint: AccountView,
    pub authority: AccountView,
}

#[derive(Accounts)]
pub struct InitializeMint {
    pub mint: AccountView,
    pub rent: AccountView,
}

#[derive(Accounts)]
pub struct InitializeMint2 {
    pub mint: AccountView,
}

#[derive(Accounts)]
pub struct SetAuthority {
    pub current_authority: AccountView,
    pub account_or_mint: AccountView,
}

#[derive(Accounts)]
pub struct SyncNative {
    pub account: AccountView,
}

#[derive(Accounts)]
pub struct GetAccountDataSize {
    pub mint: AccountView,
}

#[derive(Accounts)]
pub struct InitializeMintCloseAuthority {
    pub mint: AccountView,
}

#[derive(Accounts)]
pub struct InitializeImmutableOwner {
    pub account: AccountView,
}

#[derive(Accounts)]
pub struct AmountToUiAmount {
    pub account: AccountView,
}

#[derive(Accounts)]
pub struct UiAmountToAmount {
    pub account: AccountView,
}

#[derive(Clone)]
pub struct Token2022;

impl anchor_lang::Id for Token2022 {
    fn id() -> Pubkey {
        ID
    }
}
