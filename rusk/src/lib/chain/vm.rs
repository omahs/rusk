// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

mod query;

use dusk_bls12_381_sign::PublicKey;
use std::sync::mpsc;
use tracing::info;

use dusk_bytes::DeserializableSlice;
use dusk_consensus::operations::{CallParams, VerificationOutput};
use dusk_consensus::user::provisioners::Provisioners;
use dusk_consensus::user::stake::Stake;
use node::vm::VMExecution;
use node_data::ledger::{Block, SpentTransaction, Transaction};
use phoenix_core::transaction::StakeData;
use rusk_abi::{ContractData, ContractId, Session, STAKE_CONTRACT};

use super::{Rusk, MINIMUM_STAKE};

impl VMExecution for Rusk {
    fn execute_state_transition<I: Iterator<Item = Transaction>>(
        &self,
        params: &CallParams,
        txs: I,
    ) -> anyhow::Result<(
        Vec<SpentTransaction>,
        Vec<Transaction>,
        VerificationOutput,
    )> {
        info!("Received execute_state_transition request");

        let (txs, discarded_txs, verification_output) = self
            .execute_transactions(
                params.round,
                params.block_gas_limit,
                params.generator_pubkey.inner(),
                txs,
                &params.missed_generators[..],
            )
            .map_err(|inner| {
                anyhow::anyhow!("Cannot execute txs: {inner}!!")
            })?;

        Ok((txs, discarded_txs, verification_output))
    }

    fn verify_state_transition(
        &self,
        blk: &Block,
    ) -> anyhow::Result<VerificationOutput> {
        info!("Received verify_state_transition request");
        let generator = blk.header().generator_bls_pubkey;
        let generator =
            dusk_bls12_381_sign::PublicKey::from_slice(&generator.0)
                .map_err(|e| anyhow::anyhow!("Error in from_slice {e:?}"))?;

        let (_, verification_output) = self
            .verify_transactions(
                blk.header().height,
                blk.header().gas_limit,
                &generator,
                blk.txs(),
                &blk.header().failed_iterations.to_missed_generators()?,
            )
            .map_err(|inner| anyhow::anyhow!("Cannot verify txs: {inner}!!"))?;

        Ok(verification_output)
    }

    fn accept(
        &self,
        blk: &Block,
    ) -> anyhow::Result<(Vec<SpentTransaction>, VerificationOutput)> {
        info!("Received accept request");
        let generator = blk.header().generator_bls_pubkey;
        let generator =
            dusk_bls12_381_sign::PublicKey::from_slice(&generator.0)
                .map_err(|e| anyhow::anyhow!("Error in from_slice {e:?}"))?;

        let (txs, verification_output) = self
            .accept_transactions(
                blk.header().height,
                blk.header().gas_limit,
                generator,
                blk.txs().clone(),
                Some(VerificationOutput {
                    state_root: blk.header().state_hash,
                    event_hash: blk.header().event_hash,
                }),
                &blk.header().failed_iterations.to_missed_generators()?,
            )
            .map_err(|inner| anyhow::anyhow!("Cannot accept txs: {inner}!!"))?;

        Ok((txs, verification_output))
    }

    fn finalize(
        &self,
        blk: &Block,
    ) -> anyhow::Result<(Vec<SpentTransaction>, VerificationOutput)> {
        info!("Received finalize request");
        let generator = blk.header().generator_bls_pubkey;
        let generator =
            dusk_bls12_381_sign::PublicKey::from_slice(&generator.0)
                .map_err(|e| anyhow::anyhow!("Error in from_slice {e:?}"))?;

        let (txs, state_root) = self
            .finalize_transactions(
                blk.header().height,
                blk.header().gas_limit,
                generator,
                blk.txs().clone(),
                Some(VerificationOutput {
                    state_root: blk.header().state_hash,
                    event_hash: blk.header().event_hash,
                }),
                &blk.header().failed_iterations.to_missed_generators()?,
            )
            .map_err(|inner| {
                anyhow::anyhow!("Cannot finalize txs: {inner}!!")
            })?;

        let r = self.migrate(blk.header().height); // we ignore error for the time being
        println!("MIGRATION RESULT={:?}", r);

        Ok((txs, state_root))
    }

    fn migrate(&self, block_height: u64) -> anyhow::Result<()> {
        const MIGRATION_BLOCK: u64 = 3;
        const GAS_LIMIT: u64 = 1000_000_000;
        const OWNER: [u8; 32] = [0u8; 32]; // todo !!! get owner from the old contract
        let new_stake_contract_bytecode =
            include_bytes!("../../assets/stake_contract.wasm");
        if block_height == MIGRATION_BLOCK {
            info!("MIGRATING STAKE CONTRACT");
            let inner = self.inner.lock();
            let current_commit = inner.current_commit;
            let mut session =
                rusk_abi::new_session(&inner.vm, current_commit, block_height)?;
            session = session.migrate(
                STAKE_CONTRACT,
                new_stake_contract_bytecode,
                ContractData::builder(OWNER),
                GAS_LIMIT,
                |new_contract, session| {
                    for (pk, stake_data) in
                        do_get_provisioners(STAKE_CONTRACT, session)?
                    {
                        session.call::<_, ()>(
                            new_contract,
                            "insert_stake",
                            &(pk, stake_data),
                            GAS_LIMIT,
                        )?;
                    }
                    Ok(())
                },
            )?;
            let _root = session.commit()?;
            info!("STAKE CONTRACT MIGRATION FINISHED");
        }
        Ok(())
    }

    fn preverify(&self, tx: &Transaction) -> anyhow::Result<()> {
        info!("Received preverify request");
        let tx = &tx.inner;
        let existing_nullifiers = self
            .existing_nullifiers(&tx.nullifiers)
            .map_err(|e| anyhow::anyhow!("Cannot check nullifiers: {e}"))?;

        if !existing_nullifiers.is_empty() {
            let err = crate::Error::RepeatingNullifiers(existing_nullifiers);
            return Err(anyhow::anyhow!("Invalid tx: {err}"));
        }
        match crate::verifier::verify_proof(tx) {
            Ok(true) => Ok(()),
            Ok(false) => Err(anyhow::anyhow!("Invalid proof")),
            Err(e) => Err(anyhow::anyhow!("Cannot verify the proof: {e}")),
        }
    }

    fn get_provisioners(
        &self,
        base_commit: [u8; 32],
    ) -> anyhow::Result<Provisioners> {
        self.query_provisioners(Some(base_commit))
    }

    fn get_state_root(&self) -> anyhow::Result<[u8; 32]> {
        Ok(self.state_root())
    }

    fn get_finalized_state_root(&self) -> anyhow::Result<[u8; 32]> {
        Ok(self.base_root())
    }

    fn revert(&self, state_hash: [u8; 32]) -> anyhow::Result<[u8; 32]> {
        let state_hash = self
            .revert(state_hash)
            .map_err(|inner| anyhow::anyhow!("Cannot revert: {inner}"))?;

        Ok(state_hash)
    }

    fn revert_to_finalized(&self) -> anyhow::Result<[u8; 32]> {
        let state_hash = self.revert_to_base_root().map_err(|inner| {
            anyhow::anyhow!("Cannot revert to finalized: {inner}")
        })?;

        Ok(state_hash)
    }
}

impl Rusk {
    fn query_provisioners(
        &self,
        base_commit: Option<[u8; 32]>,
    ) -> anyhow::Result<Provisioners> {
        info!("Received get_provisioners request");
        let provisioners = self
            .provisioners(base_commit)
            .map_err(|e| anyhow::anyhow!("Cannot get provisioners {e}"))?
            .filter(|(_, stake)| {
                stake
                    .amount
                    .map(|(amount, _)| amount >= MINIMUM_STAKE)
                    .unwrap_or_default()
            })
            .filter_map(|(key, stake)| {
                stake.amount.map(|(value, eligibility)| {
                    let stake = Stake::new(value, stake.reward, eligibility);
                    let pubkey_bls = node_data::bls::PublicKey::new(key);
                    (pubkey_bls, stake)
                })
            });
        let mut ret = Provisioners::empty();
        for (pubkey_bls, stake) in provisioners {
            ret.add_member_with_stake(pubkey_bls, stake);
        }

        Ok(ret)
    }
}

fn do_get_provisioners(
    contract_id: ContractId,
    session: &mut Session,
) -> anyhow::Result<impl Iterator<Item = (PublicKey, StakeData)>> {
    let (sender, receiver) = mpsc::channel();
    let r = session.feeder_call::<_, ()>(contract_id, "stakes", &(), sender);
    println!("r={:?}", r);
    r?;
    Ok(receiver.into_iter().map(|bytes| {
        rkyv::from_bytes::<(PublicKey, StakeData)>(&bytes)
            .expect("The contract should only return (pk, stake_data) tuples")
    }))
}
