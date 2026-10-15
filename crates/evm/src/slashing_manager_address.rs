// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

//! The SlashingManager that startup chooses for each configured chain.

use crate::{
    chains::{Chain, Chains},
    fetch_slashing_manager,
};
use alloy::primitives::Address;
use anyhow::Result;
use std::collections::HashMap;
use tracing::{error, warn};

/// Delays before the startup retries of `Interfold.slashingManager()`.
const SLASHING_MANAGER_READ_RETRY_DELAYS: [u64; 2] = [1, 3];

/// Decide the SlashingManager for one chain.
///
/// `Interfold.slashingManager()` is authoritative: the contracts call that manager, and accusation
/// votes sign its address as the EIP-712 `verifyingContract`. The configured address applies only
/// when the chain gives no address. `None` means that neither source gives an address.
fn choose_slashing_manager(
    chain: &str,
    configured: Option<Address>,
    on_chain: Option<Address>,
) -> Option<Address> {
    if let (Some(on_chain), Some(configured)) = (on_chain, configured) {
        if on_chain != configured {
            warn!(
                chain = %chain,
                configured = %configured,
                interfold = %on_chain,
                "Configured SlashingManager differs from Interfold.slashingManager(); \
                 using the Interfold value"
            );
        }
    }
    on_chain.or(configured)
}

/// Resolve the SlashingManager address of each configured chain, in `chains` order.
///
/// An enabled chain reads `Interfold.slashingManager()`. A disabled chain has no provider, so
/// only its configured address applies. `choose_slashing_manager` decides the address.
pub async fn resolve_slashing_managers(chains: &Chains) -> Result<Vec<Option<Address>>> {
    let mut slashing_managers = Vec::new();
    for chain in chains.iter() {
        let config = chain.config();
        let configured = config
            .contracts
            .slashing_manager
            .as_ref()
            .map(|contract| contract.address())
            .transpose()?;
        let mut on_chain = None;
        if let Chain::Enabled(chain) = chain {
            let provider = chain.provider.provider();
            let interfold = config.contracts.interfold.address()?;
            // A configured address can be a retired manager, so a transient read failure should
            // not decide the signing domain for the whole run.
            let mut read = fetch_slashing_manager(provider, interfold).await;
            for delay_secs in SLASHING_MANAGER_READ_RETRY_DELAYS {
                if read.is_ok() {
                    break;
                }
                tokio::time::sleep(std::time::Duration::from_secs(delay_secs)).await;
                read = fetch_slashing_manager(provider, interfold).await;
            }
            match read {
                Ok(Some(address)) => on_chain = Some(address),
                Ok(None) => warn!(
                    chain = %config.name,
                    configured = ?configured,
                    "Interfold.slashingManager() is not set; the configured SlashingManager applies"
                ),
                Err(error) => error!(
                    chain = %config.name,
                    configured = ?configured,
                    error = %error,
                    "Could not read Interfold.slashingManager(); the configured SlashingManager \
                     applies until the next restart. Check that it is the live manager"
                ),
            }
        }
        slashing_managers.push(choose_slashing_manager(&config.name, configured, on_chain));
    }
    Ok(slashing_managers)
}

/// The SlashingManager of each chain ID, for accusation votes. `slashing_managers` is in `chains`
/// order.
///
/// Each E3 signs and checks accusation votes with the SlashingManager of its own chain, because
/// that address is the EIP-712 `verifyingContract` of the vote domain.
pub fn slashing_managers_by_chain(
    chains: &Chains,
    slashing_managers: &[Option<Address>],
) -> HashMap<u64, Address> {
    let mut by_chain = HashMap::new();
    for (chain, slashing_manager) in chains.iter().zip(slashing_managers) {
        let Some(slashing_manager) = *slashing_manager else {
            continue;
        };
        match (chain, chain.chain_id()) {
            // The enabled chain's reader and writer use its resolved address, so a disabled entry
            // with the same chain_id must not replace it.
            (Chain::Enabled(_), Some(chain_id)) => {
                by_chain.insert(chain_id, slashing_manager);
            }
            (Chain::Disabled(_), Some(chain_id)) => {
                by_chain.entry(chain_id).or_insert(slashing_manager);
            }
            (_, None) => warn!(
                chain = %chain.config().name,
                "Disabled chain has no chain_id; accusation votes do not use its \
                 SlashingManager"
            ),
        }
    }
    by_chain
}

#[cfg(test)]
mod tests {
    use super::choose_slashing_manager;
    use alloy::primitives::Address;

    #[test]
    fn the_interfold_slashing_manager_wins_over_the_configured_address() {
        let live = Address::repeat_byte(0x75);
        let retired = Address::repeat_byte(0x97);

        assert_eq!(
            choose_slashing_manager("test", Some(retired), Some(live)),
            Some(live)
        );
        assert_eq!(
            choose_slashing_manager("test", None, Some(live)),
            Some(live)
        );
        assert_eq!(
            choose_slashing_manager("test", Some(retired), None),
            Some(retired)
        );
        assert_eq!(choose_slashing_manager("test", None, None), None);
    }
}
