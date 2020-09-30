// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use crate::gadgets::range::range;
use anyhow::{Error, Result};
use dusk_plonk::constraint_system::ecc::scalar_mul::fixed_base::scalar_mul;
use dusk_plonk::jubjub::{
    AffinePoint, GENERATOR_EXTENDED, GENERATOR_NUMS_EXTENDED,
};
use dusk_plonk::prelude::*;
use plonk_gadgets::AllocatedScalar;

/// XXX: missing fields from stuct as per gitbook. This is for one time encryption pk.
#[derive(Debug, Default, Clone)]
pub struct SendToContractObfuscatedCircuit {
    /// Value within commitment crossover
    pub commitment_crossover_value: Option<BlsScalar>,
    /// Blinder within commitment crossover
    pub commitment_crossover_blinder: Option<BlsScalar>,
    /// Commitment crossover point
    pub commitment_crossover: Option<AffinePoint>,
    /// Value within commitment message
    pub commitment_message_value: Option<BlsScalar>,
    /// Blinder within message commitment 
    pub commitment_message_blinder: Option<BlsScalar>,
    /// Message commitment point 
    pub commitment_message: Option<AffinePoint>,
    /// Returns circuit size
    pub size: usize,
    /// Gives Public Inputs
    pub pi_constructor: Option<Vec<PublicInput>>,
}

impl Circuit<'_> for SendToContractObfuscatedCircuit {
    fn gadget(
        &mut self,
        composer: &mut StandardComposer,
    ) -> Result<Vec<PublicInput>, Error> {
        let mut pi: Vec<PublicInput> = vec![];

        let commitment_crossover_value = self
            .commitment_crossover_value
            .ok_or_else(|| CircuitErrors::CircuitInputsNotFound)?;
        let commitment_crossover_blinder = self
            .commitment_crossover_blinder
            .ok_or_else(|| CircuitErrors::CircuitInputsNotFound)?;
        let commitment_crossover = self
            .commitment_crossover
            .ok_or_else(|| CircuitErrors::CircuitInputsNotFound)?;
        let commitment_message_value = self
            .commitment_message_value
            .ok_or_else(|| CircuitErrors::CircuitInputsNotFound)?;
        let commitment_message_blinder = self
            .commitment_message_blinder
            .ok_or_else(|| CircuitErrors::CircuitInputsNotFound)?;
        let commitment_message = self
            .commitment_message
            .ok_or_else(|| CircuitErrors::CircuitInputsNotFound)?;

        // Create allocated scalars for private inputs
        let allocated_crossover_value =
            AllocatedScalar::allocate(composer, commitment_crossover_value);
        let allocated_crossover_blinder =
            AllocatedScalar::allocate(composer, commitment_crossover_blinder);
        let allocated_message_value =
            AllocatedScalar::allocate(composer, commitment_message_value);
        let allocated_message_blinder =
            AllocatedScalar::allocate(composer, commitment_message_blinder);
        
        // Prove the knowledge of the commitment opening of the commitment of the crossover in the input
        let p1 = scalar_mul(
            composer,
            allocated_crossover_value.var,
            GENERATOR_EXTENDED,
        );
        let p2 = scalar_mul(
            composer,
            allocated_crossover_blinder.var,
            GENERATOR_NUMS_EXTENDED,
        );

        let commitment = p1.point().fast_add(composer, *p2.point());

        // Add PI constraint for the commitment computation check.
        pi.push(PublicInput::AffinePoint(
            commitment_crossover,
            composer.circuit_size(),
            composer.circuit_size() + 1,
        ));

        // Assert computed commitment is equal to publicly inputted affine point
        composer.assert_equal_public_point(commitment, commitment_crossover);

        // Prove that the value of the opening of the commitment of the input is within range
        range(composer, allocated_crossover_value, 64);

        // Prove the knowledge of the commitment opening of the commitment of the crossover in the input
        let p3 = scalar_mul(
            composer,
            allocated_message_value.var,
            GENERATOR_EXTENDED,
        );
        let p4 = scalar_mul(
            composer,
            allocated_message_blinder.var,
            GENERATOR_NUMS_EXTENDED,
        );

        let commitment2 = p3.point().fast_add(composer, *p4.point());

        // Add PI constraint for the commitment computation check.
        pi.push(PublicInput::AffinePoint(
            commitment_message,
            composer.circuit_size(),
            composer.circuit_size() + 1,
        ));

        // Assert computed commitment is equal to publicly inputted affine point
        composer.assert_equal_public_point(commitment2, commitment_message);

        // Prove that the value of the opening of the commitment of the input is within range
        range(composer, allocated_message_value, 64);

        composer.assert_equal(allocated_crossover_value.var, allocated_message_value.var);

        self.size = composer.circuit_size();
        Ok(pi)
        
    }

    fn compile(
        &mut self,
        pub_params: &PublicParameters,
    ) -> Result<(ProverKey, VerifierKey, usize), Error> {
        // Setup PublicParams
        let (ck, _) = pub_params.trim(1 << 13)?;
        // Generate & save `ProverKey` with some random values.
        let mut prover = Prover::new(b"TestCircuit");
        // Set size & Pi builder
        self.pi_constructor = Some(self.gadget(prover.mut_cs())?);
        prover.preprocess(&ck)?;

        // Generate & save `VerifierKey` with some random values.
        let mut verifier = Verifier::new(b"TestCircuit");
        self.gadget(verifier.mut_cs())?;
        verifier.preprocess(&ck)?;
        Ok((
            prover
                .prover_key
                .expect("Unexpected error. Missing VerifierKey in compilation")
                .clone(),
            verifier
                .verifier_key
                .expect("Unexpected error. Missing VerifierKey in compilation"),
            self.circuit_size(),
        ))
    }

    fn build_pi(&self, pub_inputs: &[PublicInput]) -> Result<Vec<BlsScalar>> {
        let mut pi = vec![BlsScalar::zero(); self.size];
        self.pi_constructor
            .as_ref()
            .ok_or(CircuitErrors::CircuitInputsNotFound)?
            .iter()
            .enumerate()
            .for_each(|(idx, pi_constr)| {
                match pi_constr {
                    PublicInput::BlsScalar(_, pos) => {
                        pi[*pos] = pub_inputs[idx].value()[0]
                    }
                    PublicInput::JubJubScalar(_, pos) => {
                        pi[*pos] = pub_inputs[idx].value()[0]
                    }
                    PublicInput::AffinePoint(_, pos_x, pos_y) => {
                        let (coord_x, coord_y) = (
                            pub_inputs[idx].value()[0],
                            pub_inputs[idx].value()[1],
                        );
                        pi[*pos_x] = -coord_x;
                        pi[*pos_y] = -coord_y;
                    }
                };
            });
        Ok(pi)
    }

    fn circuit_size(&self) -> usize {
        self.size
    }

    fn gen_proof(
        &mut self,
        pub_params: &PublicParameters,
        prover_key: &ProverKey,
        transcript_initialisation: &'static [u8],
    ) -> Result<Proof> {
        let (ck, _) = pub_params.trim(1 << 13)?;
        // New Prover instance
        let mut prover = Prover::new(transcript_initialisation);
        // Fill witnesses for Prover
        self.gadget(prover.mut_cs())?;
        // Add ProverKey to Prover
        prover.prover_key = Some(prover_key.clone());
        prover.prove(&ck)
    }

    fn verify_proof(
        &mut self,
        pub_params: &PublicParameters,
        verifier_key: &VerifierKey,
        transcript_initialisation: &'static [u8],
        proof: &Proof,
        pub_inputs: &[PublicInput],
    ) -> Result<(), Error> {
        let (_, vk) = pub_params.trim(1 << 13)?;
        // New Verifier instance
        let mut verifier = Verifier::new(transcript_initialisation);
        // Fill witnesses for Verifier
        self.gadget(verifier.mut_cs())?;
        verifier.verifier_key = Some(*verifier_key);
        verifier.verify(proof, &vk, &self.build_pi(pub_inputs)?)
    }
    
}


#[cfg(test)]
mod tests {
    use super::*;
    use anyhow::Result;
    use dusk_plonk::commitment_scheme::kzg10::PublicParameters;

    #[test]
    fn test_send_to_contract_obfuscated() -> Result<()> {
        // Define and create commitment crossover values
        let commitment_crossover_value = JubJubScalar::from(300 as u64);
        let commitment_crossover_blinder = JubJubScalar::from(100 as u64);
        let commitment_crossover = AffinePoint::from(
            &(GENERATOR_EXTENDED * commitment_crossover_value)
                + &(GENERATOR_NUMS_EXTENDED * commitment_crossover_blinder),
        );

        let commitment_message_value = JubJubScalar::from(300 as u64);
        let commitment_message_blinder = JubJubScalar::from(200 as u64);
        let commitment_message = AffinePoint::from(
            &(GENERATOR_EXTENDED * commitment_message_value)
                + &(GENERATOR_NUMS_EXTENDED * commitment_message_blinder),
        );

        // Build circuit structure
        let mut circuit = SendToContractObfuscatedCircuit {
            commitment_crossover_value: Some(commitment_crossover_value.into()),
            commitment_crossover_blinder: Some(commitment_crossover_blinder.into()),
            commitment_crossover: Some(commitment_crossover),
            commitment_message_value: Some(commitment_message_value.into()),
            commitment_message_blinder: Some(commitment_message_blinder.into()),
            commitment_message: Some(commitment_message),
            size: 0,
            pi_constructor: None,
        };

        // Generate Composer & Public Parameters
        let pub_params =
            PublicParameters::setup(1 << 14, &mut rand::thread_rng())?;
        let (pk, vk, _) = circuit.compile(&pub_params)?;
        let proof = circuit.gen_proof(&pub_params, &pk, b"ObfuscatedSend")?;

        let pi = vec![
            PublicInput::AffinePoint(commitment_crossover, 0, 0),
            PublicInput::AffinePoint(commitment_message, 0, 0)
        ];

        circuit.verify_proof(&pub_params, &vk, b"ObfuscatedSend", &proof, &pi)
    }

    #[test]
    fn test_send_to_contract_obfuscated_wrong_value() -> Result<()> {
        // Define and create commitment crossover values
        let commitment_crossover_value = JubJubScalar::from(200 as u64);
        let commitment_crossover_blinder = JubJubScalar::from(100 as u64);
        let commitment_crossover = AffinePoint::from(
            &(GENERATOR_EXTENDED * commitment_crossover_value)
                + &(GENERATOR_NUMS_EXTENDED * commitment_crossover_blinder),
        );

        let commitment_message_value = JubJubScalar::from(300 as u64);
        let commitment_message_blinder = JubJubScalar::from(200 as u64);
        let commitment_message = AffinePoint::from(
            &(GENERATOR_EXTENDED * commitment_message_value)
                + &(GENERATOR_NUMS_EXTENDED * commitment_message_blinder),
        );

        // Build circuit structure
        let mut circuit = SendToContractObfuscatedCircuit {
            commitment_crossover_value: Some(commitment_crossover_value.into()),
            commitment_crossover_blinder: Some(commitment_crossover_blinder.into()),
            commitment_crossover: Some(commitment_crossover),
            commitment_message_value: Some(commitment_message_value.into()),
            commitment_message_blinder: Some(commitment_message_blinder.into()),
            commitment_message: Some(commitment_message),
            size: 0,
            pi_constructor: None,
        };

        // Generate Composer & Public Parameters
        let pub_params =
            PublicParameters::setup(1 << 14, &mut rand::thread_rng())?;
        let (pk, vk, _) = circuit.compile(&pub_params)?;
        let proof = circuit.gen_proof(&pub_params, &pk, b"ObfuscatedSend")?;

        let pi = vec![
            PublicInput::AffinePoint(commitment_crossover, 0, 0),
            PublicInput::AffinePoint(commitment_message, 0, 0)
        ];

        assert!(circuit
            .verify_proof(&pub_params, &vk, b"ObfuscatedSend", &proof, &pi)
            .is_err());
        Ok(())
    }
}