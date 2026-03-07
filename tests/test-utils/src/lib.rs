use anchor_lang::{InstructionData, ToAccountMetas};
use anyhow::{Result, bail};
use litesvm::{LiteSVM, types::TransactionResult};
use solana_account::{Account, state_traits::StateMut};
use solana_keypair::{Keypair, Signer};
use solana_loader_v3_interface::state::UpgradeableLoaderState;
use solana_pubkey::Pubkey;
use solana_rpc_client::rpc_client::RpcClient;
use solana_sdk_ids::{bpf_loader, bpf_loader_deprecated, bpf_loader_upgradeable};
use solana_transaction::{Instruction, Transaction};

use std::cell::LazyCell;
use std::path::Path;
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

pub mod token;

pub struct TestContext {
    svm: LiteSVM,
    rpc: LazyCell<RpcClient>,
    payer: Keypair,
}

impl TestContext {
    pub fn new() -> Self {
        Self {
            svm: LiteSVM::new(),
            rpc: LazyCell::new(|| RpcClient::new("https://api.mainnet.solana.com")),
            payer: Keypair::new(),
        }
    }

    pub fn payer(&self) -> &Keypair {
        &self.payer
    }

    pub fn payer_pubkey(&self) -> Pubkey {
        self.payer.pubkey()
    }

    pub fn svm(&mut self) -> &mut LiteSVM {
        &mut self.svm
    }

    /// Airdrop lamports to the given address
    pub fn airdrop(&mut self, address: Pubkey, lamports: u64) -> Result<()> {
        self.svm.airdrop(&address, lamports).map_err(|e| e.err)?;
        Ok(())
    }

    /// Like [`Self::airdrop`], but to [`Self::payer`]
    pub fn airdrop_payer(&mut self, lamports: u64) -> Result<()> {
        self.svm
            .airdrop(&self.payer.pubkey(), lamports)
            .map_err(|e| e.err)?;
        Ok(())
    }

    /// Clone the given program address from mainnet, and install it into LiteSVM at the given address
    pub fn clone_from_mainnet(&mut self, address: Pubkey) -> Result<()> {
        let account = self.rpc.get_account(&address)?;
        let bytes = match account.owner {
            bpf_loader::ID | bpf_loader_deprecated::ID => account.data,
            bpf_loader_upgradeable::ID => {
                let Ok(UpgradeableLoaderState::Program {
                    programdata_address,
                }) = account.state()
                else {
                    bail!("not a program account")
                };
                let programdata_account = self.rpc.get_account(&programdata_address)?;
                assert!(matches!(
                    programdata_account.state(),
                    Ok(UpgradeableLoaderState::ProgramData { .. })
                ));

                let offset = UpgradeableLoaderState::size_of_programdata_metadata();
                programdata_account.data[offset..].to_vec()
            }
            _ => bail!("unsupported program type"),
        };
        self.svm.add_program(address, &bytes)?;

        Ok(())
    }

    /// Build a program with `cargo build-sbf` and deploy the resulting `.so` into LiteSVM.
    pub fn build_and_deploy_sbf_program(
        &mut self,
        manifest_path: impl AsRef<Path>,
        program_binary_name: &str,
        program_id: Pubkey,
    ) -> Result<()> {
        let manifest_path = manifest_path.as_ref();
        let nonce = SystemTime::now().duration_since(UNIX_EPOCH)?.as_nanos();
        let out_dir = std::env::temp_dir().join(format!("anchor-sbf-{nonce}"));
        std::fs::create_dir_all(&out_dir)?;

        let status = Command::new("cargo")
            .arg("build-sbf")
            .arg("--tools-version")
            .arg("v1.52")
            .arg("--manifest-path")
            .arg(manifest_path)
            .arg("--sbf-out-dir")
            .arg(&out_dir)
            .status()?;
        if !status.success() {
            bail!("cargo build-sbf failed for {}", manifest_path.display());
        }

        let program_path = out_dir.join(format!("{program_binary_name}.so"));
        self.svm.add_program_from_file(program_id, program_path)?;
        Ok(())
    }

    /// Send a transaction, signing with `signers` (and additionally `payer` if `sign_with_payer` is set), and using
    /// [`Self::payer`] as a payer.
    #[allow(clippy::result_large_err)]
    pub fn send_signed_transaction_with_payer(
        &mut self,
        instructions: &[Instruction],
        signers: &[&Keypair],
        sign_with_payer: bool,
    ) -> TransactionResult {
        let mut signers = signers.to_vec();
        if sign_with_payer {
            signers.push(&self.payer);
        }

        let tx = Transaction::new_signed_with_payer(
            instructions,
            Some(&self.payer_pubkey()),
            &signers,
            self.svm.latest_blockhash(),
        );
        self.svm.send_transaction(tx)
    }
}

impl Default for TestContext {
    fn default() -> Self {
        Self::new()
    }
}

pub fn anchor_instruction<A: ToAccountMetas, D: InstructionData>(
    program_id: Pubkey,
    accounts: A,
    data: D,
) -> Instruction {
    Instruction {
        program_id,
        accounts: accounts.to_account_metas(None),
        data: data.data(),
    }
}

pub trait AccountBuilderBase: Sized {
    fn account_state_mut(&mut self) -> &mut Account;
    fn address_mut(&mut self) -> &mut Pubkey;

    fn pubkey(mut self, pk: Pubkey) -> Self {
        *self.address_mut() = pk;
        self
    }

    fn owner(mut self, pk: Pubkey) -> Self {
        self.account_state_mut().owner = pk;
        self
    }

    fn executable(mut self, val: bool) -> Self {
        self.account_state_mut().executable = val;
        self
    }

    fn rent_epoch(mut self, val: u64) -> Self {
        self.account_state_mut().rent_epoch = val;
        self
    }

    fn lamports(mut self, amount: u64) -> Self {
        self.account_state_mut().lamports = amount;
        self
    }

    fn size(mut self, length: usize) -> Self {
        self.account_state_mut().data = vec![0; length];
        self
    }

    fn data(mut self, bytes: &[u8]) -> Self {
        self.account_state_mut().data = Vec::from(bytes);
        self
    }
}
