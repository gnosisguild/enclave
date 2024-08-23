/// Actor for connecting to an libp2p client via it's mpsc channel interface 
/// This Actor should be responsible for 
/// 1. Sending and Recieving Vec<u8> messages with libp2p
/// 2. Converting between Vec<u8> and EnclaveEvents::Xxxxxxxxx()
/// 3. Broadcasting over the local eventbus
/// 4. Listening to the local eventbus for messages to be published to libp2p
use actix::{Actor, Context};
use tokio::sync::mpsc::{Receiver, Sender};
use p2p::EnclaveRouter;

pub struct P2p;

impl Actor for P2p{
    type Context = Context<Self>;
}

impl P2p {
    pub fn new() {
        // Construct owning Libp2p module
    }
    pub fn from_channel(tx:Sender<Vec<u8>>, rx:Receiver<Vec<u8>>){
        // Construct from tx/rx
    }
}
