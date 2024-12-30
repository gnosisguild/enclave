use petname::{Generator, Petnames};
use rand::rngs::StdRng;
use rand::SeedableRng;

static COMPILE_ID: u64 = compile_time::unix!();

/// Generate a unique compilation ID for the build based on the time of compilation
pub fn generate_id() -> String {
    let mut rng = StdRng::seed_from_u64(COMPILE_ID);
    Petnames::small()
        .generate(&mut rng, 3, "_")
        .unwrap_or("default-name".to_owned())
}
