pub mod commands;
pub mod settings;
pub mod api;
pub mod spaceport;
pub mod holobank;

use std::net::{IpAddr, Ipv4Addr};

pub const CELESTIAD_HOST_DIR: &str = "/etc/constellations";
pub const CELESTIAD_DATA_DIR: &str = "/var/lib/constellations";
pub const CELESTIAD_USER_DIR: &str = ".constellations";
pub const CELESTIAD_ENV_PREFIX: &str = "CELESTIA";

pub const LOCALHOST: IpAddr = IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1));
pub const TCP_ENDPOINT: &str = "tcp/[::]:7447";
pub const UDP_ENDPOINT: &str = "udp/[::]:7447";
pub const CELESTIAD_PORT: u16 = 9494;

pub const HOST_SCHEMA: &str = "
    :create host {
        id: Ulid,
        =>
        self: Bool,
    }
";

pub const SPACEPORT_SCHEMA: &str = "
    :create spaceport {
        spaceport_id: Ulid,
        =>
        spaceport_name: String,
    }
";

// let sp_db = cozo::DbInstance::new("sqlite", format!("{}/test/test.sqlite", root), "").unwrap();
// let mut parameters = BTreeMap::new();
// parameters.insert("name".to_string(), DataValue::Str(name.into()));
// let id = match Spaceport::setup_sp_db(&sp_db, crate::holobank::schema::SPACEPORT_SCHEMA) {
//     Ok(_) => {
//         let new_id = Ulid::new();
//         parameters.insert("id".to_string(), DataValue::Ulid(UlidWrapper(new_id)));
//         sp_db.run_script(
//             "?[id, name] <- [[$id, $name]] :put spaceport {id => name}",
//             parameters,
//             ScriptMutability::Mutable
//         );
//         new_id
//     },
//     Err(_) => {
//         let result = sp_db.run_script(
//             "?[a] := *spaceport{id: a, name: $name}",
//             parameters,
//             ScriptMutability::Mutable
//         ).unwrap();
//         let id = result.rows[0][0].clone().get_ulid().unwrap();
//         debug!("Database already exists for spaceport: {} (ID: {})", name, id);
//         id
//     }
// };