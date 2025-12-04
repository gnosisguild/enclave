/**
 * This is an example of how to best use actix to avoid sending stuff over async boundaries
 **/
use std::{ops::Deref, rc::Rc};

use actix::{Actor, Addr, AsyncContext, Context, Handler, Message, Recipient};

struct Alice {
    bob: Addr<Bob>,
    result: Option<String>,
}

impl Actor for Alice {
    type Context = Context<Self>;
    fn started(&mut self, ctx: &mut Self::Context) {
        self.bob
            .do_send(Responder::new(Greet::new("Alice"), ctx.address()))
    }
}

impl Alice {
    pub fn new(bob: &Addr<Bob>) -> Self {
        Self {
            bob: bob.clone(),
            result: None,
        }
    }
}

impl Handler<GreetResponse> for Alice {
    type Result = ();
    fn handle(&mut self, msg: GreetResponse, _: &mut Self::Context) -> Self::Result {
        self.result = Some(msg.value);
    }
}

impl Handler<GetInner> for Alice {
    type Result = Option<String>;
    fn handle(&mut self, msg: GetInner, _: &mut Self::Context) -> Self::Result {
        self.result.take()
    }
}

struct Bob {
    value: Rc<String>,
}

impl Actor for Bob {
    type Context = Context<Self>;
}

impl Bob {
    pub fn new(value: &str) -> Self {
        Self {
            value: Rc::new(value.into()),
        }
    }
}

impl Handler<Responder<Greet, GreetResponse>> for Bob {
    type Result = ();
    fn handle(
        &mut self,
        msg: Responder<Greet, GreetResponse>,
        _: &mut Self::Context,
    ) -> Self::Result {
        let reply = &format!("Hello {}! Regards, {}", msg.name, self.value);

        // do some sync work maybe...
        // could store the responder locally

        msg.reply(GreetResponse::new(reply));
    }
}

#[derive(Message)]
#[rtype("()")]
struct Greet {
    name: String,
}

impl Greet {
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_owned(),
        }
    }
}

#[derive(Message)]
#[rtype("()")]
struct GreetResponse {
    value: String,
}

impl GreetResponse {
    pub fn new(value: &str) -> Self {
        Self {
            value: value.to_owned(),
        }
    }
}

///////////////////////////////////////////////////////////////////////

#[derive(Message)]
#[rtype("Option<String>")]
struct GetInner;

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::{Alice, Bob, GetInner};
    use actix::prelude::*;
    use tokio::time::sleep;

    #[actix::test]
    async fn test_things() -> anyhow::Result<()> {
        let bob = Bob::new("Blurg").start();
        let alice = Alice::new(&bob).start();
        sleep(Duration::from_millis(1)).await;
        alice.send(GetInner).await?.unwrap();

        Ok(())
    }
}

///////////////////////////////////////////////////////////////////////

#[derive(Message)]
#[rtype("()")]
struct Responder<T, U: Send>
where
    U: Message + Send,
    U::Result: Send,
{
    value: T,
    sender: Recipient<U>,
}

impl<T, U> Responder<T, U>
where
    U: Message + Send,
    U::Result: Send,
{
    pub fn new(value: T, sender: impl Into<Recipient<U>>) -> Self {
        Self {
            value,
            sender: sender.into(),
        }
    }
    pub fn reply(&self, msg: U) {
        let sender = &self.sender;
        sender.do_send(msg);
    }
}

impl<T, U> Deref for Responder<T, U>
where
    U: Message + Send,
    U::Result: Send,
{
    type Target = T;
    fn deref(&self) -> &Self::Target {
        &self.value
    }
}
