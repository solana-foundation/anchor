use crate::{
    AsSigner, ClientError, Config, EventContext, EventUnsubscriber, Program,
    ProgramAccountsIterator, RequestBuilder,
};
use anchor_lang::{prelude::Pubkey, AccountDeserialize, Discriminator};
use solana_client::{rpc_config::RpcSendTransactionConfig, rpc_filter::RpcFilterType};
use solana_sdk::{
    commitment_config::CommitmentConfig, signature::Signature, signer::Signer,
    transaction::Transaction,
};
use std::{marker::PhantomData, ops::Deref, sync::Arc};
use tokio::sync::RwLock;

impl<'a> EventUnsubscriber<'a> {
    /// Unsubscribe gracefully.
    pub async fn unsubscribe(self) {
        self.unsubscribe_internal().await
    }
}

pub trait ThreadSafeSigner: Signer + Send + Sync + 'static {
    fn to_signer(&self) -> &dyn Signer;
}

impl<T: Signer + Send + Sync + 'static> ThreadSafeSigner for T {
    fn to_signer(&self) -> &dyn Signer {
        self
    }
}

impl AsSigner for Arc<dyn ThreadSafeSigner> {
    fn as_signer(&self) -> &dyn Signer {
        self.to_signer()
    }
}

impl<C: Deref<Target = impl Signer> + Clone> Program<C> {
    pub fn new(program_id: Pubkey, cfg: Config<C>) -> Result<Self, ClientError> {
        Ok(Self {
            program_id,
            cfg,
            sub_client: Arc::new(RwLock::new(None)),
        })
    }

    /// Returns a threadsafe request builder
    pub fn request(&self) -> RequestBuilder<'_, C, Arc<dyn ThreadSafeSigner>> {
        RequestBuilder::from(
            self.program_id,
            self.cfg.cluster.url(),
            self.cfg.payer.clone(),
            self.cfg.options,
        )
    }

    /// Returns the account at the given address.
    pub async fn account<T: AccountDeserialize>(&self, address: Pubkey) -> Result<T, ClientError> {
        self.account_internal(address).await
    }

    /// Returns all program accounts of the given type matching the given filters
    pub async fn accounts<T: AccountDeserialize + Discriminator>(
        &self,
        filters: Vec<RpcFilterType>,
    ) -> Result<Vec<(Pubkey, T)>, ClientError> {
        self.accounts_lazy(filters).await?.collect()
    }

    /// Returns all program accounts of the given type matching the given filters as an iterator
    /// Deserialization is executed lazily
    pub async fn accounts_lazy<T: AccountDeserialize + Discriminator>(
        &self,
        filters: Vec<RpcFilterType>,
    ) -> Result<ProgramAccountsIterator<T>, ClientError> {
        self.accounts_lazy_internal(filters).await
    }

    /// Subscribe to program logs.
    ///
    /// Returns an [`EventUnsubscriber`] to unsubscribe and close connection gracefully.
    pub async fn on<T: anchor_lang::Event + anchor_lang::AnchorDeserialize>(
        &self,
        f: impl Fn(&EventContext, T) + Send + 'static,
    ) -> Result<EventUnsubscriber, ClientError> {
        let (handle, rx) = self.on_internal(f).await?;

        Ok(EventUnsubscriber {
            handle,
            rx,
            _lifetime_marker: PhantomData,
        })
    }
}

impl<'a, C: Deref<Target = impl Signer> + Clone> RequestBuilder<'a, C, Arc<dyn ThreadSafeSigner>> {
    pub fn from(
        program_id: Pubkey,
        cluster: &str,
        payer: C,
        options: Option<CommitmentConfig>,
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
            _phantom: PhantomData,
        }
    }

    #[must_use]
    pub fn signer<T: ThreadSafeSigner>(mut self, signer: T) -> Self {
        self.signers.push(Arc::new(signer));
        self
    }

    pub async fn signed_transaction(&self) -> Result<Transaction, ClientError> {
        self.signed_transaction_internal().await
    }

    pub async fn send(self) -> Result<Signature, ClientError> {
        self.send_internal().await
    }

    pub async fn send_with_spinner_and_config(
        self,
        config: RpcSendTransactionConfig,
    ) -> Result<Signature, ClientError> {
        self.send_with_spinner_and_config_internal(config).await
    }
}
