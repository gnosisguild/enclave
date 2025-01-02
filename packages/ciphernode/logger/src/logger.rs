use actix::{Actor, Addr, Context, Handler};
use events::{EnclaveEvent, Event, EventBus, Subscribe};
use std::marker::PhantomData;
use tracing::{error, info};

pub trait EventLogging: Event {
    fn log(&self, logger_name: &str);
}

pub struct SimpleLogger<E: EventLogging> {
    name: String,
    _p: PhantomData<E>,
}

impl<E: EventLogging> SimpleLogger<E> {
    pub fn attach(name: &str, bus: Addr<EventBus<E>>) -> Addr<Self> {
        let addr = Self {
            name: name.to_owned(),
            _p: PhantomData,
        }
        .start();
        bus.do_send(Subscribe::<E>::new(
            "*".to_string(),
            addr.clone().recipient(),
        ));
        info!(node=%name, "READY!");
        addr
    }
}

impl<E: EventLogging> Actor for SimpleLogger<E> {
    type Context = Context<Self>;
}

impl<E: EventLogging> Handler<E> for SimpleLogger<E> {
    type Result = ();

    fn handle(&mut self, msg: E, _: &mut Self::Context) -> Self::Result {
        msg.log(&self.name);
    }
}

impl EventLogging for EnclaveEvent {
    fn log(&self, logger_name: &str) {
        match self {
            EnclaveEvent::EnclaveError { .. } => error!(event=%self, "ERROR!"),
            _ => match self.get_e3_id() {
                Some(e3_id) => info!(me=logger_name, evt=%self, e3_id=%e3_id, "Event Broadcasted"),
                None => info!(me=logger_name, evt=%self, "Event Broadcasted"),
            },
        };
    }
}
