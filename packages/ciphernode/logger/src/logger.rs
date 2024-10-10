use actix::{Actor, Addr, Context, Handler};
use enclave_core::{EnclaveEvent, EventBus, Subscribe};
use tracing::{error, info};

pub struct SimpleLogger {
    name: String,
}

impl SimpleLogger {
    pub fn attach(name: &str, bus: Addr<EventBus>) -> Addr<Self> {
        let addr = Self {
            name: name.to_owned(),
        }
        .start();
        bus.do_send(Subscribe {
            listener: addr.clone().recipient(),
            event_type: "*".to_string(),
        });
        info!(node=%name, "READY!");
        addr
    }
}

impl Actor for SimpleLogger {
    type Context = Context<Self>;
}

impl Handler<EnclaveEvent> for SimpleLogger {
    type Result = ();
    fn handle(&mut self, msg: EnclaveEvent, _: &mut Self::Context) -> Self::Result {
        match msg {
            EnclaveEvent::EnclaveError { .. } => error!(event=%msg, "ERROR!"),
            _ => match msg.get_e3_id() {
                Some(e3_id) => info!(me=self.name, evt=%msg, e3_id=%e3_id, "Event Broadcasted"),
                None => info!(me=self.name, evt=%msg, "Event Broadcasted"),
            },
        };
    }
}
