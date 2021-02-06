//! `anchor_client` provides an RPC client to send transactions and fetch
//! deserialized accounts from Solana programs written in `anchor_lang`.

use anchor_lang::solana_program::instruction::{AccountMeta, Instruction};
use anchor_lang::solana_program::program_error::ProgramError;
use anchor_lang::solana_program::pubkey::Pubkey;
use anchor_lang::{AccountDeserialize, InstructionData, ToAccountMetas};
use solana_client::client_error::ClientError as SolanaClientError;
use solana_client::rpc_client::RpcClient;
use solana_sdk::commitment_config::CommitmentConfig;
use solana_sdk::signature::{Keypair, Signature, Signer};
use solana_sdk::transaction::Transaction;
use std::convert::Into;
use thiserror::Error;

pub use anchor_lang;
pub use solana_client;
pub use solana_sdk;

/// Client defines the base configuration for building RPC clients to
/// communitcate with Anchor programs running on a Solana cluster. It's
/// primary use is to build a `Program` client via the `program` method.
pub struct Client {
    cfg: Config,
}

impl Client {
    pub fn new(cluster: &str, payer: Keypair) -> Self {
        Self {
            cfg: Config {
                cluster: cluster.to_string(),
                payer,
                options: None,
            },
        }
    }

    pub fn new_with_options(cluster: &str, payer: Keypair, options: CommitmentConfig) -> Self {
        Self {
            cfg: Config {
                cluster: cluster.to_string(),
                payer,
                options: Some(options),
            },
        }
    }

    pub fn program(&self, program_id: Pubkey) -> Program {
        Program {
            program_id,
            cfg: Config {
                cluster: self.cfg.cluster.clone(),
                options: self.cfg.options.clone(),
                payer: Keypair::from_bytes(&self.cfg.payer.to_bytes()).unwrap(),
            },
        }
    }
}

// Internal configuration for a client.
struct Config {
    cluster: String,
    payer: Keypair,
    options: Option<CommitmentConfig>,
}

/// Program is the primary client handle to be used to build and send requests.
pub struct Program {
    program_id: Pubkey,
    cfg: Config,
}

impl Program {
    pub fn payer(&self) -> Pubkey {
        self.cfg.payer.pubkey()
    }

    /// Returns a request builder.
    pub fn request(&self) -> RequestBuilder {
        RequestBuilder::new(
            self.program_id,
            &self.cfg.cluster,
            Keypair::from_bytes(&self.cfg.payer.to_bytes()).unwrap(),
            self.cfg.options.clone(),
        )
    }

    /// Returns the account at the given address.
    pub fn account<T: AccountDeserialize>(&self, address: Pubkey) -> Result<T, ClientError> {
        let rpc_client = RpcClient::new_with_commitment(
            self.cfg.cluster.clone(),
            self.cfg.options.unwrap_or(Default::default()),
        );
        let account = rpc_client
            .get_account_with_commitment(&address, CommitmentConfig::recent())?
            .value
            .ok_or(ClientError::AccountNotFound)?;
        let mut data: &[u8] = &account.data;
        T::try_deserialize(&mut data).map_err(Into::into)
    }

    pub fn rpc(&self) -> RpcClient {
        RpcClient::new_with_commitment(
            self.cfg.cluster.clone(),
            self.cfg.options.unwrap_or(Default::default()),
        )
    }

    pub fn id(&self) -> Pubkey {
        self.program_id
    }
}

#[derive(Debug, Error)]
pub enum ClientError {
    #[error("Account not found")]
    AccountNotFound,
    #[error("{0}")]
    ProgramError(#[from] ProgramError),
    #[error("{0}")]
    SolanaClientError(#[from] SolanaClientError),
}

/// `RequestBuilder` provides a builder interface to create and send
/// transactions to a cluster.
pub struct RequestBuilder<'a> {
    cluster: String,
    program_id: Pubkey,
    accounts: Vec<AccountMeta>,
    options: CommitmentConfig,
    instructions: Vec<Instruction>,
    payer: Keypair,
    // Serialized instruction data for the target RPC.
    instruction_data: Option<Vec<u8>>,
    signers: Vec<&'a dyn Signer>,
}

impl<'a> RequestBuilder<'a> {
    pub fn new(
        program_id: Pubkey,
        cluster: &str,
        payer: Keypair,
        options: Option<CommitmentConfig>,
    ) -> Self {
        Self {
            program_id,
            payer,
            cluster: cluster.to_string(),
            accounts: Vec::new(),
            options: options.unwrap_or(Default::default()),
            instructions: Vec::new(),
            instruction_data: None,
            signers: Vec::new(),
        }
    }

    pub fn payer(mut self, payer: Keypair) -> Self {
        self.payer = payer;
        self
    }

    pub fn cluster(mut self, url: &str) -> Self {
        self.cluster = url.to_string();
        self
    }

    pub fn instruction(mut self, ix: Instruction) -> Self {
        self.instructions.push(ix);
        self
    }

    pub fn program(mut self, program_id: Pubkey) -> Self {
        self.program_id = program_id;
        self
    }

    pub fn accounts(mut self, accounts: impl ToAccountMetas) -> Self {
        let mut metas = accounts.to_account_metas(None);
        self.accounts.append(&mut metas);
        self
    }

    pub fn options(mut self, options: CommitmentConfig) -> Self {
        self.options = options;
        self
    }

    pub fn args(mut self, args: impl InstructionData) -> Self {
        self.instruction_data = Some(args.data());
        self
    }

    pub fn signer(mut self, signer: &'a dyn Signer) -> Self {
        self.signers.push(signer);
        self
    }

    pub fn send(self) -> Result<Signature, ClientError> {
        let mut instructions = self.instructions;
        if let Some(ix_data) = self.instruction_data {
            instructions.push(Instruction {
                program_id: self.program_id,
                data: ix_data,
                accounts: self.accounts,
            });
        }

        let mut signers = self.signers;
        signers.push(&self.payer);

        let rpc_client = RpcClient::new_with_commitment(self.cluster, self.options);

        let tx = {
            let (recent_hash, _fee_calc) = rpc_client.get_recent_blockhash()?;
            Transaction::new_signed_with_payer(
                &instructions,
                Some(&self.payer.pubkey()),
                &signers,
                recent_hash,
            )
        };

        rpc_client
            .send_and_confirm_transaction(&tx)
            .map_err(Into::into)
    }
}
