use actix::{Actor, Addr, AsyncContext, Context, Handler, Message, WrapFuture};
use alloy::{
    primitives::B256,
    providers::{Provider, RootProvider},
    rpc::types::{Filter, Log},
    sol_types::SolEvent,
    transports::BoxTransport,
};
use anyhow::Result;
use futures_util::stream::StreamExt;
use std::collections::HashMap;
use std::fmt::Debug;
use std::sync::Arc;
use std::marker::PhantomData;

use crate::{EnclaveEvent, EventBus};

pub trait ContractEvent: Send + Sync + 'static {
    fn process(&self, bus: Addr<EventBus>) -> Result<()>;
}

impl<T> ContractEvent for T
where
    T: SolEvent + Debug + Send + Sync + 'static,
{
    fn process(&self, bus: Addr<EventBus>) -> Result<()> {
        println!("Processing event: {:?}", self);
        // bus.do_send(EnclaveEvent::from(self));
        Ok(())
    }
}

pub struct EvmEventListener {
    provider: Arc<RootProvider<BoxTransport>>,
    filter: Filter,
    handlers: HashMap<B256, Arc<dyn Fn(Log) -> Result<Box<dyn ContractEvent>> + Send + Sync>>,
    bus: Addr<EventBus>,
}

impl EvmEventListener {
    pub fn new(
        provider: Arc<RootProvider<BoxTransport>>,
        filter: Filter,
        bus: Addr<EventBus>,
    ) -> Self {
        Self {
            provider,
            filter,
            handlers: HashMap::new(),
            bus,
        }
    }

    pub fn add_event_handler<E>(&mut self)
    where
        E: SolEvent + ContractEvent + 'static,
    {
        let signature = E::SIGNATURE_HASH;
        let handler = Arc::new(move |log: Log| -> Result<Box<dyn ContractEvent>> {
            let event = log.log_decode::<E>()?.inner.data;
            Ok(Box::new(event))
        });

        self.handlers.insert(signature, handler);
    }

    pub async fn listen(&self) -> Result<()> {
        let mut stream = self
            .provider
            .subscribe_logs(&self.filter)
            .await?
            .into_stream();
        while let Some(log) = stream.next().await {
            if let Some(topic0) = log.topic0() {
                if let Some(decoder) = self.handlers.get(topic0) {
                    if let Ok(event) = decoder(log.clone()) {
                        event.process(self.bus.clone())?;
                    }
                }
            }
        }

        Ok(())
    }
}

impl Actor for EvmEventListener {
    type Context = Context<Self>;
}

#[derive(Message)]
#[rtype(result = "()")]
pub struct AddEventHandler<E>
where
    E: SolEvent + ContractEvent + 'static,
{
    pub _marker: PhantomData<E>,
}

impl<E> AddEventHandler<E>
where
    E: SolEvent + ContractEvent + 'static,
{
    pub fn new() -> Self {
        Self {
            _marker: PhantomData,
        }
    }
}

impl<E> Handler<AddEventHandler<E>> for EvmEventListener
where
    E: SolEvent + ContractEvent + 'static,
{
    type Result = ();

    fn handle(&mut self, _: AddEventHandler<E>, _: &mut Self::Context) -> Self::Result {
        self.add_event_handler::<E>();
    }
}

#[derive(Message)]
#[rtype(result = "()")]
pub struct StartListening;

impl Handler<StartListening> for EvmEventListener {
    type Result = ();
    fn handle(&mut self, _: StartListening, ctx: &mut Self::Context) -> Self::Result {
        let (provider, filter, handlers, bus) = (
            self.provider.clone(),
            self.filter.clone(),
            self.handlers.clone(),
            self.bus.clone(),
        );

        ctx.spawn(
            async move {
                let listener = EvmEventListener {
                    provider,
                    filter,
                    handlers,
                    bus,
                };
                if let Err(err) = listener.listen().await {
                    eprintln!("Error listening for events: {:?}", err);
                }
            }
            .into_actor(self),
        );
    }
}
