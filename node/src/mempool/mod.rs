// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use crate::{database, LongLivedService, Message, Network};
use async_trait::async_trait;
use node_data::message::AsyncQueue;
use node_data::message::Topics;
use std::sync::Arc;
use tokio::sync::RwLock;

const TOPICS: &[u8] = &[Topics::Tx as u8];

#[derive(Default)]
pub struct MempoolSrv {
    inbound: AsyncQueue<Message>,
}

pub struct TxFilter {}
impl crate::Filter for TxFilter {
    fn filter(&mut self, msg: &Message) -> anyhow::Result<()> {
        // TODO: Ensure transaction does not exist in the mempool state
        // TODO: Ensure transaction does not exist in blockchain
        // TODO: Check  Nullifier
        Ok(())
    }
}

#[async_trait]
impl<N: Network, DB: database::DB> LongLivedService<N, DB> for MempoolSrv {
    async fn execute(
        &mut self,
        network: Arc<RwLock<N>>,
        db: Arc<RwLock<DB>>,
    ) -> anyhow::Result<usize> {
        LongLivedService::<N, DB>::add_routes(
            self,
            TOPICS,
            self.inbound.clone(),
            &network,
        )
        .await?;

        // Add a filter that will discard any transactions invalid to the actual
        // mempool, blockchain state.
        LongLivedService::<N, DB>::add_filter(
            self,
            Topics::Tx.into(),
            Box::new(TxFilter {}),
            &network,
        )
        .await?;

        loop {
            if let Ok(msg) = self.inbound.recv().await {
                match msg.topic() {
                    Topics::Tx => {
                        if self.handle_tx(&msg).is_ok() {
                            network.read().await.broadcast(&msg).await;
                        }
                    }
                    _ => todo!(),
                };
            }
        }
    }

    /// Returns service name.
    fn name(&self) -> &'static str {
        "mempool"
    }
}

impl MempoolSrv {
    fn handle_tx(&mut self, msg: &Message) -> anyhow::Result<()> {
        // TODO: Preverify

        // TODO: Put in mempool storage
        Ok(())
    }
}
