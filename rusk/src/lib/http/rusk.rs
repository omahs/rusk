// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use super::event::Event;
use super::*;

use dusk_bytes::Serializable;
use node::vm::VMExecution;
use rusk_profile::CRS_17_HASH;
use serde::Serialize;
use std::sync::{mpsc, Arc};
use std::thread;
use tokio::task;

use rusk_abi::ContractId;

use crate::chain::Rusk;

const RUSK_FEEDER_HEADER: &str = "Rusk-Feeder";

#[async_trait]
impl HandleRequest for Rusk {
    async fn handle(
        &self,
        request: &MessageRequest,
    ) -> anyhow::Result<ResponseData> {
        match &request.event.to_route() {
            (Target::Contract(_), ..) => {
                let feeder = request.header(RUSK_FEEDER_HEADER).is_some();
                self.handle_contract_query(&request.event, feeder)
            }
            (Target::Host(_), "rusk", "preverify") => {
                self.handle_preverify(request.event_data())
            }
            (Target::Host(_), "rusk", "provisioners") => {
                self.get_provisioners()
            }
            (Target::Host(_), "rusk", "crs") => self.get_crs(),
            _ => Err(anyhow::anyhow!("Unsupported")),
        }
    }
}

impl Rusk {
    fn handle_contract_query(
        &self,
        event: &Event,
        feeder: bool,
    ) -> anyhow::Result<ResponseData> {
        let contract = event.target.inner();
        let contract_bytes = hex::decode(contract)?;

        let contract_bytes = contract_bytes
            .try_into()
            .map_err(|_| anyhow::anyhow!("Invalid contract bytes"))?;

        if feeder {
            let (sender, receiver) = mpsc::channel();

            let rusk = self.clone();
            let topic = event.topic.clone();
            let arg = event.data.as_bytes().to_vec();

            thread::spawn(move || {
                rusk.feeder_query_raw(
                    ContractId::from_bytes(contract_bytes),
                    topic,
                    arg,
                    sender,
                );
            });
            Ok(ResponseData::new(receiver))
        } else {
            let data = self
                .query_raw(
                    ContractId::from_bytes(contract_bytes),
                    event.topic.clone(),
                    event.data.as_bytes(),
                )
                .map_err(|e| anyhow::anyhow!("{e}"))?;
            Ok(ResponseData::new(data))
        }
    }

    fn handle_preverify(&self, data: &[u8]) -> anyhow::Result<ResponseData> {
        let tx = phoenix_core::Transaction::from_slice(data)
            .map_err(|e| anyhow::anyhow!("Invalid Data {e:?}"))?;
        self.preverify(&tx.into())?;
        Ok(ResponseData::new(DataType::None))
    }

    fn get_provisioners(&self) -> anyhow::Result<ResponseData> {
        let prov: Vec<_> = self
            .provisioners(None)
            .expect("Cannot query state for provisioners")
            .filter_map(|(key, stake)| {
                let key = bs58::encode(key.to_bytes()).into_string();
                let (amount, eligibility) = stake.amount.unwrap_or_default();
                (amount > 0).then_some(Provisioner {
                    amount,
                    eligibility,
                    key,
                })
            })
            .collect();

        Ok(ResponseData::new(serde_json::to_value(prov)?))
    }

    fn get_crs(&self) -> anyhow::Result<ResponseData> {
        let crs = rusk_profile::get_common_reference_string()?;
        Ok(ResponseData::new(crs).with_header("crs-hash", CRS_17_HASH))
    }
}

#[derive(Serialize)]
struct Provisioner {
    key: String,
    amount: u64,
    eligibility: u64,
}
