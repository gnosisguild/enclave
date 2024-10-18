use actix::{Actor, Context, Handler, Message};
use std::collections::BTreeMap;

use crate::{Get, Insert};

#[derive(Message, Clone, Debug, PartialEq, Eq, Hash)]
#[rtype(result = "Vec<DataOp>")]
pub struct GetLog;

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum DataOp {
    Insert(Insert),
}

pub struct InMemStore {
    db: BTreeMap<Vec<u8>, Vec<u8>>,
    log: Vec<DataOp>,
    capture: bool,
}

impl Actor for InMemStore {
    type Context = Context<Self>;
}

impl InMemStore {
    pub fn new(capture: bool) -> Self {
        Self {
            db: BTreeMap::new(),
            capture,
            log: vec![],
        }
    }
}

impl Handler<Insert> for InMemStore {
    type Result = ();
    fn handle(&mut self, event: Insert, _: &mut Self::Context) {
        // insert data into sled
        self.db.insert(event.key(), event.value());

        if self.capture {
            self.log.push(DataOp::Insert(event));
        }
    }
}

impl Handler<Get> for InMemStore {
    type Result = Option<Vec<u8>>;
    fn handle(&mut self, event: Get, _: &mut Self::Context) -> Option<Vec<u8>> {
        let key = event.key();
        self.db.get(&key).map(|r| r.clone())
    }
}

impl Handler<GetLog> for InMemStore {
    type Result = Vec<DataOp>;
    fn handle(&mut self, _: GetLog, _: &mut Self::Context) -> Vec<DataOp> {
        self.log.clone()
    }
}
