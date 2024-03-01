// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

pub mod common;

use dusk_bls12_381_sign::{PublicKey, SecretKey};
use dusk_pki::{PublicSpendKey, SecretSpendKey};
use rand::rngs::StdRng;
use rand::SeedableRng;
use rusk_abi::dusk::dusk;
use rusk_abi::Error;
use rusk_abi::{STAKE_CONTRACT, TRANSFER_CONTRACT};
use stake_contract_types::StakeData;

use crate::common::assert::assert_event;
use crate::common::init::instantiate;

const GENESIS_VALUE: u64 = dusk(1_000_000.0);

#[test]
fn reward_slash() -> Result<(), Error> {
    let rng = &mut StdRng::seed_from_u64(0xfeeb);

    let vm = &mut rusk_abi::new_ephemeral_vm()
        .expect("Creating ephemeral VM should work");

    let ssk = SecretSpendKey::random(rng);
    let psk = PublicSpendKey::from(&ssk);

    let sk = SecretKey::random(rng);
    let pk = PublicKey::from(&sk);

    let mut session = instantiate(rng, vm, &psk, GENESIS_VALUE);

    let reward_amount = dusk(10.0);
    let slash_amount = dusk(5.0);

    let receipt = session.call::<_, ()>(
        STAKE_CONTRACT,
        "reward",
        &(pk, reward_amount),
        u64::MAX,
    )?;
    assert_event(&receipt.events, "reward", &pk, reward_amount);

    let receipt = session.call::<_, ()>(
        STAKE_CONTRACT,
        "slash",
        &(pk, slash_amount),
        u64::MAX,
    )?;
    assert_event(&receipt.events, "slash", &pk, slash_amount);
    Ok(())
}

#[test]
fn stake_hard_slash() -> Result<(), Error> {
    let rng = &mut StdRng::seed_from_u64(0xfeeb);

    let vm = &mut rusk_abi::new_ephemeral_vm()
        .expect("Creating ephemeral VM should work");

    let ssk = SecretSpendKey::random(rng);
    let psk = PublicSpendKey::from(&ssk);

    let sk = SecretKey::random(rng);
    let pk = PublicKey::from(&sk);

    let mut session = instantiate(rng, vm, &psk, GENESIS_VALUE);

    let balance = dusk(14.0);
    let hard_slash_amount = dusk(5.0);
    let block_height = 0;

    let stake_data = StakeData {
        reward: 0,
        amount: Some((balance, block_height)),
        counter: 0,
    };

    session.call::<_, ()>(
        TRANSFER_CONTRACT,
        "add_module_balance",
        &(STAKE_CONTRACT, balance),
        u64::MAX,
    )?;

    session.call::<_, ()>(
        STAKE_CONTRACT,
        "insert_stake",
        &(pk, stake_data),
        u64::MAX,
    )?;

    let receipt = session.call::<_, ()>(
        STAKE_CONTRACT,
        "hard_slash",
        &(pk, hard_slash_amount),
        u64::MAX,
    )?;
    assert_event(&receipt.events, "hard_slash", &pk, hard_slash_amount);

    Ok(())
}
