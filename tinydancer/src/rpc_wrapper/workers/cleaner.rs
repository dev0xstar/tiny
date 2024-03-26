use std::{sync::Arc, time::Duration};

use tiny_logger::logs::info;
use tokio::task::JoinHandle;

use crate::rpc_wrapper::{block_store::BlockStore, tpu_manager::TpuManager};

use super::{BlockListener, TxSender};

/// Background worker which cleans up memory  
#[derive(Clone)]
pub struct Cleaner {
    tx_sender: TxSender,
    block_listenser: BlockListener,
    block_store: BlockStore,
    tpu_manager: Arc<TpuManager>,
}

impl Cleaner {
    pub fn new(
        tx_sender: TxSender,
        block_listenser: BlockListener,
        block_store: BlockStore,
        tpu_manager: Arc<TpuManager>,
    ) -> Self {
        Self {
            tx_sender,
            block_listenser,
            block_store,
            tpu_manager,
        }
    }

    pub fn clean_tx_sender(&self, ttl_duration: Duration) {
        let length_before = self.tx_sender.txs_sent_store.len();
        self.tx_sender
            .txs_sent_store
            .retain(|_k, v| v.sent_at.elapsed() < ttl_duration);
        info!(
            "Cleaned {} transactions",
            length_before - self.tx_sender.txs_sent_store.len()
        );
    }

    /// Clean Signature Subscribers from Block Listeners
    pub fn clean_block_listeners(&self, ttl_duration: Duration) {
        self.block_listenser.clean(ttl_duration);
    }

    pub async fn clean_block_store(&self, ttl_duration: Duration) {
        self.block_store.clean(ttl_duration).await;
    }

    pub fn start(self, ttl_duration: Duration) -> JoinHandle<anyhow::Result<()>> {
        let mut ttl = tokio::time::interval(ttl_duration);

        tokio::spawn(async move {
            info!("Cleaning memory");

            loop {
                ttl.tick().await;

                self.clean_tx_sender(ttl_duration);
                self.clean_block_listeners(ttl_duration);
                self.clean_block_store(ttl_duration).await;
                let _ = self.tpu_manager.reset_tpu_client().await;
            }
        })
    }
}
