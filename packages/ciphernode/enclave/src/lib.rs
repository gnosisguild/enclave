use config::{Config, ConfigError, File};
use enclave_node::AppConfig;

pub fn load_config(config_path: &str) -> Result<AppConfig, ConfigError> {
    let config_builder = Config::builder()
        .add_source(File::with_name(&config_path).required(true))
        .build()?;

    // TODO: How do we ensure things like eth addresses are in a valid format?
    let config: AppConfig = config_builder.try_deserialize()?;

    Ok(config)
}
