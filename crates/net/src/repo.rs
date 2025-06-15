use e3_config::StoreKeys;
use e3_data::{Repositories, Repository};

pub trait NetRepositoryFactory {
    fn libp2p_keypair(&self) -> Repository<Vec<u8>>;
}

impl NetRepositoryFactory for Repositories {
    fn libp2p_keypair(&self) -> Repository<Vec<u8>> {
        Repository::new(self.store.scope(StoreKeys::libp2p_keypair()))
    }
}
