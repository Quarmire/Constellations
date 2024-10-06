mod home;
mod bank;

use std::{io, str::FromStr};

use bank::Bank;
use zenoh::{self, config::ZenohId, Config};

struct Spaceport {
    // Designed around a zenoh session
    // multiple spaceports on one celestial body is possible
    // use hyperrail to communicate between local spaceports

    // has means to discover (scout) and establish 
    // (auto config) comms with other spaceports
    

    // certain config is stored locally such as zenoh config
    z_session: zenoh::Session,
    bank: Bank
}

impl Spaceport {
    async fn new() -> Spaceport {
        let config = Spaceport::configure().unwrap();
        let z_session = zenoh::open(config).await.unwrap();
        let bank = Bank::new();

        Spaceport {
            z_session,
            bank
        }
    }

    fn configure() -> io::Result<Config> {
        let mut config = zenoh::Config::default();
        config.set_id(ZenohId::from_str("221b72df20924c15b8794c6bdb471150").unwrap());
        config.connect.endpoints.set(
            ["tcp/10.10.10.10:7447", "tcp/11.11.11.11:7447"].iter().map(|s|s.parse().unwrap()).collect());
        Ok(config)
    }
}