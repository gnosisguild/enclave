use std::collections::BTreeMap;

use actix::{Actor, Context, Handler, Message};

// TODO: replace with sled version

#[derive(Message, Clone, Debug, PartialEq, Eq, Hash)]
#[rtype(result = "()")]
pub struct Insert(pub Vec<u8>, pub Vec<u8>);
impl Insert {
    fn key(&self) -> Vec<u8> {
        self.0.clone()
    }

    fn value(&self) -> Vec<u8> {
        self.1.clone()
    }
}

#[derive(Message, Clone, Debug, PartialEq, Eq, Hash)]
#[rtype(result = "Option<Vec<u8>>")]
pub struct Get(pub Vec<u8>);
impl Get {
    fn key(&self) -> Vec<u8> {
        self.0.clone()
    }
}

#[derive(Message, Clone, Debug, PartialEq, Eq, Hash)]
#[rtype(result = "Vec<DataOp>")]
pub struct GetLog;

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum DataOp {
    Insert(Insert),
}

pub struct Data {
    db: BTreeMap<Vec<u8>, Vec<u8>>,
    log: Vec<DataOp>,
    capture: bool,
}

impl Actor for Data {
    type Context = Context<Self>;
}

impl Data {
    pub fn new(capture: bool) -> Self {
        Self {
            db: BTreeMap::new(),
            capture,
            log: vec![],
        }
    }
}

impl Handler<Insert> for Data {
    type Result = ();
    fn handle(&mut self, event: Insert, _: &mut Self::Context) {
        // insert data into sled
        self.db.insert(event.key(), event.value());

        if self.capture {
            self.log.push(DataOp::Insert(event));
        }
    }
}

impl Handler<Get> for Data {
    type Result = Option<Vec<u8>>;
    fn handle(&mut self, event: Get, _: &mut Self::Context) -> Option<Vec<u8>> {
        let key = event.key();
        self.db.get(&key).map(|r| r.clone())
    }
}

impl Handler<GetLog> for Data {
    type Result = Vec<DataOp>;
    fn handle(&mut self, _: GetLog, _: &mut Self::Context) -> Vec<DataOp> {
        self.log.clone()
    }
}
