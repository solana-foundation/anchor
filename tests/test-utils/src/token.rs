use anyhow::{Result, anyhow};
use solana_account::Account;
use solana_program_option::COption;
use solana_program_pack::Pack;
use solana_pubkey::Pubkey;
use solana_sysvar::rent::Rent;
use spl_token_interface::state::{Account as TokenAccount, AccountState, Mint};

use crate::AccountBuilderBase;

use super::TestContext;

pub use spl_token_interface::ID;

impl TestContext {
    pub fn init_native_mint(&mut self) -> Result<Pubkey> {
        let native_mint = spl_token_interface::native_mint::ID;
        self.create_mint_builder()
            .pubkey(native_mint)
            .decimals(spl_token_interface::native_mint::DECIMALS)
            .create()
    }

    pub fn associated_token_address(&self, owner: Pubkey, mint: Pubkey) -> Pubkey {
        spl_associated_token_account_interface::address::get_associated_token_address_with_program_id(
            &owner,
            &mint,
            &spl_token_interface::ID,
        )
    }

    pub fn create_mint_builder(&mut self) -> MintAccountBuilder<'_> {
        let rent = Rent::default();

        MintAccountBuilder {
            ctx: self,
            address: Pubkey::default(),
            account_state: Account {
                lamports: rent.minimum_balance(Mint::LEN),
                data: vec![0; Mint::LEN],
                owner: spl_token_interface::id(),
                executable: false,
                rent_epoch: 0,
            },
            mint: Mint {
                mint_authority: COption::None,
                supply: 0,
                decimals: 0,
                is_initialized: true,
                freeze_authority: COption::None,
            },
        }
    }

    pub fn create_token_account_builder(
        &mut self,
        mint: Pubkey,
        owner: Pubkey,
    ) -> TokenAccountBuilder<'_> {
        let rent = Rent::default();
        TokenAccountBuilder {
            ctx: self,
            address: Pubkey::default(),
            account_state: Account {
                lamports: rent.minimum_balance(spl_token_interface::state::Account::LEN),
                data: vec![0; spl_token_interface::state::Account::LEN],
                owner: spl_token_interface::id(),
                executable: false,
                rent_epoch: 0,
            },
            token_state: spl_token_interface::state::Account {
                mint,
                owner,
                amount: 0,
                delegate: COption::None,
                state: AccountState::Initialized,
                is_native: COption::None,
                delegated_amount: 0,
                close_authority: COption::None,
            },
        }
    }

    pub fn create_associated_token_account_builder(
        &mut self,
        mint: Pubkey,
        owner: Pubkey,
    ) -> TokenAccountBuilder<'_> {
        let ata = self.associated_token_address(owner, mint);
        self.create_token_account_builder(mint, owner).pubkey(ata)
    }

    pub fn set_token_account_delegate(
        &mut self,
        token_account: Pubkey,
        delegate: Option<Pubkey>,
        delegated_amount: u64,
    ) -> Result<()> {
        let mut account = self
            .svm
            .get_account(&token_account)
            .ok_or_else(|| anyhow!("token account not found: {token_account}"))?;
        let mut token_state = TokenAccount::unpack(&account.data)?;
        token_state.delegate = match delegate {
            Some(pk) => COption::Some(pk),
            None => COption::None,
        };
        token_state.delegated_amount = delegated_amount;
        TokenAccount::pack(token_state, &mut account.data)?;
        self.svm.set_account(token_account, account)?;
        Ok(())
    }
}

pub struct MintAccountBuilder<'a> {
    pub(crate) ctx: &'a mut TestContext,
    pub(crate) address: Pubkey,
    pub(crate) account_state: Account,
    pub(crate) mint: Mint,
}

impl AccountBuilderBase for MintAccountBuilder<'_> {
    fn account_state_mut(&mut self) -> &mut Account {
        &mut self.account_state
    }
    fn address_mut(&mut self) -> &mut Pubkey {
        &mut self.address
    }
}

impl MintAccountBuilder<'_> {
    pub fn create(mut self) -> Result<Pubkey> {
        if self.address == Pubkey::default() {
            self.address = Pubkey::new_unique();
        }
        let mut account = self.account_state;
        Mint::pack(self.mint, &mut account.data)?;
        let _ = self.ctx.svm.set_account(self.address, account);
        Ok(self.address)
    }

    pub fn decimals(mut self, decimals: u8) -> Self {
        self.mint.decimals = decimals;
        self
    }

    pub fn mint_authority(mut self, address: Pubkey) -> Self {
        self.mint.mint_authority = COption::Some(address);
        self
    }

    pub fn freeze_authority(mut self, address: Pubkey) -> Self {
        self.mint.freeze_authority = COption::Some(address);
        self
    }
}

pub struct TokenAccountBuilder<'a> {
    pub(crate) ctx: &'a mut TestContext,
    pub(crate) address: Pubkey,
    pub(crate) account_state: Account,
    pub(crate) token_state: TokenAccount,
}

impl AccountBuilderBase for TokenAccountBuilder<'_> {
    fn account_state_mut(&mut self) -> &mut Account {
        &mut self.account_state
    }
    fn address_mut(&mut self) -> &mut Pubkey {
        &mut self.address
    }
}

impl TokenAccountBuilder<'_> {
    pub fn create(mut self) -> Result<Pubkey> {
        if self.address == Pubkey::default() {
            self.address = Pubkey::new_unique();
        }
        let mut account = self.account_state;
        TokenAccount::pack(self.token_state, &mut account.data)?;
        let _ = self.ctx.svm.set_account(self.address, account);
        Ok(self.address)
    }

    pub fn mint(mut self, mint: Pubkey) -> Self {
        self.token_state.mint = mint;
        self
    }

    pub fn token_owner(mut self, owner: Pubkey) -> Self {
        self.token_state.owner = owner;
        self
    }

    pub fn amount(mut self, amount: u64) -> Self {
        self.token_state.amount = amount;
        self
    }

    pub fn delegate(mut self, delegate: Option<Pubkey>) -> Self {
        self.token_state.delegate = match delegate {
            Some(pk) => COption::Some(pk),
            None => COption::None,
        };
        self
    }

    pub fn state(mut self, state: AccountState) -> Self {
        self.token_state.state = state;
        self
    }

    pub fn is_native(mut self, native_amount: Option<u64>) -> Self {
        self.token_state.is_native = match native_amount {
            Some(amount) => COption::Some(amount),
            None => COption::None,
        };
        self
    }

    pub fn delegated_amount(mut self, amount: u64) -> Self {
        self.token_state.delegated_amount = amount;
        self
    }

    pub fn close_authority(mut self, authority: Option<Pubkey>) -> Self {
        self.token_state.close_authority = match authority {
            Some(pk) => COption::Some(pk),
            None => COption::None,
        };
        self
    }
}
