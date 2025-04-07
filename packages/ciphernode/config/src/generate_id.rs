use petname::{Generator, Petnames};
use rand::thread_rng;

/// Generate a unique compilation ID for the build based on the time of compilation
pub fn generate_id() -> String {
    let mut rng = thread_rng();
    format!(
        "n:{}",
        Petnames::small()
            .generate(&mut rng, 3, "_")
            .unwrap_or("default-name".to_owned())
    )
}
