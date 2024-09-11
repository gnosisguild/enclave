#![crate_name = "sortition"]
#![crate_type = "lib"]
// #![warn(missing_docs, unused_imports)]

mod distance;
mod index;

pub use distance::*;
pub use index::*;