use std::{collections::HashMap, io, str::FromStr};
use cozo::DbInstance;
use tracing;

use ulid::Ulid;
use zenoh::{self, config::{WhatAmI, ZenohId}, Config};

// Provides services (data, communication, AI)
// HTTP for management via web ui, cli, tui, or native ui (tonic or hyper)
// Stores keys and configs in user home directory inside the .constellations folder.
// ~/.constellations/quarmire - commander directory; keys live here
// ~/.constellations/salvador - spaceport directory; configs live here
// ~/.constellations/frontier - another spaceport
// ~/.constellations/frontier/llama3 - service directory; service config lives here
// ~/.constellations/quarmire/holobank - holobank directory; dbinstance lives here
// ~/.constellations/quarmire/celestia-tui-seeker01 - spacecraft directory; application state lives here
// ~/.constellations/iro-01/ - starport

// Executed as command:
// spaceportd {home_directory} {commander_name}
// All spaceports are open by default
// If commander private keys are not in commander folder, password authentication is required.
// Spaceportd is running.  The API can be accessed via localhost:5425

// CLI:
// spaceport list - lists spaceports on device (opened and closed) (option --system: lists open spaceports in network)
// spaceport open [name]
// spaceport close [name]
// spaceport facilities [name]

// Future IPC: iceoryx2 + zenoh
// Current IPC: zenoh

const ROOT_DIR: &str = "~/.constellations";

pub struct Spaceport {
    // Designed around a zenoh session
    // multiple spaceports on one celestial body is possible
    // use hyperrail to communicate between local spaceports

    // has means to discover (scout) and establish
    // (auto config) comms with other spaceports

    // Has two means of communication: fast messages (radio) or
    // large payloads (ships)
    id: Ulid,
    name: String,
    db: DbInstance,
    radio: zenoh::Session, // external comms
    docks: zenoh::Session, // internal comms
    banks: Option<HashMap<&str, Holobank>>,
}

impl Spaceport {
    /// Build a new spaceport.  There can be multiple spaceports on a
    /// celestial body but only a single spaceport per process.
    pub async fn new(name: &str) -> Spaceport {
        let db = cozo::DbInstance::new("sqlite", ROOT_DIR + "/" + name, "").unwrap();
        let id = Ulid::new();
        let internal_id = id.increment().unwrap();

        let radio = zenoh::open(config).await.unwrap();
        let docks = zenoh::open(config).await.unwrap();

        // zenoh id = spaceport id

        // System id discovery:
        // scout for starports (zenoh routers)
        // if starport is found; get star system name
        // else scout for spaceports (zenoh peers)
        // if spaceport is found; get system id
        // else generate system ulid (time of discovery encoded)
        // this id is always the oldest active spaceport in the system
        // does not tie system id to network name, i.e., Wi-Fi SSID

        // Commander spacecraft:
        // stores commander id matched with username and personal info
        // stores biometric info
        // provides authentication service
        // peer nodes query the network for commander id using username + birthday
        // commander id is embedded in spacecraft and spaceports

        Spaceport {
            id,
            name: name.to_string(),
            db,
            radio,
            docks,
            banks: Some(HashMap::new()),
        }
    }
    /// Open essential docks and facilities.
    pub async fn open(name: &str) -> Spaceport {
        let db = cozo::DbInstance::new("sqlite", ROOT_DIR + "/" + name, "").unwrap();
    }
    /// Close all docks and put facilities into hibernation.
    pub async fn close() {
        todo!()
    }

    fn configure_docks(spaceport_id: Ulid) -> io::Result<Config> {
        let dock_id = spaceport_id.increment().unwrap().to_string();
        let mut config = zenoh::Config::default();
        config.set_id(ZenohId::from_str(&dock_id).unwrap()).unwrap();
        config.set_mode(Some(WhatAmI::Router));
        config.transport.link.set_protocols(Some(vec!["unixsock-stream".to_string()]));
        config.listen.endpoints.set(
            ["unixsock-stream/" + dock_id].iter().map(|s|s.parse().unwrap()).collect()
        ).unwrap();
        Ok(config)
    }

    fn configure_radio(spaceport_id: Ulid) -> io::Result<Config> {
        let mut config = zenoh::Config::default();
        config.set_id(ZenohId::from_str(&spaceport_id.to_string()).unwrap()).unwrap();
        config.set_mode(Some(WhatAmI::Peer));
        config.transport.link.set_protocols(Some(vec!["tcp".to_string(), "udp".to_string()]));
        config.listen.endpoints.set(
            ["tcp/[::]:0"].iter().map(|s|s.parse().unwrap()).collect()
        ).unwrap();
        Ok(config)
    }
}