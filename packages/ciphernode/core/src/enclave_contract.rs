
use actix::{Actor, Context};

/// Manage an internal web3 instance and express protocol specific behaviour through the events it
/// accepts and emits to the EventBus
/// Monitor contract events using `contract.events().create_filter()` and rebroadcast to eventbus by
/// creating `EnclaveEvent` events
/// Delegate signing to a separate actor responsible for managing Eth keys
/// Accept eventbus events and forward as appropriate contract calls as required
pub struct EnclaveContract;

impl Actor for EnclaveContract{
    type Context = Context<Self>;
}


