mod api;

use std::io;
use std::net::SocketAddr;
use crate::settings::Settings;
use crate::LOCALHOST;

use anyhow::Result;
use ulid::Ulid;
use tracing::{debug, info};
use tokio;
use zenoh::{self, config::{WhatAmI, ZenohId}, Session};


const SPACEPORT_DIR: &str = "spaceport";
const SOCKET_DIR: &str = "/tmp";

#[derive(Clone)]
pub struct Spaceport {
    pub id: Ulid,
    docks: Session,
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
            spaceport_id.to_string().as_str() +
            ".sock"
        ].iter().map(|s|s.parse().unwrap()).collect()
        ).unwrap();
        // config.transport.auth.usrpwd.set_dictionary_file(Some("auth.txt".to_string()));
        let session = zenoh::open(config).await.unwrap();
        Ok(session)
    }

    

    async fn serve_api(spaceport: Spaceport, port: u16) {
        let addr = SocketAddr::new(
            LOCALHOST,
            port
        );
        let router = crate::spaceport::api::configure(spaceport);
        let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
        axum::serve(listener, router.into_make_service()).await.unwrap();
    }
}