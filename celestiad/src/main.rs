use std::{collections::BTreeMap, net::SocketAddr};

use clap::{Arg, Command};
use cozo::{DataValue, DbInstance, ScriptMutability, UlidWrapper};
use dotenv::dotenv;
use celestiad::{commands, settings, CELESTIAD_DATA_DIR, CELESTIAD_ENV_PREFIX, CELESTIAD_PORT, HOST_SCHEMA, LOCALHOST, SPACEPORT_SCHEMA, TCP_ENDPOINT, UDP_ENDPOINT};
use tracing::debug;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};
use ulid::Ulid;
use zenoh::config::{WhatAmI, ZenohId};

#[tokio::main]
pub async fn main() -> anyhow::Result<()> {
    dotenv().ok();

    let mut command = Command::new("celestiad")
        .arg(
            Arg::new("config")
                .short('c')
                .long("config")
                .help("Configuration file location")
                .default_value("/etc/constellations/config.json"),
        );

    command = commands::configure(command);

    let matches = command.get_matches();

    let config_location = matches
        .get_one("config")
        .map(|s: &String| s.as_str());

    let settings = settings::Settings::new(config_location, CELESTIAD_ENV_PREFIX)?;

    // Setup logging
    let fmt_layer = tracing_subscriber::fmt::layer()
        .with_target(true);
    let filter_layer = EnvFilter::try_from_default_env()
        .or_else(|_| EnvFilter::builder().parse(settings.logging.log_level.clone().unwrap().as_str()))
        .unwrap();

    tracing_subscriber::registry()
        .with(filter_layer)
        .with(fmt_layer)
        .init();

    commands::handle(&matches, &settings)?;

    let db = cozo::DbInstance::new("sqlite", format!("{}/celestiad.sqlite", CELESTIAD_DATA_DIR), "").unwrap();
    let mut parameters = BTreeMap::new();
    parameters.insert("self".to_string(), DataValue::Bool(true));

    let id = match setup_db(&db) {
        Ok(_) => {
            let new_id = Ulid::new();
            parameters.insert("id".to_string(), DataValue::Ulid(UlidWrapper(new_id)));
            let _ = db.run_script(
                "?[id, self] <- [[$id, $self]] :put host {id, self}",
                parameters,
                ScriptMutability::Mutable
            );
            new_id
        },
        Err(_) => {
            let result = db.run_script(
                "?[a] := *host{id: a, self: $self}",
                parameters,
                ScriptMutability::Mutable
            ).unwrap();
            let id = result.rows[0][0].clone().get_ulid().unwrap();
            debug!("Database already exists! Host ID: {}", id);
            id
        }
    };

    run(id, db, CELESTIAD_PORT).await;

    Ok(())
}

async fn run(id: Ulid, db: DbInstance, port: u16) {
    let mut config = zenoh::Config::default();
    config.set_id(ZenohId::try_from(id.to_bytes().as_slice()).unwrap()).unwrap();
    config.set_mode(Some(WhatAmI::Peer));
    config.transport.link.set_protocols(Some(vec!["tcp".to_string(), "udp".to_string()]));
    config.listen.endpoints.set(
        [TCP_ENDPOINT].iter().map(|s|s.parse().unwrap()).collect()
    ).unwrap();

    let zenoh_session = zenoh::open(config).await.unwrap();
    
    let addr = SocketAddr::new(
        LOCALHOST,
        port
    );

    let router = celestiad::api::configure(db, zenoh_session);

    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();

    axum::serve(listener, router.into_make_service()).await.unwrap();
}

fn setup_db(db: &DbInstance) -> Result<(), cozo::Error> {
    db.run_default(HOST_SCHEMA)?;
    db.run_default(SPACEPORT_SCHEMA)?;
    Ok(())
}