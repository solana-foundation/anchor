//! A relatively advanced example of a lockup program. If you're new to Anchor,
//! it's suggested to start with the other examples.

#![feature(proc_macro_hygiene)]

use anchor_lang::prelude::*;
use anchor_lang::solana_program;
use anchor_lang::solana_program::instruction::Instruction;
use anchor_spl::token::{self, TokenAccount, Transfer};

mod calculator;

#[program]
mod lockup {
    use super::*;
    pub fn initialize(ctx: Context<Initialize>, authority: Pubkey) -> Result<(), Error> {
        let safe = &mut ctx.accounts.safe;
        let whitelist = &mut ctx.accounts.whitelist;

        safe.authority = authority;
        safe.whitelist = *whitelist.to_account_info().key;
        whitelist.safe = *safe.to_account_info().key;

        Ok(())
    }

    pub fn set_authority(ctx: Context<SetAuthority>, new_authority: Pubkey) -> Result<(), Error> {
        let safe = &mut ctx.accounts.safe;
        safe.authority = new_authority;
        Ok(())
    }

    pub fn create_vesting(
        ctx: Context<CreateVesting>,
        beneficiary: Pubkey,
        end_ts: i64,
        period_count: u64,
        deposit_amount: u64,
        nonce: u8,
    ) -> Result<(), Error> {
        // Vesting scheudle.
        if end_ts <= ctx.accounts.clock.unix_timestamp {
            return Err(ErrorCode::InvalidTimestamp.into());
        }
        if period_count == 0 {
            return Err(ErrorCode::InvalidPeriod.into());
        }
        if deposit_amount == 0 {
            return Err(ErrorCode::InvalidDepositAmount.into());
        }
        // Vault.
        let vault_authority = Pubkey::create_program_address(
            &vault_signer_seeds(
                ctx.accounts.safe.to_account_info().key,
                &beneficiary,
                &nonce,
            ),
            ctx.program_id,
        )
        .map_err(|_| ErrorCode::InvalidProgramAddress)?;
        if ctx.accounts.vault.owner != vault_authority {
            return Err(ErrorCode::InvalidVaultOwner)?;
        }
        if ctx.accounts.vault.amount != 0 {
            return Err(ErrorCode::InvalidVaultAmount)?;
        }

        let vesting = &mut ctx.accounts.vesting;

        vesting.safe = *ctx.accounts.safe.to_account_info().key;
        vesting.beneficiary = beneficiary;
        vesting.mint = ctx.accounts.vault.mint;
        vesting.vault = *ctx.accounts.vault.to_account_info().key;
        vesting.period_count = period_count;
        vesting.start_balance = deposit_amount;
        vesting.end_ts = end_ts;
        vesting.start_ts = ctx.accounts.clock.unix_timestamp;
        vesting.outstanding = deposit_amount;
        vesting.whitelist_owned = 0;
        vesting.grantor = *ctx.accounts.depositor_authority.key;
        vesting.nonce = nonce;

        token::transfer(ctx.accounts.into(), deposit_amount)?;

        Ok(())
    }

    pub fn withdraw(ctx: Context<Withdraw>, amount: u64) -> Result<(), Error> {
        if amount == 0 {
            return Err(ErrorCode::InvalidVaultAmount.into());
        }
        if amount
            > calculator::available_for_withdrawal(
                &ctx.accounts.vesting,
                ctx.accounts.clock.unix_timestamp,
            )
        {
            return Err(ErrorCode::InsufficienWithdrawalBalance.into());
        }

        let vesting = &mut ctx.accounts.vesting;
        vesting.outstanding -= amount;

        let nonce = ctx.accounts.vesting.nonce;
        let signer = &[&vault_signer_seeds(
            ctx.accounts.safe.to_account_info().key,
            ctx.accounts.beneficiary.key,
            &nonce,
        )[..]];
        let cpi_ctx = CpiContext::from(ctx.accounts).with_signer(signer);
        token::transfer(cpi_ctx, amount)?;

        Ok(())
    }

    pub fn whitelist_add(ctx: Context<WhitelistAdd>, entry: WhitelistEntry) -> Result<(), Error> {
        if ctx.accounts.whitelist.entries.len() == 5 {
            return Err(ErrorCode::WhitelistFull.into());
        }
        let entry_derived_address = entry.derived_address()?;
        let mut items = ctx.accounts.whitelist.entries.iter().filter_map(|entry| {
            let da = entry.derived_address().expect("always valid");
            match da == entry_derived_address {
                false => None,
                true => Some(entry),
            }
        });
        if items.next().is_some() {
            return Err(ErrorCode::WhitelistEntryAlreadyExists.into());
        }
        ctx.accounts.whitelist.entries.push(entry);

        Ok(())
    }

    pub fn whitelist_delete(
        ctx: Context<WhitelistAdd>,
        entry: WhitelistEntry,
    ) -> Result<(), Error> {
        let entry_derived_address = entry.derived_address()?;

        let whitelist = &mut ctx.accounts.whitelist;
        whitelist.entries = whitelist
            .entries
            .clone()
            .into_iter()
            .filter_map(|e: WhitelistEntry| {
                if e.derived_address().expect("always valid") == entry_derived_address {
                    None
                } else {
                    Some(e)
                }
            })
            .collect::<Vec<WhitelistEntry>>();

        Ok(())
    }

    // Sends funds from a whitelisted program back to the lockup program.
    pub fn whitelist_deposit(
        ctx: Context<WhitelistDeposit>,
        instruction_data: Vec<u8>,
    ) -> Result<(), Error> {
        let accounts = ctx.accounts;

        let before_amount = accounts.vault.amount;

        // Invoke opaque relay.
        {
            let mut meta_accounts = vec![
                AccountMeta::new_readonly(*accounts.vesting.to_account_info().key, false),
                AccountMeta::new(*accounts.vault.to_account_info().key, false),
                AccountMeta::new_readonly(*accounts.vault_authority.to_account_info().key, true),
                AccountMeta::new_readonly(*accounts.token_program.to_account_info().key, false),
                AccountMeta::new(*accounts.whitelisted_program.to_account_info().key, false),
                AccountMeta::new_readonly(
                    *accounts
                        .whitelisted_program_vault_authority
                        .to_account_info()
                        .key,
                    false,
                ),
            ];
            meta_accounts.extend(ctx.remaining_accounts.iter().map(|a| {
                if a.is_writable {
                    AccountMeta::new(*a.key, a.is_signer)
                } else {
                    AccountMeta::new_readonly(*a.key, a.is_signer)
                }
            }));
            let relay_instruction = Instruction {
                program_id: *accounts.whitelisted_program.to_account_info().key,
                accounts: meta_accounts,
                data: instruction_data.to_vec(),
            };

            let signer_seeds = &[];
            solana_program::program::invoke_signed(
                &relay_instruction,
                &accounts.to_account_infos(),
                signer_seeds,
            )?;
        }

        let after_amount = accounts.vault.reload()?.amount;

        // Deposit safety checks.
        let deposit_amount = after_amount - before_amount;
        if deposit_amount <= 0 {
            return Err(ErrorCode::InsufficientWhitelistDepositAmount)?;
        }
        if deposit_amount > accounts.vesting.whitelist_owned {
            return Err(ErrorCode::WhitelistDepositOverflow)?;
        }

        // Bookkeeping.
        accounts.vesting.whitelist_owned -= deposit_amount;

        Ok(())
    }

    // Sends funds from the lockup program to a whitelisted program.
    pub fn whitelist_withdraw(
        ctx: Context<WhitelistWithdraw>,
        instruction_data: Vec<u8>,
        amount: u64,
    ) -> Result<(), Error> {
        let accounts = ctx.accounts;

        let before_amount = accounts.vault.amount;

        // Invoke opaque relay.
        {
            let mut meta_accounts = vec![
                AccountMeta::new_readonly(*accounts.vesting.to_account_info().key, false),
                AccountMeta::new(*accounts.vault.to_account_info().key, false),
                AccountMeta::new_readonly(*accounts.vault_authority.to_account_info().key, true),
                AccountMeta::new_readonly(*accounts.token_program.to_account_info().key, false),
                AccountMeta::new(
                    *accounts.whitelisted_program_vault.to_account_info().key,
                    false,
                ),
                AccountMeta::new_readonly(
                    *accounts
                        .whitelisted_program_vault_authority
                        .to_account_info()
                        .key,
                    false,
                ),
            ];
            meta_accounts.extend(ctx.remaining_accounts.iter().map(|a| {
                if a.is_writable {
                    AccountMeta::new(*a.key, a.is_signer)
                } else {
                    AccountMeta::new_readonly(*a.key, a.is_signer)
                }
            }));
            let relay_instruction = Instruction {
                program_id: *accounts.whitelisted_program.to_account_info().key,
                accounts: meta_accounts,
                data: instruction_data.to_vec(),
            };

            let signer_seeds = &[];
            solana_program::program::invoke_signed(
                &relay_instruction,
                &accounts.to_account_infos(),
                signer_seeds,
            )?;
        }

        let after_amount = accounts.vault.reload()?.amount;

        // Withdrawal safety checks.
        let amount_transferred = before_amount - after_amount;
        if amount_transferred > amount {
            return Err(ErrorCode::WhitelistWithdrawLimit)?;
        }

        // Bookeeping.
        accounts.vesting.whitelist_owned += amount_transferred;

        Ok(())
    }

    // Convenience function for UI's to calculate the withdrawalable amount.
    pub fn available_for_withdrawal(ctx: Context<AvailableForWithdrawal>) -> Result<(), Error> {
        let available = calculator::available_for_withdrawal(
            &ctx.accounts.vesting,
            ctx.accounts.clock.unix_timestamp,
        );
        // Log as string so that JS can read as a BN.
        msg!(&format!("{{ \"result\": \"{}\" }}", available));
        Ok(())
    }
}

#[derive(Accounts)]
pub struct Initialize<'info> {
    #[account(init)]
    safe: ProgramAccount<'info, Safe>,
    #[account(init)]
    whitelist: ProgramAccount<'info, Whitelist>,
    rent: Sysvar<'info, Rent>,
}

#[derive(Accounts)]
pub struct SetAuthority<'info> {
    #[account(signer)]
    authority: AccountInfo<'info>,
    #[account(mut, "&safe.authority == authority.key")]
    safe: ProgramAccount<'info, Safe>,
}

#[derive(Accounts)]
pub struct CreateVesting<'info> {
    #[account(init)]
    vesting: ProgramAccount<'info, Vesting>,
    safe: ProgramAccount<'info, Safe>,
    #[account(mut)]
    vault: CpiAccount<'info, TokenAccount>,
    #[account(mut)]
    depositor: AccountInfo<'info>,
    #[account(signer)]
    depositor_authority: AccountInfo<'info>,
    token_program: AccountInfo<'info>,
    rent: Sysvar<'info, Rent>,
    clock: Sysvar<'info, Clock>,
}

#[derive(Accounts)]
pub struct Withdraw<'info> {
    safe: ProgramAccount<'info, Safe>,
    #[account(mut, belongs_to = safe)]
    vesting: ProgramAccount<'info, Vesting>,
    #[account(signer, "beneficiary.key == &vesting.beneficiary")]
    beneficiary: AccountInfo<'info>,
    #[account(mut)]
    token: CpiAccount<'info, TokenAccount>,
    #[account(mut)]
    vault: CpiAccount<'info, TokenAccount>,
    vault_authority: AccountInfo<'info>,
    token_program: AccountInfo<'info>,
    clock: Sysvar<'info, Clock>,
}

#[derive(Accounts)]
pub struct WhitelistAdd<'info> {
    #[account(signer)]
    authority: AccountInfo<'info>,
    #[account("&safe.authority == authority.key")]
    safe: ProgramAccount<'info, Safe>,
    #[account(mut, belongs_to = safe)]
    whitelist: ProgramAccount<'info, Whitelist>,
}

#[derive(Accounts)]
pub struct WhitelistDelete<'info> {
    #[account(signer)]
    authority: AccountInfo<'info>,
    #[account("&safe.authority == authority.key")]
    safe: ProgramAccount<'info, Safe>,
    #[account(mut, belongs_to = safe)]
    whitelist: ProgramAccount<'info, Whitelist>,
}

#[derive(Accounts)]
pub struct WhitelistDeposit<'info> {
    #[account(signer)]
    beneficiary: AccountInfo<'info>,
    safe: ProgramAccount<'info, Safe>,
    #[account(belongs_to = safe)]
    whitelist: ProgramAccount<'info, Whitelist>,
    whitelisted_program: AccountInfo<'info>,

    // Whitelist interface.
    #[account(
				mut,
				belongs_to = safe,
				"&vesting.beneficiary == beneficiary.key",
		)]
    vesting: ProgramAccount<'info, Vesting>,
    #[account(mut, "&vesting.vault == vault.to_account_info().key")]
    vault: CpiAccount<'info, TokenAccount>,
    vault_authority: AccountInfo<'info>,
    token_program: AccountInfo<'info>,
    #[account(mut)]
    whitelisted_program_vault: AccountInfo<'info>,
    whitelisted_program_vault_authority: AccountInfo<'info>,
}

#[derive(Accounts)]
pub struct WhitelistWithdraw<'info> {
    #[account(signer)]
    beneficiary: AccountInfo<'info>,
    safe: ProgramAccount<'info, Safe>,
    #[account(belongs_to = safe)]
    whitelist: ProgramAccount<'info, Whitelist>,
    whitelisted_program: AccountInfo<'info>,

    // Whitelist interface.
    #[account(
				mut,
				belongs_to = safe,
				"&vesting.beneficiary == beneficiary.key",
		)]
    vesting: ProgramAccount<'info, Vesting>,
    #[account(mut, "&vesting.vault == vault.to_account_info().key")]
    vault: CpiAccount<'info, TokenAccount>,
    vault_authority: AccountInfo<'info>,
    token_program: AccountInfo<'info>,
    #[account(mut)]
    whitelisted_program_vault: AccountInfo<'info>,
    whitelisted_program_vault_authority: AccountInfo<'info>,
}

#[derive(Accounts)]
pub struct AvailableForWithdrawal<'info> {
    vesting: ProgramAccount<'info, Vesting>,
    clock: Sysvar<'info, Clock>,
}

#[account]
pub struct Safe {
    /// The key with the ability to change the whitelist.
    pub authority: Pubkey,
    /// The whitelist of valid programs the Safe can relay transactions to.
    pub whitelist: Pubkey,
}

#[account]
pub struct Whitelist {
    pub safe: Pubkey,
    pub entries: Vec<WhitelistEntry>,
}

#[derive(AnchorSerialize, AnchorDeserialize, Default, Copy, Clone)]
pub struct WhitelistEntry {
    pub program_id: Pubkey,
    pub instance: Option<Pubkey>,
    pub nonce: u8,
}

impl WhitelistEntry {
    pub fn derived_address(&self) -> Result<Pubkey, Error> {
        let pk = {
            if let Some(i) = self.instance {
                Pubkey::create_program_address(&[i.as_ref(), &[self.nonce]], &self.program_id)
            } else {
                Pubkey::create_program_address(&[&[self.nonce]], &self.program_id)
            }
        };
        pk.map_err(|_| ErrorCode::InvalidWhitelistEntry.into())
    }
}

#[account]
pub struct Vesting {
    /// The Safe instance this account is associated with.
    pub safe: Pubkey,
    /// The owner of this Vesting account.
    pub beneficiary: Pubkey,
    /// The mint of the SPL token locked up.
    pub mint: Pubkey,
    /// Address of the account's token vault.
    pub vault: Pubkey,
    /// The owner of the token account funding this account.
    pub grantor: Pubkey,
    /// The outstanding SRM deposit backing this vesting account. All
    /// withdrawals will deduct this balance.
    pub outstanding: u64,
    /// The starting balance of this vesting account, i.e., how much was
    /// originally deposited.
    pub start_balance: u64,
    /// The unix timestamp at which this vesting account was created.
    pub start_ts: i64,
    /// The ts at which all the tokens associated with this account
    /// should be vested.
    pub end_ts: i64,
    /// The number of times vesting will occur. For example, if vesting
    /// is once a year over seven years, this will be 7.
    pub period_count: u64,
    /// The amount of tokens in custody of whitelisted programs.
    pub whitelist_owned: u64,
    /// Signer nonce.
    pub nonce: u8,
}

#[error]
pub enum ErrorCode {
    #[msg("Vesting end must be greater than the current unix timestamp.")]
    InvalidTimestamp,
    #[msg("The number of vesting periods must be greater than zero.")]
    InvalidPeriod,
    #[msg("The vesting deposit amount must be greater than zero.")]
    InvalidDepositAmount,
    #[msg("The Whitelist entry is not a valid program address.")]
    InvalidWhitelistEntry,
    #[msg("Invalid program address. Did you provide the correct nonce?")]
    InvalidProgramAddress,
    #[msg("Invalid vault owner.")]
    InvalidVaultOwner,
    #[msg("Vault amount must be zero.")]
    InvalidVaultAmount,
    #[msg("Insufficient withdrawal balance.")]
    InsufficienWithdrawalBalance,
    #[msg("Whitelist is full")]
    WhitelistFull,
    #[msg("Whitelist entry already exists")]
    WhitelistEntryAlreadyExists,
    #[msg("Balance must go up when performing a whitelist deposit")]
    InsufficientWhitelistDepositAmount,
    #[msg("Cannot deposit more than withdrawn")]
    WhitelistDepositOverflow,
    #[msg("Tried to withdraw over the specified limit")]
    WhitelistWithdrawLimit,
}

impl<'a, 'b, 'c, 'info> From<&mut CreateVesting<'info>>
    for CpiContext<'a, 'b, 'c, 'info, Transfer<'info>>
{
    fn from(accounts: &mut CreateVesting<'info>) -> CpiContext<'a, 'b, 'c, 'info, Transfer<'info>> {
        let cpi_accounts = Transfer {
            from: accounts.depositor.clone(),
            to: accounts.vault.to_account_info(),
            authority: accounts.depositor_authority.clone(),
        };
        let cpi_program = accounts.token_program.clone();
        CpiContext::new(cpi_program, cpi_accounts)
    }
}

impl<'a, 'b, 'c, 'info> From<&mut Withdraw<'info>>
    for CpiContext<'a, 'b, 'c, 'info, Transfer<'info>>
{
    fn from(accounts: &mut Withdraw<'info>) -> CpiContext<'a, 'b, 'c, 'info, Transfer<'info>> {
        let cpi_accounts = Transfer {
            from: accounts.vault.to_account_info(),
            to: accounts.token.to_account_info(),
            authority: accounts.vault_authority.to_account_info(),
        };
        let cpi_program = accounts.token_program.to_account_info();
        CpiContext::new(cpi_program, cpi_accounts)
    }
}

fn vault_signer_seeds<'a>(
    safe: &'a Pubkey,
    beneficiary: &'a Pubkey,
    nonce: &'a u8,
) -> [&'a [u8]; 3] {
    [
        safe.as_ref(),
        beneficiary.as_ref(),
        bytemuck::bytes_of(nonce),
    ]
}
