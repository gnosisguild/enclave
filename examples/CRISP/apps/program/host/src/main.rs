use compute_provider::FHEInputs;
use fhe::bfv::{
    BfvParameters, BfvParametersBuilder, Ciphertext, Encoding, Plaintext, PublicKey, SecretKey,
};
use fhe_traits::{
    Deserialize, DeserializeParametrized, FheDecoder, FheDecrypter, FheEncoder, FheEncrypter,
    Serialize,
};
use rand::thread_rng;
use std::sync::Arc;
use std::time::Instant;
use voting_host_lib::run_compute;

fn generate_inputs() -> (FHEInputs, SecretKey) {
    let params = create_params();
    let (sk, pk) = generate_keys(&params);
    let inputs: Vec<u64> = (1..=2).collect();
    let incs: Vec<Vec<u8>> = encrypt_inputs(&inputs, &pk, &params)
        .iter()
        .map(|c| c.to_bytes())
        .collect();

    println!("Generated {} encrypted inputs for profiling", incs.len());
    println!("Expected sum: {}", inputs.iter().sum::<u64>());

    (
        FHEInputs {
            ciphertexts: incs.iter().map(|c| (c.to_vec(), 1)).collect(),
            params: params.to_bytes(),
        },
        sk,
    )
}

/// Create BFV parameters for FHE
fn create_params() -> Arc<BfvParameters> {
    BfvParametersBuilder::new()
        .set_degree(2048) 
        .set_plaintext_modulus(1032193)
        .set_moduli(&[0xffffffff00001])
        .build_arc()
        .expect("Failed to build parameters")
}

/// Generate encryption keys
fn generate_keys(params: &Arc<BfvParameters>) -> (SecretKey, PublicKey) {
    let mut rng = thread_rng();
    let sk = SecretKey::random(params, &mut rng);
    let pk = PublicKey::new(&sk, &mut rng);
    (sk, pk)
}

/// Encrypt input values
fn encrypt_inputs(inputs: &[u64], pk: &PublicKey, params: &Arc<BfvParameters>) -> Vec<Ciphertext> {
    let mut rng = thread_rng();
    inputs
        .iter()
        .map(|&input| {
            let pt = Plaintext::try_encode(&[input], Encoding::poly(), params)
                .expect("Failed to encode plaintext");
            pk.try_encrypt(&pt, &mut rng).expect("Failed to encrypt")
        })
        .collect()
}

/// Decrypt a ciphertext using the saved secret key
fn decrypt_result(result_bytes: &[u8], params_bytes: &[u8], sk: &SecretKey) -> u64 {
    let params = Arc::new(
        BfvParameters::try_deserialize(params_bytes).expect("Failed to deserialize parameters"),
    );

    let ciphertext = Ciphertext::from_bytes(result_bytes, &params)
        .expect("Failed to deserialize result ciphertext");

    let plaintext = sk
        .try_decrypt(&ciphertext)
        .expect("Failed to decrypt result");
    let values =
        Vec::<u64>::try_decode(&plaintext, Encoding::poly()).expect("Failed to decode result");

    values[0]
}

fn main() {
    // Generate FHE computation inputs
    println!("Generating FHE inputs for profiling...");
    let start_gen = Instant::now();
    let (fhe_inputs, sk) = generate_inputs();
    let gen_duration = start_gen.elapsed();
    println!("Input generation took: {:?}", gen_duration);

    let start_compute = Instant::now();

    let result_bytes = run_compute(fhe_inputs.clone()).unwrap();

    let compute_duration = start_compute.elapsed();
    println!("Local computation took: {:?}", compute_duration);

    let start_decrypt = Instant::now();
    let decrypted = decrypt_result(&result_bytes.0.bytes, &fhe_inputs.params, &sk);
    let decrypt_duration = start_decrypt.elapsed();
    println!("Decryption took: {:?}", decrypt_duration);
    println!("Decrypted result: {}", decrypted);

    println!("Profiling run finished.");
}
