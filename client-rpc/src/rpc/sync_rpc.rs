use jsonrpc_core::Result;
use jsonrpc_derive::rpc;

use crate::server::{to_rpc_error, WalletRequest};
use client_common::tendermint::Client;
use client_common::Storage;
use client_core::synchronizer::PollingSynchronizer;
use client_core::wallet::syncer::{ObfuscationSyncerConfig, WalletSyncer};
use client_core::TransactionObfuscation;

#[rpc]
pub trait SyncRpc: Send + Sync {
    #[rpc(name = "sync")]
    fn sync(&self, request: WalletRequest) -> Result<()>;

    #[rpc(name = "sync_all")]
    fn sync_all(&self, request: WalletRequest) -> Result<()>;

    #[rpc(name = "sync_unlockWallet")]
    fn sync_unlock_wallet(&self, request: WalletRequest) -> Result<()>;

    #[rpc(name = "sync_stop")]
    fn sync_stop(&self, request: WalletRequest) -> Result<()>;
}

pub struct SyncRpcImpl<S, C, O>
where
    S: Storage,
    C: Client,
    O: TransactionObfuscation,
{
    config: ObfuscationSyncerConfig<S, C, O>,
    polling_synchronizer: PollingSynchronizer,
}

impl<S, C, O> SyncRpc for SyncRpcImpl<S, C, O>
where
    S: Storage + 'static,
    C: Client + 'static,
    O: TransactionObfuscation + 'static,
{
    #[inline]
    fn sync(&self, request: WalletRequest) -> Result<()> {
        self.do_sync(request, false)
    }

    #[inline]
    fn sync_all(&self, request: WalletRequest) -> Result<()> {
        self.do_sync(request, true)
    }

    #[inline]
    fn sync_unlock_wallet(&self, request: WalletRequest) -> Result<()> {
        self.polling_synchronizer
            .add_wallet(request.name, request.passphrase);
        Ok(())
    }

    #[inline]
    fn sync_stop(&self, request: WalletRequest) -> Result<()> {
        self.polling_synchronizer.remove_wallet(&request.name);
        Ok(())
    }
}

impl<S, C, O> SyncRpcImpl<S, C, O>
where
    S: Storage + 'static,
    C: Client + 'static,
    O: TransactionObfuscation + 'static,
{
    pub fn new(config: ObfuscationSyncerConfig<S, C, O>) -> Self {
        let mut polling_synchronizer = PollingSynchronizer::default();
        polling_synchronizer.spawn(config.clone());

        SyncRpcImpl {
            config,
            polling_synchronizer,
        }
    }

    fn do_sync(&self, request: WalletRequest, reset: bool) -> Result<()> {
        let syncer = WalletSyncer::with_obfuscation_config(
            self.config.clone(),
            None,
            request.name,
            request.passphrase,
        )
        .map_err(to_rpc_error)?;
        if reset {
            syncer.reset_state().map_err(to_rpc_error)?;
        }
        syncer.sync().map_err(to_rpc_error)
    }
}
