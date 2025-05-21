use super::events::{
    CiphertextOutputPublished, CommitteePublished, E3Activated, InputPublished,
    PlaintextOutputPublished,
};
use crate::server::{
    config::CONFIG,
    database::{generate_emoji, get_e3, update_e3_status, GLOBAL_DB},
    models::{CurrentRound, E3},
};
use chrono::Utc;
use compute_provider::FHEInputs;
use enclave_sdk::evm::contracts::{EnclaveContract, EnclaveRead, EnclaveWrite};
use enclave_sdk::indexer::DataStore;
use log::info;
use std::error::Error;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tokio::time::{sleep_until, Instant};
use voting_host::run_compute;

type Result<T> = std::result::Result<T, Box<dyn Error + Send + Sync>>;

pub async fn handle_e3(e3_activated: E3Activated) -> Result<()> {
    let e3_id = e3_activated.e3Id.to::<u64>();
    info!("Handling E3 request with id {}", e3_id);

    // Fetch E3 from the contract
    let contract = EnclaveContract::new(
        &CONFIG.http_rpc_url,
        &CONFIG.private_key,
        &CONFIG.enclave_address,
    )
    .await?;

    let e3 = contract.get_e3(e3_activated.e3Id).await?;
    info!("Fetched E3 from the contract.");
    info!("E3: {:?}", e3);

    let start_time = Utc::now().timestamp() as u64;
    let expiration = e3_activated.expiration.to::<u64>();

    let e3_obj = E3 {
        // Identifiers
        id: e3_id,
        chain_id: CONFIG.chain_id, // Hardcoded for testing
        enclave_address: CONFIG.enclave_address.clone(),

        // Status-related
        status: "Active".to_string(),
        has_voted: vec![],
        vote_count: 0,
        votes_option_1: 0,
        votes_option_2: 0,

        // Timing-related
        start_time,
        block_start: e3.requestBlock.to::<u64>(),
        duration: e3.duration.to::<u64>(),
        expiration,

        // Parameters
        e3_params: e3.e3ProgramParams.to_vec(),
        committee_public_key: e3_activated.committeePublicKey.to_vec(),

        // Outputs
        ciphertext_output: vec![],
        plaintext_output: vec![],

        // Ciphertext Inputs
        ciphertext_inputs: vec![],

        // Emojis
        emojis: generate_emoji(),
    };

    // Save E3 to the database
    let key = format!("e3:{}", e3_id);
    GLOBAL_DB.insert(&key, &e3_obj).await?;

    // Set Current Round
    let current_round = CurrentRound { id: e3_id };
    GLOBAL_DB.insert("e3:current_round", &current_round).await?;

    let expiration = Instant::now()
        + (UNIX_EPOCH + Duration::from_secs(expiration))
            .duration_since(SystemTime::now())
            .unwrap_or_else(|_| Duration::ZERO);

    info!("Expiration: {:?}", expiration);

    // Sleep till the E3 expires (instantly if in the past)
    sleep_until(expiration).await;

    // Get All Encrypted Votes
    let (mut e3, _) = get_e3(e3_id).await.unwrap();
    update_e3_status(e3_id, "Expired".to_string()).await?;

    if e3.vote_count > 0 {
        info!("E3 FROM DB");
        info!("Vote Count: {:?}", e3.vote_count);

        let fhe_inputs = FHEInputs {
            params: e3.e3_params,
            ciphertexts: e3.ciphertext_inputs,
        };
        info!("Starting computation for E3: {}", e3_id);
        update_e3_status(e3_id, "Computing".to_string()).await?;
        // Call Compute Provider in a separate thread
        let (risc0_output, ciphertext) =
            tokio::task::spawn_blocking(move || run_compute(fhe_inputs).unwrap())
                .await
                .unwrap();

        info!("Computation completed for E3: {}", e3_id);
        info!("RISC0 Output: {:?}", risc0_output);
        update_e3_status(e3_id, "PublishingCiphertext".to_string()).await?;
        // Params will be encoded on chain to create the journal
        let tx = contract
            .publish_ciphertext_output(
                e3_activated.e3Id,
                ciphertext.into(),
                risc0_output.seal.into(),
            )
            .await?;

        info!(
            "CiphertextOutputPublished event published with tx: {:?}",
            tx.transaction_hash
        );
    } else {
        info!("E3 has no votes to decrypt. Setting status to Finished.");
        e3.status = "Finished".to_string();

        GLOBAL_DB.insert(&key, &e3).await?;
    }
    info!("E3 request handled successfully.");
    Ok(())
}

pub async fn handle_input_published(input: InputPublished) -> Result<()> {
    info!("Handling VoteCast event...");

    let e3_id = input.e3Id.to::<u64>();
    let (mut e3, key) = get_e3(e3_id).await?;

    e3.ciphertext_inputs
        .push((input.data.to_vec(), input.index.to::<u64>()));
    e3.vote_count += 1;

    GLOBAL_DB.insert(&key, &e3).await?;

    info!("Saved Input with Hash: {:?}", input.inputHash);
    Ok(())
}

pub async fn handle_ciphertext_output_published(
    ciphertext_output: CiphertextOutputPublished,
) -> Result<()> {
    info!("Handling CiphertextOutputPublished event...");

    let e3_id = ciphertext_output.e3Id.to::<u64>();
    let (mut e3, key) = get_e3(e3_id).await?;

    e3.ciphertext_output = ciphertext_output.ciphertextOutput.to_vec();
    e3.status = "CiphertextPublished".to_string();

    GLOBAL_DB.insert(&key, &e3).await?;

    info!("CiphertextOutputPublished event handled.");
    Ok(())
}

pub async fn handle_plaintext_output_published(
    plaintext_output: PlaintextOutputPublished,
) -> Result<()> {
    info!("Handling PlaintextOutputPublished event...");
    let e3_id = plaintext_output.e3Id.to::<u64>();
    let (mut e3, key) = get_e3(e3_id).await?;

    let decoded: Vec<u64> = bincode::deserialize(&plaintext_output.plaintextOutput.to_vec())?;
    e3.plaintext_output = plaintext_output.plaintextOutput.to_vec();
    e3.votes_option_2 = decoded[0];
    e3.votes_option_1 = e3.vote_count - e3.votes_option_2;
    e3.status = "Finished".to_string();

    info!("Vote Count: {:?}", e3.vote_count);
    info!("Votes Option 1: {:?}", e3.votes_option_1);
    info!("Votes Option 2: {:?}", e3.votes_option_2);

    GLOBAL_DB.insert(&key, &e3).await?;

    info!("PlaintextOutputPublished event handled.");
    Ok(())
}

pub async fn handle_committee_published(committee_published: CommitteePublished) -> Result<()> {
    info!(
        "Handling CommitteePublished event for E3: {}",
        committee_published.e3Id
    );
    let contract = EnclaveContract::new(
        &CONFIG.http_rpc_url,
        &CONFIG.private_key,
        &CONFIG.enclave_address,
    )
    .await?;

    let tx = contract
        .activate(committee_published.e3Id, committee_published.publicKey)
        .await?;
    info!("E3 activated with tx: {:?}", tx.transaction_hash);
    Ok(())
}
