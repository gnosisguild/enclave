use anyhow::Result;
use e3_program_server::E3ProgramServer;

#[tokio::main]
async fn main() -> Result<()> {
    let server = E3ProgramServer::builder(move |_| async { Ok((vec![], vec![])) }).build();
    server.run().await?;
    println!("Hello, world!");
    Ok(())
}
