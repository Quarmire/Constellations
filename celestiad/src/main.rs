use std::{collections::BTreeMap, net::{IpAddr, SocketAddr}, sync::Arc};

use clap::{Arg, Command};
use cozo::{DataValue, DbInstance, ScriptMutability, UlidWrapper};
use dotenv::dotenv;
use celestiad::{commands, settings::{self, Settings}, spaceport::Spaceport, HolobankRequest, SpaceportRequest, State, ALL, CELESTIAD_AUDIO_DIR, CELESTIAD_DATA_DIR, CELESTIAD_ENV_PREFIX, CELESTIAD_MODEL_DIR, CELESTIAD_PORT, HOLOBANK_SCHEMA, HOST_SCHEMA, LOCALHOST, SPACEPORT_SCHEMA, TCP_ENDPOINT, UDP_ENDPOINT};
use tokio::{sync::{mpsc, Mutex}, task::JoinHandle};
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

    let db = cozo::DbInstance::new("sqlite", format!("{}celestiad.sqlite", CELESTIAD_DATA_DIR), "").unwrap();
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

    run(id, db, CELESTIAD_PORT, settings).await;

    Ok(())
}

async fn run(id: Ulid, db: DbInstance, port: u16, settings: Settings) {
    let (task_tx, task_rx) = mpsc::channel::<JoinHandle<()>>(100);
    let (sp_tx, sp_rx) = mpsc::channel::<SpaceportRequest>(100);
    let (hb_tx, hb_rx) = mpsc::channel::<HolobankRequest>(100);

    // Start the join handle processor in a separate task
    let _ = tokio::spawn(celestiad::process_join_handles(task_rx));

    let mut config = zenoh::Config::default();
    config.set_id(ZenohId::try_from(id.to_bytes().as_slice()).unwrap()).unwrap();
    config.set_mode(Some(WhatAmI::Peer));
    config.transport.link.set_protocols(Some(vec!["tcp".to_string(), "udp".to_string()]));
    config.listen.endpoints.set(
        [TCP_ENDPOINT].iter().map(|s|s.parse().unwrap()).collect()
    ).unwrap();

    let zenoh_session: zenoh::Session = zenoh::open(config).await.unwrap();
    
    let addr = SocketAddr::new(
        ALL,
        port
    );

    let mut llm_addr = None;
    if settings.llm.llm_ip.is_some() & settings.llm.llm_port.is_some() {
        llm_addr = Some(SocketAddr::new(
            settings.llm.llm_ip.unwrap().parse().unwrap(),
            settings.llm.llm_port.unwrap()
        ));
    }

    let state = State {
        session: zenoh_session,
        db,
        task_tx: task_tx.clone(),
        llm_addr,
        spaceport_tx: sp_tx.clone(),
        holobank_tx: hb_tx,
    };

    let _ = tokio::spawn(celestiad::process_spaceport_requests(sp_rx, state.clone()));
    let _ = tokio::spawn(celestiad::process_holobank_requests(hb_rx));

    let router = celestiad::api::configure(state.clone());

    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();

    axum::serve(listener, router.into_make_service()).await.unwrap();
}

fn setup_db(db: &DbInstance) -> Result<(), cozo::Error> {
    db.run_default(HOST_SCHEMA)?;
    db.run_default(SPACEPORT_SCHEMA)?;
    db.run_default(HOLOBANK_SCHEMA)?;
    Ok(())
}