// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

use anyhow::Result;
use clap::{Args, Subcommand};
use e3_config::AppConfig;

mod context;
mod license;
mod lifecycle;
mod tickets;
mod utils;

use context::ChainContext;

#[derive(Debug, Args, Clone, Default)]
pub struct ChainArgs {
    /// Chain name as defined in the enclave config (defaults to the first entry)
    #[arg(long = "chain")]
    pub chain: Option<String>,
}

impl ChainArgs {
    fn selection(&self) -> Option<&str> {
        self.chain.as_deref()
    }
}

#[derive(Subcommand, Debug)]
pub enum CiphernodeCommands {
    /// Manage ENCL license tokens and bonding state
    License {
        #[command(subcommand)]
        command: LicenseCommands,
        #[command(flatten)]
        chain: ChainArgs,
    },
    /// Manage collateral tickets backed by the stable token
    Tickets {
        #[command(subcommand)]
        command: TicketCommands,
        #[command(flatten)]
        chain: ChainArgs,
    },
    /// Register the current operator in the bonding registry
    Register {
        #[command(flatten)]
        chain: ChainArgs,
    },
    /// Request deregistration by providing the IMT proof siblings
    Deregister {
        #[arg(long = "proof", value_delimiter = ',', value_name = "NODE")]
        sibling_nodes: Vec<String>,
        #[command(flatten)]
        chain: ChainArgs,
    },
    /// Force the registry to recompute activation for the node
    Activate {
        #[command(flatten)]
        chain: ChainArgs,
    },
    /// Intentionally deactivate by withdrawing tickets and/or license stake
    Deactivate {
        #[arg(long = "tickets", value_name = "AMOUNT")]
        ticket_amount: Option<String>,
        #[arg(long = "license", value_name = "AMOUNT")]
        license_amount: Option<String>,
        #[command(flatten)]
        chain: ChainArgs,
    },
    /// Display the current on-chain status for this operator
    Status {
        #[command(flatten)]
        chain: ChainArgs,
    },
}

#[derive(Subcommand, Debug)]
pub enum LicenseCommands {
    /// Bond ENCL into the bonding registry
    Bond {
        #[arg(long = "amount")]
        amount: String,
    },
    /// Unbond ENCL (moves stake to the exit queue)
    Unbond {
        #[arg(long = "amount")]
        amount: String,
    },
    /// Claim any unlocked exits
    Claim {
        #[arg(long = "max-ticket")]
        max_ticket: Option<String>,
        #[arg(long = "max-license")]
        max_license: Option<String>,
    },
}

#[derive(Subcommand, Debug)]
pub enum TicketCommands {
    /// Deposit stablecoins to mint tickets
    Buy {
        #[arg(long = "amount")]
        amount: String,
    },
    /// Burn tickets by withdrawing the underlying stablecoin
    Burn {
        #[arg(long = "amount")]
        amount: String,
    },
}

pub async fn execute(command: CiphernodeCommands, config: &AppConfig) -> Result<()> {
    match command {
        CiphernodeCommands::License { chain, command } => {
            let ctx = ChainContext::new(config, chain.selection()).await?;
            license::execute(&ctx, command).await?
        }
        CiphernodeCommands::Tickets { chain, command } => {
            let ctx = ChainContext::new(config, chain.selection()).await?;
            tickets::execute(&ctx, command).await?
        }
        CiphernodeCommands::Register { chain } => {
            let ctx = ChainContext::new(config, chain.selection()).await?;
            lifecycle::register(&ctx).await?
        }
        CiphernodeCommands::Deregister {
            chain,
            sibling_nodes,
        } => {
            let ctx = ChainContext::new(config, chain.selection()).await?;
            lifecycle::deregister(&ctx, sibling_nodes).await?
        }
        CiphernodeCommands::Activate { chain } => {
            let ctx = ChainContext::new(config, chain.selection()).await?;
            lifecycle::activate(&ctx).await?
        }
        CiphernodeCommands::Deactivate {
            chain,
            ticket_amount,
            license_amount,
        } => {
            let ctx = ChainContext::new(config, chain.selection()).await?;
            lifecycle::deactivate(&ctx, ticket_amount, license_amount).await?
        }
        CiphernodeCommands::Status { chain } => {
            let ctx = ChainContext::new(config, chain.selection()).await?;
            lifecycle::status(&ctx).await?
        }
    }

    Ok(())
}
