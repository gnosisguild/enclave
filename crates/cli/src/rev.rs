pub const GIT_SHA: &str = env!("GIT_SHA");

pub async fn execute() -> anyhow::Result<()> {
    println!("{}", GIT_SHA);
    Ok(())
}
