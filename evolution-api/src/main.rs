mod api;
mod config;

use std::io;

use ::config::ConfigError;
use actix_web::{middleware::Logger, web::Data, App, HttpServer};
use api::ApiDoc;
use config::{app::AppConfig, log};
use thiserror::Error;
use utoipa::OpenApi;
use utoipa_rapidoc::RapiDoc;

const API_DOC_PATH: &str = "/doc";
const API_MANIFEST_PATH: &str = "/api-docs/openapi.json";

#[derive(Error, Debug)]
pub enum AppError {
    #[error("Unable to load configuration: {0}")]
    InvalidAppConfig(#[from] ConfigError),
    #[error("Unable to parse log config from embedded file config: {0}")]
    InvalidLogConfigFile(#[from] toml::de::Error),
}

impl From<AppError> for io::Error {
    fn from(e: AppError) -> Self {
        io::Error::new(io::ErrorKind::Other, e)
    }
}

#[actix_web::main]
async fn main() -> io::Result<()> {
    log::init();

    let app_config = AppConfig::new()?;
    let bind_settings = (app_config.service_host.clone(), app_config.service_port);

    let openapi = ApiDoc::openapi();

    let data = Data::new(app_config);
    HttpServer::new(move || {
        App::new()
            .app_data(data.clone())
            .wrap(Logger::default())
            .configure(api::v1::configure())
            .service(RapiDoc::with_openapi(API_MANIFEST_PATH, openapi.clone()).path(API_DOC_PATH))
    })
    .bind(bind_settings)?
    .run()
    .await
}

#[cfg(test)]
mod tests {
    use crate::AppError;
    use config::ConfigError;
    use std::io::{self, ErrorKind};

    #[test]
    fn test_from() {
        let error = AppError::InvalidAppConfig(ConfigError::Frozen);
        let result = io::Error::from(error);

        assert_eq!(ErrorKind::Other, result.kind());
    }
}
