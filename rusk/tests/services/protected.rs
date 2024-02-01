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
use rusk_abi::{
    ContractId, TRANSFER_CONTRACT, TRANSFER_DATA_CONTRACT,
    TRANSFER_LOGIC_CONTRACT,
};
use tempfile::tempdir;

use crate::common::logger;
use crate::common::state::{generator_procedure, new_state, ExecuteResult};
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

fn verify_protected_method(
    contract: ContractId,
    protected_method: &str,
    rusk: &Rusk,
    wallet: &wallet::Wallet<TestStore, TestStateClient, TestProverClient>,
    should_discard: bool,
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

    let expected = if should_discard {
        Some(ExecuteResult {
            discarded: 1,
            executed: 0,
        })
    } else {
        None
    };

    let spent_transactions = generator_procedure(
        rusk,
        &[tx],
        BLOCK_HEIGHT,
        BLOCK_GAS_LIMIT,
        vec![],
        expected,
    )
    .expect("generator procedure should succeed");

    if !should_discard {
        assert!(spent_transactions
            .first()
            .expect("Transaction should exist")
            .err
            .is_some());
    }
}

pub async fn test_protected_method(
    method: &str,
    contract: ContractId,
    should_discard: bool,
) -> Result<()> {
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

    verify_protected_method(contract, method, &rusk, &wallet, should_discard);

    Ok(())
}

macro_rules! protected_methods_tests {
    ($($name:ident: $value:expr,)*) => {
    $(
        #[tokio::test(flavor = "multi_thread")]
        pub async fn $name() -> Result<()> {
            let (method, contract, should_discard) = $value;
            test_protected_method(method, contract, should_discard).await
        }
    )*
    }
}

const SHOULD_DISCARD: bool = true;
const SHOULD_NOT_DISCARD: bool = false; // meaning: should err

protected_methods_tests! {
    test_protected_method_00: ("root", TRANSFER_DATA_CONTRACT, SHOULD_DISCARD),
    test_protected_method_01: ("num_notes", TRANSFER_DATA_CONTRACT, SHOULD_DISCARD),
    test_protected_method_02: ("module_balance", TRANSFER_DATA_CONTRACT, SHOULD_DISCARD),
    test_protected_method_03: ("message", TRANSFER_DATA_CONTRACT, SHOULD_DISCARD),
    test_protected_method_04: ("opening", TRANSFER_DATA_CONTRACT, SHOULD_DISCARD),
    test_protected_method_05: ("existing_nullifiers", TRANSFER_DATA_CONTRACT, SHOULD_DISCARD),
    test_protected_method_06: ("any_nullifier_exists", TRANSFER_DATA_CONTRACT, SHOULD_DISCARD),
    test_protected_method_07: ("extend_nullifiers", TRANSFER_DATA_CONTRACT, SHOULD_DISCARD),
    test_protected_method_08: ("take_message_from_address_key", TRANSFER_DATA_CONTRACT, SHOULD_DISCARD),
    test_protected_method_09: ("root_exists", TRANSFER_DATA_CONTRACT, SHOULD_DISCARD),
    test_protected_method_10: ("push_message", TRANSFER_DATA_CONTRACT, SHOULD_DISCARD),
    test_protected_method_11: ("take_crossover", TRANSFER_DATA_CONTRACT, SHOULD_DISCARD),
    test_protected_method_12: ("set_crossover", TRANSFER_DATA_CONTRACT, SHOULD_DISCARD),
    test_protected_method_13: ("get_crossover", TRANSFER_DATA_CONTRACT, SHOULD_DISCARD),
    test_protected_method_14: ("extend_notes", TRANSFER_DATA_CONTRACT, SHOULD_DISCARD),
    test_protected_method_15: ("sub_balance", TRANSFER_DATA_CONTRACT, SHOULD_DISCARD),
    test_protected_method_16: ("leaves_from_height", TRANSFER_DATA_CONTRACT, SHOULD_DISCARD),
    test_protected_method_17: ("leaves_from_pos", TRANSFER_DATA_CONTRACT, SHOULD_DISCARD),
    test_protected_method_18: ("push_note", TRANSFER_DATA_CONTRACT, SHOULD_DISCARD),
    test_protected_method_19: ("get_note", TRANSFER_DATA_CONTRACT, SHOULD_DISCARD),
    test_protected_method_20: ("update_root", TRANSFER_DATA_CONTRACT, SHOULD_DISCARD),
    test_protected_method_21: ("add_module_balance", TRANSFER_DATA_CONTRACT, SHOULD_DISCARD),
    test_protected_method_22: ("get_module_balance", TRANSFER_DATA_CONTRACT, SHOULD_DISCARD),

    test_protected_method_30: ("mint", TRANSFER_LOGIC_CONTRACT, SHOULD_DISCARD),
    test_protected_method_31: ("stct", TRANSFER_LOGIC_CONTRACT, SHOULD_DISCARD),
    test_protected_method_32: ("wfct", TRANSFER_LOGIC_CONTRACT, SHOULD_DISCARD),
    test_protected_method_33: ("wfct_raw", TRANSFER_LOGIC_CONTRACT, SHOULD_DISCARD),
    test_protected_method_34: ("stco", TRANSFER_LOGIC_CONTRACT, SHOULD_DISCARD),
    test_protected_method_35: ("wfco", TRANSFER_LOGIC_CONTRACT, SHOULD_DISCARD),
    test_protected_method_36: ("wfco_raw", TRANSFER_LOGIC_CONTRACT, SHOULD_DISCARD),
    test_protected_method_37: ("wfctc", TRANSFER_LOGIC_CONTRACT, SHOULD_DISCARD),
    test_protected_method_38: ("root", TRANSFER_LOGIC_CONTRACT, SHOULD_DISCARD),
    test_protected_method_39: ("num_notes", TRANSFER_LOGIC_CONTRACT, SHOULD_DISCARD),
    test_protected_method_40: ("module_balance", TRANSFER_LOGIC_CONTRACT, SHOULD_DISCARD),
    test_protected_method_41: ("message", TRANSFER_LOGIC_CONTRACT, SHOULD_DISCARD),
    test_protected_method_42: ("opening", TRANSFER_LOGIC_CONTRACT, SHOULD_DISCARD),
    test_protected_method_43: ("existing_nullifiers", TRANSFER_LOGIC_CONTRACT, SHOULD_DISCARD),
    test_protected_method_44: ("leaves_from_height", TRANSFER_LOGIC_CONTRACT, SHOULD_DISCARD),
    test_protected_method_45: ("leaves_from_pos", TRANSFER_LOGIC_CONTRACT, SHOULD_DISCARD),
    test_protected_method_46: ("spend_and_execute", TRANSFER_LOGIC_CONTRACT, SHOULD_DISCARD),
    test_protected_method_47: ("refund", TRANSFER_LOGIC_CONTRACT, SHOULD_DISCARD),
    test_protected_method_48: ("push_note", TRANSFER_LOGIC_CONTRACT, SHOULD_DISCARD),
    test_protected_method_49: ("update_root", TRANSFER_LOGIC_CONTRACT, SHOULD_DISCARD),
    test_protected_method_50: ("add_module_balance", TRANSFER_LOGIC_CONTRACT, SHOULD_DISCARD),
    test_protected_method_51: ("sub_module_balance", TRANSFER_LOGIC_CONTRACT, SHOULD_DISCARD),

    test_protected_method_60: ("spend_and_execute", TRANSFER_CONTRACT, SHOULD_NOT_DISCARD),
    test_protected_method_61: ("refund", TRANSFER_CONTRACT, SHOULD_NOT_DISCARD),
    test_protected_method_62: ("push_note", TRANSFER_CONTRACT, SHOULD_NOT_DISCARD),
    test_protected_method_63: ("update_root", TRANSFER_CONTRACT, SHOULD_NOT_DISCARD),
    test_protected_method_64: ("add_module_balance", TRANSFER_CONTRACT, SHOULD_NOT_DISCARD),
}
