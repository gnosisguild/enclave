use std::sync::Arc;

/// Reference count bytes so event can be cloned and shared between threads
pub type Bytes = Arc<Vec<u8>>;
