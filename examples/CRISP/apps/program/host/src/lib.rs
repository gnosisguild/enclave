use anyhow::{Error, Result};
use bincode::serialize;
use compute_provider::{ComputeInput, ComputeManager, ComputeProvider, ComputeResult, FHEInputs};
use methods::VOTING_ELF;
use risc0_ethereum_contracts::groth16;
use risc0_zkvm::{default_prover, ExecutorEnv, ProverOpts, VerifierContext};
use serde::{Deserialize, Serialize};
use std::time::Instant;
use voting_core::fhe_processor;

fn encode_input(input: &[u8]) -> Result<Vec<u8>, Error> {
    Ok(bytemuck::pod_collect_to_vec(&risc0_zkvm::serde::to_vec(
        input,
    )?))
}

pub struct Risc0Provider;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Risc0Output {
    pub result: ComputeResult,
    pub bytes: Vec<u8>,
    pub seal: Vec<u8>,
}

impl ComputeProvider for Risc0Provider {
    type Output = Risc0Output;

    fn prove(&self, input: &ComputeInput) -> Self::Output {
        let encoded_input = encode_input(&serialize(input).unwrap()).unwrap();
        let env = ExecutorEnv::builder()
            .write_slice(&encoded_input)
            .build()
            .unwrap();

        let receipt = default_prover()
            .prove_with_ctx(
                env,
                &VerifierContext::default(),
                VOTING_ELF,
                &ProverOpts::groth16(),
            )
            .unwrap()
            .receipt;

        let decoded_journal = receipt.journal.decode().unwrap();

        let seal = groth16::encode(receipt.inner.groth16().unwrap().seal.clone()).unwrap();

        Risc0Output {
            result: decoded_journal,
            bytes: receipt.journal.bytes.clone(),
            seal,
        }
    }
}

pub fn run_compute(params: FHEInputs) -> Result<(Risc0Output, Vec<u8>)> {
    let risc0_provider = Risc0Provider;

    let mut provider = ComputeManager::new(risc0_provider, params, fhe_processor, false, None);

    // Start timer
    let start_time = Instant::now();

    let output = provider.start();

    // Capture end time and calculate the duration
    let elapsed_time = start_time.elapsed();

    // Convert the elapsed time to minutes and seconds
    let minutes = elapsed_time.as_secs() / 60;
    let seconds = elapsed_time.as_secs() % 60;

    println!(
        "Prove function execution time: {} minutes and {} seconds",
        minutes, seconds
    );

    Ok(output)
}
