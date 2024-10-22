use config::{Config, Environment, File, FileFormat};
use serde::Deserialize;

use crate::AppError;

const DEFAULT_CONFIG: &str = include_str!("../../resources/config/default.toml");
const DEFAULT_CONFIG_PREFIX: &str = "APP";

#[derive(Debug, Deserialize, Clone)]
pub struct AppConfig {
    pub service_host: String,
    pub service_port: u16,
    pub statsd_host: String,
    pub statsd_port: u16,
    pub statsd_factor: f32,
}

impl AppConfig {
    pub fn new() -> Result<Self, AppError> {
        let config = Config::builder()
            .add_source(File::from_str(DEFAULT_CONFIG, FileFormat::Toml))
            .add_source(Environment::with_prefix(DEFAULT_CONFIG_PREFIX))
            .build()?;

        config.try_deserialize().map_err(|e| e.into())
    }
}

#[cfg(test)]
mod tests {
    use super::AppConfig;

    #[test]
    fn test_new() {
        let result = AppConfig::new();
        assert!(
            matches!(result, Ok(_)),
            "By default, it should return a valid config"
        );

        let port = 8080u16;
        temp_env::with_var("APP_SERVICE_PORT", Some(port.to_string()), || {
            let result = AppConfig::new();
            assert!(
                matches!(result, Ok(x) if x.service_port == port),
                "Should take into account env vars"
            )
        });

        temp_env::with_var("APP_SERVICE_PORT", Some("invalid"), || {
            let result = AppConfig::new();
            assert!(
                matches!(result, Err(_)),
                "Should return error when config is not valid"
            )
        });
    }
}
