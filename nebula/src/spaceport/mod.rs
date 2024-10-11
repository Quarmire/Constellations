mod home;
mod bank;

use std::{io, str::FromStr};

use bank::HoloBank;
use zenoh::{self, config::ZenohId, Config};

pub struct Spaceport {
    // Designed around a zenoh session
    // multiple spaceports on one celestial body is possible
    // use hyperrail to communicate between local spaceports

    // has means to discover (scout) and establish 
    // (auto config) comms with other spaceports

    // Has two means of communication: fast messages or
    // large payloads
    

    // certain config is stored locally such as zenoh config
    z_session: zenoh::Session,
    bank: HoloBank
}

impl Spaceport {
    pub async fn new() -> Spaceport {
        let config = Spaceport::configure().unwrap();
        let z_session = zenoh::open(config).await.unwrap();
        let bank = HoloBank::new("./test");
        println!("{}", bank);

        Spaceport {
            z_session,
            bank
        }
    }

    fn configure() -> io::Result<Config> {
        let mut config = zenoh::Config::default();
        config.set_id(ZenohId::from_str("221b72df20924c15b8794c6bdb471150").unwrap()).unwrap();
        config.connect.endpoints.set(
            ["tcp/10.10.10.10:7447", "tcp/11.11.11.11:7447"].iter().map(|s|s.parse().unwrap()).collect()).unwrap();
        Ok(config)
    }
}