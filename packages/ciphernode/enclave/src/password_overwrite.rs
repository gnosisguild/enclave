use super::password_create::execute as set_password;
use super::password_delete::{prompt_delete, DeleteMode};
use anyhow::Result;
use config::AppConfig;

pub async fn execute(config: &AppConfig, input: Option<String>) -> Result<()> {
    if prompt_delete(config, DeleteMode::Overwrite).await? {
        set_password(config, input, true).await?;
    }
    Ok(())
}
