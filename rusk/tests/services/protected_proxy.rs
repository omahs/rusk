// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use std::collections::HashMap;
use std::path::Path;
use std::sync::{Arc, RwLock};

use dusk_wallet_core::{self as wallet};
use rand::prelude::*;
use rand::rngs::StdRng;
use rusk::{Result, Rusk};
use rusk_abi::{ContractId, TRANSFER_CONTRACT};
use tempfile::tempdir;

use crate::common::logger;
use crate::common::state::{generator_procedure, new_state};
use crate::common::wallet::{TestProverClient, TestStateClient, TestStore};

const BLOCK_HEIGHT: u64 = 1;
const BLOCK_GAS_LIMIT: u64 = 1_000_000_000_000;

const GAS_LIMIT: u64 = 200_000_000;

fn initial_state<P: AsRef<Path>>(dir: P) -> Result<Rusk> {
    let snapshot = toml::from_str(include_str!("../config/protected.toml"))
        .expect("Cannot deserialize config");

    new_state(dir, &snapshot)
}

const SENDER_INDEX: u64 = 0;

const TRANSFER_PROXY_PROTECTED_METHODS: &'static [&'static str] = &[
    "spend_and_execute",
    "refund",
    "push_note",
    "update_root",
    "add_module_balance",
];

fn test_protected_proxy_method(
    contract: ContractId,
    protected_method: &str,
    rusk: &Rusk,
    wallet: &wallet::Wallet<TestStore, TestStateClient, TestProverClient>,
) {
    let mut rng = StdRng::seed_from_u64(0xcafe);

    let refund = wallet
        .public_spend_key(SENDER_INDEX)
        .expect("Getting a public spend key should succeed");

    let tx = wallet
        .execute(
            &mut rng,
            contract.to_bytes().into(),
            String::from(protected_method),
            (),
            SENDER_INDEX,
            &refund,
            GAS_LIMIT,
            1,
        )
        .expect("Making the transaction should succeed");

    let spent_transactions = generator_procedure(
        rusk,
        &[tx],
        BLOCK_HEIGHT,
        BLOCK_GAS_LIMIT,
        vec![],
        None,
    )
    .expect("generator procedure should succeed");
    assert!(spent_transactions
        .first()
        .expect("Transaction should exist")
        .err
        .is_some());
}

#[tokio::test(flavor = "multi_thread")]
pub async fn protected_proxy_methods() -> Result<()> {
    logger();

    let tmp = tempdir().expect("Should be able to create temporary directory");
    let rusk = initial_state(&tmp)?;

    let cache = Arc::new(RwLock::new(HashMap::new()));

    let wallet = wallet::Wallet::new(
        TestStore,
        TestStateClient {
            rusk: rusk.clone(),
            cache,
        },
        TestProverClient::default(),
    );

    for method in TRANSFER_PROXY_PROTECTED_METHODS {
        test_protected_proxy_method(TRANSFER_CONTRACT, *method, &rusk, &wallet);
    }

    Ok(())
}
