#![crate_name = "core"]
#![crate_type = "lib"]
#![warn(missing_docs, unused_imports)]

pub struct Core {
	pub name: String,
}

impl Core {
    fn new(name: String) -> Self {
        Self { name }
    }
}