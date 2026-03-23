use anyhow::Result;
use clap::Subcommand;

#[derive(Subcommand, Clone, Debug)]
pub enum EventsCommands {
    /// Query events
    Query {
        #[arg(long)]
        aggregate: u64,

        #[arg(long)]
        since: u64,

        #[arg(long)]
        limit: u64,
    },
}

pub async fn execute(command: EventsCommands) -> Result<()> {
    match command {
        EventsCommands::Query {
            aggregate,
            since,
            limit,
        } => {
            query_events(aggregate, since, limit).await?;
        }
    }
    Ok(())
}

async fn query_events(aggregate: u64, since: u64, limit: u64) -> Result<()> {
    tracing::info!(
        "Querying events: aggregate={}, since={}, limit={}",
        aggregate,
        since,
        limit
    );

    let url = format!(
        "TODO: construct event query endpoint: /events?aggregate={}&since={}&limit={}",
        aggregate, since, limit
    );
    tracing::debug!("Endpoint: {}", url);

    Ok(())
}
