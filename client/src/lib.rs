//! `anchor_client` provides an RPC client to send transactions and fetch
//! deserialized accounts from Solana programs written in `anchor_lang`.

use anchor_lang::solana_program::instruction::{AccountMeta, Instruction};
use anchor_lang::solana_program::program_error::ProgramError;
use anchor_lang::solana_program::pubkey::Pubkey;
use anchor_lang::solana_program::system_program;
use anchor_lang::{AccountDeserialize, InstructionData, ToAccountMetas};
use regex::Regex;
use solana_client::client_error::ClientError as SolanaClientError;
use solana_client::pubsub_client::{PubsubClient, PubsubClientError, PubsubClientSubscription};
use solana_client::rpc_client::RpcClient;
use solana_client::rpc_config::{RpcTransactionLogsConfig, RpcTransactionLogsFilter};
use solana_client::rpc_response::{Response as RpcResponse, RpcLogsResponse};
use solana_sdk::commitment_config::CommitmentConfig;
use solana_sdk::signature::{Keypair, Signature, Signer};
use solana_sdk::transaction::Transaction;
use std::convert::Into;
use thiserror::Error;

pub use anchor_lang;
pub use cluster::Cluster;
pub use solana_client;
pub use solana_sdk;

mod cluster;

/// EventHandle unsubscribes from a program event stream on drop.
pub type EventHandle = PubsubClientSubscription<RpcResponse<RpcLogsResponse>>;

/// Client defines the base configuration for building RPC clients to
/// communitcate with Anchor programs running on a Solana cluster. It's
/// primary use is to build a `Program` client via the `program` method.
pub struct Client {
    cfg: Config,
}

impl Client {
    pub fn new(cluster: Cluster, payer: Keypair) -> Self {
        Self {
            cfg: Config {
                cluster,
                payer,
                options: None,
            },
        }
    }

    pub fn new_with_options(cluster: Cluster, payer: Keypair, options: CommitmentConfig) -> Self {
        Self {
            cfg: Config {
                cluster,
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
                options: self.cfg.options,
                payer: Keypair::from_bytes(&self.cfg.payer.to_bytes()).unwrap(),
            },
        }
    }
}

// Internal configuration for a client.
struct Config {
    cluster: Cluster,
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
        RequestBuilder::from(
            self.program_id,
            self.cfg.cluster.url(),
            Keypair::from_bytes(&self.cfg.payer.to_bytes()).unwrap(),
            self.cfg.options,
            RequestNamespace::Global,
        )
    }

    /// Returns a request builder for program state.
    pub fn state_request(&self) -> RequestBuilder {
        RequestBuilder::from(
            self.program_id,
            self.cfg.cluster.url(),
            Keypair::from_bytes(&self.cfg.payer.to_bytes()).unwrap(),
            self.cfg.options,
            RequestNamespace::State { new: false },
        )
    }

    /// Returns the account at the given address.
    pub fn account<T: AccountDeserialize>(&self, address: Pubkey) -> Result<T, ClientError> {
        let rpc_client = RpcClient::new_with_commitment(
            self.cfg.cluster.url().to_string(),
            self.cfg.options.unwrap_or_default(),
        );
        let account = rpc_client
            .get_account_with_commitment(&address, CommitmentConfig::processed())?
            .value
            .ok_or(ClientError::AccountNotFound)?;
        let mut data: &[u8] = &account.data;
        T::try_deserialize(&mut data).map_err(Into::into)
    }

    pub fn state<T: AccountDeserialize>(&self) -> Result<T, ClientError> {
        self.account(anchor_lang::__private::state::address(&self.program_id))
    }

    pub fn rpc(&self) -> RpcClient {
        RpcClient::new_with_commitment(
            self.cfg.cluster.url().to_string(),
            self.cfg.options.unwrap_or_default(),
        )
    }

    pub fn id(&self) -> Pubkey {
        self.program_id
    }

    pub fn on<T: anchor_lang::Event + anchor_lang::AnchorDeserialize>(
        &self,
        f: impl Fn(&EventContext, T) + Send + 'static,
    ) -> Result<EventHandle, ClientError> {
        let addresses = vec![self.program_id.to_string()];
        let filter = RpcTransactionLogsFilter::Mentions(addresses);
        let ws_url = self.cfg.cluster.ws_url().to_string();
        let cfg = RpcTransactionLogsConfig {
            commitment: self.cfg.options,
        };
        let self_program_str = self.program_id.to_string();
        let (client, receiver) = PubsubClient::logs_subscribe(&ws_url, filter, cfg)?;
        std::thread::spawn(move || {
            loop {
                match receiver.recv() {
                    Ok(logs) => {
                        let ctx = EventContext {
                            signature: logs.value.signature.parse().unwrap(),
                            slot: logs.context.slot,
                        };
                        let mut logs = &logs.value.logs[..];
                        if !logs.is_empty() {
                            if let Ok(mut execution) = Execution::new(&mut logs) {
                                for l in logs {
                                    // Parse the log.
                                    let (event, new_program, did_pop) = {
                                        if self_program_str == execution.program() {
                                            handle_program_log(&self_program_str, l).unwrap_or_else(
                                                |e| {
                                                    println!(
                                                        "Unable to parse log: {}",
                                                        e.to_string()
                                                    );
                                                    std::process::exit(1);
                                                },
                                            )
                                        } else {
                                            let (program, did_pop) =
                                                handle_system_log(&self_program_str, l);
                                            (None, program, did_pop)
                                        }
                                    };
                                    // Emit the event.
                                    if let Some(e) = event {
                                        f(&ctx, e);
                                    }
                                    // Switch program context on CPI.
                                    if let Some(new_program) = new_program {
                                        execution.push(new_program);
                                    }
                                    // Program returned.
                                    if did_pop {
                                        execution.pop();
                                    }
                                }
                            }
                        }
                    }
                    Err(_err) => {
                        return;
                    }
                }
            }
        });
        Ok(client)
    }
}

fn handle_program_log<T: anchor_lang::Event + anchor_lang::AnchorDeserialize>(
    self_program_str: &str,
    l: &str,
) -> Result<(Option<T>, Option<String>, bool), ClientError> {
    // Log emitted from the current program.
    if l.starts_with("Program log:") {
        let log = l.to_string().split_off("Program log: ".len());
        let borsh_bytes = anchor_lang::__private::base64::decode(log)
            .map_err(|_| ClientError::LogParseError(l.to_string()))?;

        let mut slice: &[u8] = &borsh_bytes[..];
        let disc: [u8; 8] = {
            let mut disc = [0; 8];
            disc.copy_from_slice(&borsh_bytes[..8]);
            slice = &slice[8..];
            disc
        };
        let mut event = None;
        if disc == T::discriminator() {
            let e: T = anchor_lang::AnchorDeserialize::deserialize(&mut slice)
                .map_err(|e| ClientError::LogParseError(e.to_string()))?;
            event = Some(e);
        }
        Ok((event, None, false))
    }
    // System log.
    else {
        let (program, did_pop) = handle_system_log(self_program_str, l);
        Ok((None, program, did_pop))
    }
}

fn handle_system_log(this_program_str: &str, log: &str) -> (Option<String>, bool) {
    if log.starts_with(&format!("Program {} log:", this_program_str)) {
        (Some(this_program_str.to_string()), false)
    } else if log.contains("invoke") {
        (Some("cpi".to_string()), false) // Any string will do.
    } else {
        let re = Regex::new(r"^Program (.*) success*$").unwrap();
        if re.is_match(log) {
            (None, true)
        } else {
            (None, false)
        }
    }
}

struct Execution {
    stack: Vec<String>,
}

impl Execution {
    pub fn new(logs: &mut &[String]) -> Result<Self, ClientError> {
        let l = &logs[0];
        *logs = &logs[1..];

        let re = Regex::new(r"^Program (.*) invoke.*$").unwrap();
        let c = re
            .captures(l)
            .ok_or_else(|| ClientError::LogParseError(l.to_string()))?;
        let program = c
            .get(1)
            .ok_or_else(|| ClientError::LogParseError(l.to_string()))?
            .as_str()
            .to_string();
        Ok(Self {
            stack: vec![program],
        })
    }

    pub fn program(&self) -> String {
        assert!(!self.stack.is_empty());
        self.stack[self.stack.len() - 1].clone()
    }

    pub fn push(&mut self, new_program: String) {
        self.stack.push(new_program);
    }

    pub fn pop(&mut self) {
        assert!(!self.stack.is_empty());
        self.stack.pop().unwrap();
    }
}

#[derive(Debug)]
pub struct EventContext {
    pub signature: Signature,
    pub slot: u64,
}

#[derive(Debug, Error)]
pub enum ClientError {
    #[error("Account not found")]
    AccountNotFound,
    #[error("{0}")]
    ProgramError(#[from] ProgramError),
    #[error("{0}")]
    SolanaClientError(#[from] SolanaClientError),
    #[error("{0}")]
    SolanaClientPubsubError(#[from] PubsubClientError),
    #[error("Unable to parse log: {0}")]
    LogParseError(String),
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
    // True if the user is sending a state instruction.
    namespace: RequestNamespace,
}

#[derive(PartialEq)]
pub enum RequestNamespace {
    Global,
    State {
        // True if the request is to the state's new ctor.
        new: bool,
    },
    Interface,
}

impl<'a> RequestBuilder<'a> {
    pub fn from(
        program_id: Pubkey,
        cluster: &str,
        payer: Keypair,
        options: Option<CommitmentConfig>,
        namespace: RequestNamespace,
    ) -> Self {
        Self {
            program_id,
            payer,
            cluster: cluster.to_string(),
            accounts: Vec::new(),
            options: options.unwrap_or_default(),
            instructions: Vec::new(),
            instruction_data: None,
            signers: Vec::new(),
            namespace,
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

    /// Invokes the `#[state]`'s `new` constructor.
    #[allow(clippy::wrong_self_convention)]
    pub fn new(mut self, args: impl InstructionData) -> Self {
        assert!(self.namespace == RequestNamespace::State { new: false });
        self.namespace = RequestNamespace::State { new: true };
        self.instruction_data = Some(args.data());
        self
    }

    pub fn signer(mut self, signer: &'a dyn Signer) -> Self {
        self.signers.push(signer);
        self
    }

    pub fn send(self) -> Result<Signature, ClientError> {
        let accounts = match self.namespace {
            RequestNamespace::State { new } => {
                let mut accounts = match new {
                    false => vec![AccountMeta::new(
                        anchor_lang::__private::state::address(&self.program_id),
                        false,
                    )],
                    true => vec![
                        AccountMeta::new_readonly(self.payer.pubkey(), true),
                        AccountMeta::new(
                            anchor_lang::__private::state::address(&self.program_id),
                            false,
                        ),
                        AccountMeta::new_readonly(
                            Pubkey::find_program_address(&[], &self.program_id).0,
                            false,
                        ),
                        AccountMeta::new_readonly(system_program::ID, false),
                        AccountMeta::new_readonly(self.program_id, false),
                    ],
                };
                accounts.extend_from_slice(&self.accounts);
                accounts
            }
            _ => self.accounts,
        };
        let mut instructions = self.instructions;
        if let Some(ix_data) = self.instruction_data {
            instructions.push(Instruction {
                program_id: self.program_id,
                data: ix_data,
                accounts,
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

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn new_execution() {
        let mut logs: &[String] =
            &["Program 7Y8VDzehoewALqJfyxZYMgYCnMTCDhWuGfJKUvjYWATw invoke [1]".to_string()];
        let exe = Execution::new(&mut logs).unwrap();
        assert_eq!(
            exe.stack[0],
            "7Y8VDzehoewALqJfyxZYMgYCnMTCDhWuGfJKUvjYWATw".to_string()
        );
    }

    #[test]
    fn handle_system_log_pop() {
        let log = "Program 7Y8VDzehoewALqJfyxZYMgYCnMTCDhWuGfJKUvjYWATw success";
        let (program, did_pop) = handle_system_log("asdf", log);
        assert_eq!(program, None);
        assert!(did_pop);
    }

    #[test]
    fn handle_system_log_no_pop() {
        let log = "Program 7swsTUiQ6KUK4uFYquQKg4epFRsBnvbrTf2fZQCa2sTJ qwer";
        let (program, did_pop) = handle_system_log("asdf", log);
        assert_eq!(program, None);
        assert!(!did_pop);
    }
}
