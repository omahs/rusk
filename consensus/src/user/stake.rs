// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

#[derive(Clone, Default, Debug)]
pub struct Stake {
    value: u64,

    pub reward: u64,
    pub counter: u64,
    pub eligible_since: u64,
}

impl Stake {
    pub fn new(value: u64, reward: u64, eligible_since: u64) -> Self {
        Self {
            value,
            reward,
            eligible_since,
            counter: 0,
        }
    }

    pub fn value(&self) -> u64 {
        self.value
    }

    pub fn from_value(value: u64) -> Self {
        Self {
            value,
            ..Default::default()
        }
    }

    pub fn is_eligible(&self, round: u64) -> bool {
        self.eligible_since <= round
    }

    /// Subtract `sub` from the stake without overflowing if the stake is not
    /// enough.
    ///
    /// Return the effective subtracted amount
    pub fn subtract(&mut self, sub: u64) -> u64 {
        if self.value <= sub {
            let sub = self.value;
            self.value = 0;
            sub
        } else {
            self.value -= sub;
            sub
        }
    }
}
