use anyhow::Ok;
use config::{Config, Environment, File};
use serde::Deserialize;

#[derive(Debug, Deserialize, Default)]
#[allow(unused)]
pub struct Logging {
    pub log_level: Option<String>,
    pub sp_log_level: Option<String>,
}

#[derive(Debug, Deserialize, Default)]
#[allow(unused)]
pub struct Root {
    pub directory: Option<String>,
}

#[derive(Debug, Deserialize, Default)]
#[allow(unused)]
pub struct ConfigInfo {
    pub location: Option<String>,
    pub env_prefix: Option<String>,
}

#[derive(Debug, Deserialize, Default)]
#[allow(unused)]
pub struct Settings {
    #[serde(default)]
    pub logging: Logging,
    #[serde(default)]
    pub root: Root,
    #[serde(default)]
    pub config: ConfigInfo,
}

impl Settings {
    pub fn new(location: Option<&str>, env_prefix: &str) -> anyhow::Result<Self> {
        let mut builder = Config::builder();
        if let Some(location) = location {
            builder = builder.add_source(File::with_name(location))
        }

        let s = builder
            .add_source(
                Environment::with_prefix(env_prefix)
                    .separator("__")
                    .prefix_separator("__"),
            )
            .set_override("config.location", location)?
            .set_override("config.env_prefix", env_prefix)?
            .build()?;

        let settings = s.try_deserialize()?;

        Ok(settings)
    }
}