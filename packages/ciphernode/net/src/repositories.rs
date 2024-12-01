use config::StoreKeys;
use data::{Repositories, Repository};

use crate::EncryptedKeypair;

pub trait NetRepositoryFactory {
    fn libp2p_key(&self) -> Repository<EncryptedKeypair>;
}

impl NetRepositoryFactory for Repositories {
    fn libp2p_key(&self) -> Repository<EncryptedKeypair> {
        Repository::new(self.store.scope(StoreKeys::libp2p_key()))
    }
}
