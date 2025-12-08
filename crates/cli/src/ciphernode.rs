// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

use std::str::FromStr;

use alloy::{
    primitives::{Address, U256},
    providers::WalletProvider,
    sol,
};
use anyhow::{anyhow, bail, Context, Result};
use clap::{Args, Subcommand};
use e3_config::chain_config::ChainConfig;
use e3_config::AppConfig;
use e3_crypto::Cipher;
use e3_entrypoint::helpers::datastore::get_repositories;
use e3_evm::{
    helpers::{load_signer_from_repository, ConcreteWriteProvider, EthProvider, ProviderConfig},
    EthPrivateKeyRepositoryFactory,
};
mod bonding_registry_contract {
    use super::sol;

    sol!(
        #[sol(rpc)]
        BondingRegistryContract,
        "../../packages/enclave-contracts/artifacts/contracts/registry/BondingRegistry.sol/BondingRegistry.json"
    );
}

mod enclave_token_contract {
    use super::sol;

    sol!(
        #[sol(rpc)]
        EnclaveTokenContract,
        "../../packages/enclave-contracts/artifacts/contracts/token/EnclaveToken.sol/EnclaveToken.json"
    );
}

mod enclave_ticket_token_contract {
    use super::sol;

    sol!(
        #[sol(rpc)]
        EnclaveTicketTokenContract,
        "../../packages/enclave-contracts/artifacts/contracts/token/EnclaveTicketToken.sol/EnclaveTicketToken.json"
    );
}

mod erc20_metadata_interface {
    use super::sol;

    sol!(
        #[sol(rpc)]
        interface IERC20Metadata {
            function balanceOf(address account) external view returns (uint256);
            function allowance(address owner, address spender) external view returns (uint256);
            function approve(address spender, uint256 amount) external returns (bool);
            function decimals() external view returns (uint8);
            function symbol() external view returns (string memory);
            function name() external view returns (string memory);
        }
    );
}

use bonding_registry_contract::BondingRegistryContract;
use enclave_ticket_token_contract::EnclaveTicketTokenContract;
use enclave_token_contract::EnclaveTokenContract;
use erc20_metadata_interface::IERC20Metadata;

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
    /// Mint ENCL tokens using the faucet/minter role (dev tooling)
    Acquire {
        #[arg(long = "amount")]
        amount: String,
        #[arg(long = "allocation", default_value = "CLI allocation")]
        allocation: String,
    },
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

pub async fn execute(command: CiphernodeCommands, config: AppConfig) -> Result<()> {
    match command {
        CiphernodeCommands::License { chain, command } => {
            let ctx = ChainContext::new(&config, chain.selection()).await?;
            handle_license(&ctx, command).await?
        }
        CiphernodeCommands::Tickets { chain, command } => {
            let ctx = ChainContext::new(&config, chain.selection()).await?;
            handle_tickets(&ctx, command).await?
        }
        CiphernodeCommands::Register { chain } => {
            let ctx = ChainContext::new(&config, chain.selection()).await?;
            register_operator(&ctx).await?
        }
        CiphernodeCommands::Deregister {
            chain,
            sibling_nodes,
        } => {
            let ctx = ChainContext::new(&config, chain.selection()).await?;
            deregister_operator(&ctx, sibling_nodes).await?
        }
        CiphernodeCommands::Activate { chain } => {
            let ctx = ChainContext::new(&config, chain.selection()).await?;
            enforce_activation(&ctx).await?
        }
        CiphernodeCommands::Deactivate {
            chain,
            ticket_amount,
            license_amount,
        } => {
            let ctx = ChainContext::new(&config, chain.selection()).await?;
            deactivate_operator(&ctx, ticket_amount, license_amount).await?
        }
        CiphernodeCommands::Status { chain } => {
            let ctx = ChainContext::new(&config, chain.selection()).await?;
            print_status(&ctx).await?
        }
    }

    Ok(())
}

struct ChainContext {
    chain_label: String,
    bonding_registry: Address,
    provider: EthProvider<ConcreteWriteProvider>,
    signer_address: Address,
}

impl ChainContext {
    async fn new(config: &AppConfig, selection: Option<&str>) -> Result<Self> {
        let chain = select_chain(config, selection)?;
        let bonding_registry = parse_address(chain.contracts.bonding_registry.address())?;

        let rpc = chain.rpc_url()?;
        let cipher = Cipher::from_file(config.key_file()).await?;
        let repositories = get_repositories(config)?;
        let signer = load_signer_from_repository(repositories.eth_private_key(), &cipher).await?;
        let provider = ProviderConfig::new(rpc, chain.rpc_auth.clone())
            .create_signer_provider(&signer)
            .await?;
        let signer_address = provider.provider().default_signer_address();

        let label = selection.unwrap_or(chain.name.as_str()).to_string();

        Ok(Self {
            chain_label: label,
            bonding_registry,
            provider,
            signer_address,
        })
    }

    fn provider_client(&self) -> ConcreteWriteProvider {
        self.provider.provider().clone()
    }

    fn bonding(
        &self,
    ) -> BondingRegistryContract::BondingRegistryContractInstance<ConcreteWriteProvider> {
        BondingRegistryContract::new(self.bonding_registry, self.provider_client())
    }

    fn operator(&self) -> Address {
        self.signer_address
    }

    async fn license_token_address(&self) -> Result<Address> {
        Ok(self.bonding().licenseToken().call().await?)
    }

    async fn ticket_token_address(&self) -> Result<Address> {
        Ok(self.bonding().ticketToken().call().await?)
    }

    async fn ticket_underlying_address(&self) -> Result<Address> {
        let ticket = self.ticket_token_address().await?;
        Ok(
            EnclaveTicketTokenContract::new(ticket, self.provider_client())
                .underlying()
                .call()
                .await?,
        )
    }

    fn erc20(
        &self,
        address: Address,
    ) -> IERC20Metadata::IERC20MetadataInstance<ConcreteWriteProvider> {
        IERC20Metadata::new(address, self.provider_client())
    }

    fn enclave_token(
        &self,
        address: Address,
    ) -> EnclaveTokenContract::EnclaveTokenContractInstance<ConcreteWriteProvider> {
        EnclaveTokenContract::new(address, self.provider_client())
    }
}

fn select_chain<'a>(config: &'a AppConfig, name: Option<&str>) -> Result<&'a ChainConfig> {
    match name {
        Some(desired) => config
            .chains()
            .iter()
            .find(|c| c.name == desired)
            .ok_or_else(|| anyhow!("Chain '{}' not found in configuration", desired)),
        None => config
            .chains()
            .first()
            .ok_or_else(|| anyhow!("No chains configured. Run `enclave config-set` first.")),
    }
}

fn parse_address(value: &str) -> Result<Address> {
    Address::from_str(value).context("Invalid address in configuration")
}

async fn handle_license(ctx: &ChainContext, command: LicenseCommands) -> Result<()> {
    match command {
        LicenseCommands::Acquire { amount, allocation } => {
            let license = ctx.license_token_address().await?;
            let metadata = ctx.erc20(license);
            let decimals = metadata.decimals().call().await?;
            let parsed = parse_amount(&amount, decimals)?;
            let tx = ctx
                .enclave_token(license)
                .mintAllocation(ctx.operator(), parsed, allocation)
                .send()
                .await?;
            let receipt = tx.get_receipt().await?;
            println!(
                "Minted {} ENCL on {} (tx: {:#x})",
                amount, ctx.chain_label, receipt.transaction_hash
            );
        }
        LicenseCommands::Bond { amount } => {
            bond_license(ctx, &amount).await?;
        }
        LicenseCommands::Unbond { amount } => {
            let license = ctx.license_token_address().await?;
            let decimals = ctx.erc20(license).decimals().call().await?;
            let parsed = parse_amount(&amount, decimals)?;
            let receipt = ctx
                .bonding()
                .unbondLicense(parsed)
                .send()
                .await?
                .get_receipt()
                .await?;
            println!(
                "Queued {} ENCL for exit (tx: {:#x})",
                amount, receipt.transaction_hash
            );
        }
        LicenseCommands::Claim {
            max_ticket,
            max_license,
        } => {
            let ticket_decimals = ctx
                .erc20(ctx.ticket_token_address().await?)
                .decimals()
                .call()
                .await?;
            let license_decimals = ctx
                .erc20(ctx.license_token_address().await?)
                .decimals()
                .call()
                .await?;

            let ticket = if let Some(value) = max_ticket {
                parse_amount(&value, ticket_decimals)?
            } else {
                U256::MAX
            };
            let license = if let Some(value) = max_license {
                parse_amount(&value, license_decimals)?
            } else {
                U256::MAX
            };
            let receipt = ctx
                .bonding()
                .claimExits(ticket, license)
                .send()
                .await?
                .get_receipt()
                .await?;
            println!("Claimed exits (tx: {:#x})", receipt.transaction_hash);
        }
    }

    Ok(())
}

async fn bond_license(ctx: &ChainContext, amount: &str) -> Result<()> {
    let license = ctx.license_token_address().await?;
    let erc20 = ctx.erc20(license);
    let decimals = erc20.decimals().call().await?;
    let parsed = parse_amount(amount, decimals)?;
    ensure_allowance(ctx, license, ctx.bonding_registry, parsed).await?;
    let receipt = ctx
        .bonding()
        .bondLicense(parsed)
        .send()
        .await?
        .get_receipt()
        .await?;
    println!(
        "Bonded {} ENCL (tx: {:#x})",
        amount, receipt.transaction_hash
    );
    Ok(())
}

async fn handle_tickets(ctx: &ChainContext, command: TicketCommands) -> Result<()> {
    match command {
        TicketCommands::Buy { amount } => {
            let ticket_contract = ctx.ticket_token_address().await?;
            let underlying = ctx.ticket_underlying_address().await?;
            let metadata = ctx.erc20(underlying);
            let decimals = metadata.decimals().call().await?;
            let parsed = parse_amount(&amount, decimals)?;
            ensure_allowance(ctx, underlying, ticket_contract, parsed).await?;
            let receipt = ctx
                .bonding()
                .addTicketBalance(parsed)
                .send()
                .await?
                .get_receipt()
                .await?;
            println!(
                "Purchased {} tickets (tx: {:#x})",
                amount, receipt.transaction_hash
            );
        }
        TicketCommands::Burn { amount } => {
            let ticket_contract = ctx.ticket_token_address().await?;
            let decimals = ctx.erc20(ticket_contract).decimals().call().await?;
            let parsed = parse_amount(&amount, decimals)?;
            let receipt = ctx
                .bonding()
                .removeTicketBalance(parsed)
                .send()
                .await?
                .get_receipt()
                .await?;
            println!(
                "Removed {} tickets (tx: {:#x})",
                amount, receipt.transaction_hash
            );
        }
    }

    Ok(())
}

async fn register_operator(ctx: &ChainContext) -> Result<()> {
    let receipt = ctx
        .bonding()
        .registerOperator()
        .send()
        .await?
        .get_receipt()
        .await?;
    println!(
        "Registered ciphernode on {} (tx: {:#x})",
        ctx.chain_label, receipt.transaction_hash
    );
    Ok(())
}

async fn deregister_operator(ctx: &ChainContext, siblings: Vec<String>) -> Result<()> {
    let proof = parse_u256_list(&siblings)?;
    let receipt = ctx
        .bonding()
        .deregisterOperator(proof)
        .send()
        .await?
        .get_receipt()
        .await?;
    println!(
        "Deregistration requested (tx: {:#x})",
        receipt.transaction_hash
    );
    Ok(())
}

async fn enforce_activation(ctx: &ChainContext) -> Result<()> {
    register_operator(ctx).await
}

async fn deactivate_operator(
    ctx: &ChainContext,
    ticket_amount: Option<String>,
    license_amount: Option<String>,
) -> Result<()> {
    if ticket_amount.is_none() && license_amount.is_none() {
        bail!(
            "Provide --tickets and/or --license to specify what should be withdrawn for deactivation"
        );
    }

    if let Some(amount) = ticket_amount {
        let ticket_contract = ctx.ticket_token_address().await?;
        let decimals = ctx.erc20(ticket_contract).decimals().call().await?;
        let parsed = parse_amount(&amount, decimals)?;
        let receipt = ctx
            .bonding()
            .removeTicketBalance(parsed)
            .send()
            .await?
            .get_receipt()
            .await?;
        println!(
            "Removed {} tickets (tx: {:#x})",
            amount, receipt.transaction_hash
        );
    }

    if let Some(amount) = license_amount {
        let license = ctx.license_token_address().await?;
        let decimals = ctx.erc20(license).decimals().call().await?;
        let parsed = parse_amount(&amount, decimals)?;
        let receipt = ctx
            .bonding()
            .unbondLicense(parsed)
            .send()
            .await?
            .get_receipt()
            .await?;
        println!(
            "Queued {} ENCL for exit (tx: {:#x})",
            amount, receipt.transaction_hash
        );
    }

    println!("Submitted deactivation transactions; monitor exit delays before claiming.");
    Ok(())
}

async fn print_status(ctx: &ChainContext) -> Result<()> {
    let contract = ctx.bonding();
    let operator = ctx.operator();
    let ticket_balance: U256 = contract.getTicketBalance(operator).call().await?;
    let license_bond: U256 = contract.getLicenseBond(operator).call().await?;
    let available_tickets: U256 = contract.availableTickets(operator).call().await?;
    let is_registered: bool = contract.isRegistered(operator).call().await?;
    let is_active: bool = contract.isActive(operator).call().await?;
    let has_exit: bool = contract.hasExitInProgress(operator).call().await?;
    let pending = contract.pendingExits(operator).call().await?;
    let pending_tickets = pending.ticket;
    let pending_license = pending.license;
    let ticket_price: U256 = contract.ticketPrice().call().await?;
    let min_ticket_balance: U256 = contract.minTicketBalance().call().await?;
    let license_required: U256 = contract.licenseRequiredBond().call().await?;

    let ticket_token = ctx.ticket_token_address().await?;
    let license_token = ctx.license_token_address().await?;
    let ticket_decimals = ctx.erc20(ticket_token).decimals().call().await?;
    let license_decimals = ctx.erc20(license_token).decimals().call().await?;

    println!("Ciphernode status on {}:", ctx.chain_label);
    println!("  Address: {:#x}", operator);
    println!("  Registered: {}", is_registered);
    println!("  Active: {}", is_active);
    println!("  Exit pending: {}", has_exit);
    println!(
        "  Ticket balance: {} ({} available)",
        format_amount(ticket_balance, ticket_decimals),
        format_amount(available_tickets, ticket_decimals)
    );
    println!(
        "  License bond: {}",
        format_amount(license_bond, license_decimals)
    );
    println!(
        "  Pending exits: tickets={}, license={}",
        format_amount(pending_tickets, ticket_decimals),
        format_amount(pending_license, license_decimals)
    );
    println!(
        "  Requirements: minTickets={}, ticketPrice={} EKT, licenseBond={} ENCL",
        format_amount(min_ticket_balance, ticket_decimals),
        format_amount(ticket_price, ticket_decimals),
        format_amount(license_required, license_decimals)
    );
    Ok(())
}

fn format_amount(amount: U256, decimals: u8) -> String {
    let scale = U256::from(10u64).pow(U256::from(decimals as u64));
    let int_part = amount / scale;
    let frac_part = amount % scale;

    if frac_part == U256::from(0) {
        int_part.to_string()
    } else {
        let frac_str = frac_part.to_string();
        let frac_padded = format!("{:0>width$}", frac_str, width = decimals as usize);
        let frac_trimmed = frac_padded.trim_end_matches('0');
        if frac_trimmed.is_empty() {
            int_part.to_string()
        } else {
            format!("{}.{}", int_part, frac_trimmed)
        }
    }
}

fn parse_amount(value: &str, decimals: u8) -> Result<U256> {
    let normalized = value.trim().replace('_', "");
    if normalized.is_empty() {
        bail!("Amount cannot be empty");
    }

    let parts: Vec<&str> = normalized.split('.').collect();
    if parts.len() > 2 {
        bail!("Invalid decimal amount '{}'", value);
    }

    let int_part = parts[0];
    let int_value = U256::from_str(int_part).context("Invalid integer component")?;
    let scale = U256::from(10u64).pow(U256::from(decimals as u64));
    let mut result = int_value * scale;

    if parts.len() == 2 {
        let frac = parts[1];
        if frac.is_empty() {
            return Ok(result);
        }
        let frac_len = frac.len();
        if frac_len > decimals as usize {
            bail!(
                "Fractional precision exceeds token decimals ({} > {})",
                frac_len,
                decimals
            );
        }
        let frac_value = U256::from_str(frac).context("Invalid fractional component")?;
        let power = decimals as usize - frac_len;
        let multiplier = U256::from(10u64).pow(U256::from(power as u64));
        result += frac_value * multiplier;
    }

    Ok(result)
}

fn parse_u256_list(values: &[String]) -> Result<Vec<U256>> {
    values
        .iter()
        .filter(|s| !s.trim().is_empty())
        .map(|value| parse_u256(value))
        .collect()
}

fn parse_u256(value: &str) -> Result<U256> {
    let trimmed = value.trim();
    if let Some(hex) = trimmed
        .strip_prefix("0x")
        .or_else(|| trimmed.strip_prefix("0X"))
    {
        U256::from_str_radix(hex, 16).context("Invalid hex value")
    } else {
        U256::from_str(trimmed).context("Invalid decimal value")
    }
}

async fn ensure_allowance(
    ctx: &ChainContext,
    token: Address,
    spender: Address,
    amount: U256,
) -> Result<()> {
    let erc20 = ctx.erc20(token);
    let current = erc20.allowance(ctx.operator(), spender).call().await?;
    if current >= amount {
        return Ok(());
    }

    erc20
        .approve(spender, amount)
        .send()
        .await?
        .get_receipt()
        .await?;
    Ok(())
}
