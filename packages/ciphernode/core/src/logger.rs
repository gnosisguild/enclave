use actix::{Actor, Addr, Context, Handler};

use crate::{EnclaveEvent, EventBus, Subscribe};

pub struct SimpleLogger;

impl SimpleLogger {
    pub fn attach(bus:Addr<EventBus>) -> Addr<Self>{
       let addr = Self.start();
       bus.do_send(Subscribe { 
           listener:addr.clone().recipient(),
           event_type: "*".to_string() 
       });
       addr
    }
}

impl Actor for SimpleLogger {
    type Context = Context<Self>;
}

impl Handler<EnclaveEvent> for SimpleLogger {
    type Result = ();
    fn handle(&mut self, msg: EnclaveEvent, _: &mut Self::Context) -> Self::Result {
        println!("{}", msg);
    }
}
