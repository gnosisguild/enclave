use super::datastore::get_repositories;
use actix::Actor;
use anyhow::anyhow;
use anyhow::Result;
use config::AppConfig;
use data::Repositories;
use events::{EnclaveEvent, EventBus, EventBusConfig};
use once_cell::sync::OnceCell;

// Static EVENT_BUS instance
static EVENT_BUS: OnceCell<actix::Addr<EventBus<EnclaveEvent>>> = OnceCell::new();

// Static REPOSITORIES instance
static REPOSITORIES: OnceCell<Repositories> = OnceCell::new();

pub fn get_static_repositories(config: &AppConfig) -> Result<&Repositories> {
    let bus = EVENT_BUS.get_or_init(|| {
        EventBus::<EnclaveEvent>::new(EventBusConfig {
            capture_history: true,
            deduplicate: true,
        })
        .start()
    });

    if REPOSITORIES.get().is_none() {
        let repositories = get_repositories(config, bus)?;
        let _ = REPOSITORIES.set(repositories);
    }

    REPOSITORIES
        .get()
        .ok_or(anyhow!("Could not get repository"))
}
