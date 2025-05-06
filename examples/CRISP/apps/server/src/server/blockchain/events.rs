use alloy::{rpc::types::Log, sol};

use eyre::Result;

use super::handlers::{
    handle_ciphertext_output_published, handle_committee_published, handle_e3,
    handle_input_published, handle_plaintext_output_published,
};
use super::listener::ContractEvent;

sol! {
    #[derive(Debug)]
    event E3Activated(uint256 e3Id, uint256 expiration, bytes committeePublicKey);

    #[derive(Debug)]
    event InputPublished(uint256 indexed e3Id, bytes data, uint256 inputHash, uint256 index);

    #[derive(Debug)]
    event CiphertextOutputPublished(uint256 indexed e3Id, bytes ciphertextOutput);

    #[derive(Debug)]
    event PlaintextOutputPublished(uint256 indexed e3Id, bytes plaintextOutput);

    #[derive(Debug)]
    event CommitteePublished(uint256 indexed e3Id, bytes publicKey);
}

impl ContractEvent for E3Activated {
    fn process(&self, _log: Log) -> Result<()> {
        let event_clone = self.clone();
        tokio::spawn(async move {
            if let Err(e) = handle_e3(event_clone).await {
                eprintln!("Error handling E3 request: {:?}", e);
            }
        });

        Ok(())
    }
}

impl ContractEvent for InputPublished {
    fn process(&self, _log: Log) -> Result<()> {
        let event_clone = self.clone();
        tokio::spawn(async move {
            if let Err(e) = handle_input_published(event_clone).await {
                eprintln!("Error handling input published: {:?}", e);
            }
        });

        Ok(())
    }
}

impl ContractEvent for CiphertextOutputPublished {
    fn process(&self, _log: Log) -> Result<()> {
        let event_clone = self.clone();
        tokio::spawn(async move {
            if let Err(e) = handle_ciphertext_output_published(event_clone).await {
                eprintln!("Error handling ciphertext output published: {:?}", e);
            }
        });

        Ok(())
    }
}

impl ContractEvent for PlaintextOutputPublished {
    fn process(&self, _log: Log) -> Result<()> {
        let event_clone = self.clone();

        tokio::spawn(async move {
            if let Err(e) = handle_plaintext_output_published(event_clone).await {
                eprintln!("Error handling public key published: {:?}", e);
            }
        });

        Ok(())
    }
}

impl ContractEvent for CommitteePublished {
    fn process(&self, _log: Log) -> Result<()> {
        let event_clone = self.clone();
        tokio::spawn(async move {
            if let Err(e) = handle_committee_published(event_clone).await {
                eprintln!("Error handling committee published: {:?}", e);
            }
        });

        Ok(())
    }
}
