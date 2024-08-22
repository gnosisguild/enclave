use actix::{Actor, Context};

use p2p::EnclaveRouter;
pub struct P2pActor;


impl Actor for P2pActor{
    type Context = Context<Self>;
}


