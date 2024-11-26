mod api;

use std::io;
use std::net::SocketAddr;
use crate::{settings::Settings, HolobankRequest};
use crate::{SpaceportRequest, LOCALHOST};

use anyhow::Result;
use tokio::task::JoinHandle;
use ulid::Ulid;
use tracing::{debug, info};
use tokio::{self, sync::mpsc};
use zenoh::{self, config::{WhatAmI, ZenohId}, Session};


const SPACEPORT_DIR: &str = "spaceport";
const SOCKET_DIR: &str = "/tmp";

#[derive(Clone)]
pub struct SpaceportState {
    pub session: Session,
    pub task_tx: mpsc::Sender<JoinHandle<()>>,
    pub holobank_tx: mpsc::Sender<HolobankRequest>,
    pub spaceport_tx: mpsc::Sender<SpaceportRequest>,
    pub celestiad_state: crate::State,
}

#[derive(Clone)]
pub struct Spaceport {
    pub id: Ulid,
    pub docks: Session,
}

impl Spaceport {
    /// Opens spaceport.
    pub async fn open(id: Ulid) -> Result<Spaceport> {
        let docks = Spaceport::open_docks(&id).await.unwrap();

        Ok(Spaceport {
            id,
            docks
        })
    }
    /// Close all docks and put facilities into hibernation.
    pub async fn close(&self) -> Result<()> {
        self.docks.close();
        Ok(())
    }

    async fn open_docks(spaceport_id: &Ulid) -> io::Result<Session> {
        let mut config = zenoh::Config::default();
        config.set_id(ZenohId::try_from(spaceport_id.to_bytes().as_slice()).unwrap()).unwrap();
        config.set_mode(Some(WhatAmI::Router));
        config.scouting.multicast.set_enabled(Some(false));
        config.scouting.gossip.set_enabled(Some(false));
        config.transport.link.set_protocols(Some(vec!["unixsock-stream".to_string()]));
        config.listen.endpoints.set([
            "unixsock-stream//tmp/".to_string() +
            "test_spaceport" +
            ".sock"
        ].iter().map(|s|s.parse().unwrap()).collect()
        ).unwrap();
        // config.transport.auth.usrpwd.set_dictionary_file(Some("auth.txt".to_string()));
        let session = zenoh::open(config).await.unwrap();
        Ok(session)
    }

    

    pub async fn serve_api(spaceport: Spaceport, spaceport_tx: mpsc::Sender<SpaceportRequest>, holobank_tx: mpsc::Sender<HolobankRequest>, celestiad_state: crate::State, port: u16) {
        let (task_tx, task_rx) = mpsc::channel::<JoinHandle<()>>(100);
        let _ = tokio::spawn(crate::process_join_handles(task_rx));

        let state = SpaceportState {
            session: spaceport.docks.clone(),
            task_tx,
            holobank_tx,
            spaceport_tx,
            celestiad_state,
        };

        let addr = SocketAddr::new(
            LOCALHOST,
            port
        );

        let router = crate::spaceport::api::configure(state);
        let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
        axum::serve(listener, router.into_make_service()).await.unwrap();
    }
}