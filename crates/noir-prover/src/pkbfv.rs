// // SPDX-License-Identifier: LGPL-3.0-only
// //
// // This file is provided WITHOUT ANY WARRANTY;
// // without even the implied warranty of MERCHANTABILITY
// // or FITNESS FOR A PARTICULAR PURPOSE.

// use pkbfv::vectors::PkBfvVectors;
// use pkbfv::toml::

// pub struct PkBfvProof {
    
// }

// impl PkBfv {

// }

// #[derive(Debug, Clone, Serialize)]
// pub struct PkBfvInputs {
//     pub pk0: Vec<String>,
//     pub pk1: Vec<String>,
//     pub sk_commitment: String,
// }

// impl PkBfvInputs {
//     pub fn dummy() -> Self {
//         Self {
//             pk0: vec!["0".to_string(); 1024],
//             pk1: vec!["0".to_string(); 1024],
//             sk_commitment: "0x0".to_string(),
//         }
//     }
// }

// #[cfg(test)]
// mod tests {

//     use super::*;
//     #[test]
//     fn test_pk_bfv_inputs_serialization() {
//         let inputs = PkBfvInputs::dummy();
//         let toml = toml::to_string(&inputs).unwrap();
//         assert!(toml.contains("pk0"));
//         assert!(toml.contains("sk_commitment"));
//     }
// }