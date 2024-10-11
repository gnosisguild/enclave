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

pub struct InMemDataStore {
    db: BTreeMap<Vec<u8>, Vec<u8>>,
    log: Vec<DataOp>,
    capture: bool,
}

impl Actor for InMemDataStore {
    type Context = Context<Self>;
}

impl InMemDataStore {
    pub fn new(capture: bool) -> Self {
        Self {
            db: BTreeMap::new(),
            capture,
            log: vec![],
        }
    }
}

impl Handler<Insert> for InMemDataStore {
    type Result = ();
    fn handle(&mut self, event: Insert, _: &mut Self::Context) {
        // insert data into sled
        self.db.insert(event.key(), event.value());

        if self.capture {
            self.log.push(DataOp::Insert(event));
        }
    }
}

impl Handler<Get> for InMemDataStore {
    type Result = Option<Vec<u8>>;
    fn handle(&mut self, event: Get, _: &mut Self::Context) -> Option<Vec<u8>> {
        let key = event.key();
        self.db.get(&key).map(|r| r.clone())
    }
}

impl Handler<GetLog> for InMemDataStore {
    type Result = Vec<DataOp>;
    fn handle(&mut self, _: GetLog, _: &mut Self::Context) -> Vec<DataOp> {
        self.log.clone()
    }
}
