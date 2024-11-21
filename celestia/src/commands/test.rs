use clap::{value_parser, Arg, ArgMatches, Command};

use crate::{spaceport::Spaceport, settings::Settings};

pub const COMMAND_NAME: &str = "test";

pub fn configure() -> Command {
    Command::new(COMMAND_NAME).about("Creates a test spaceport for demo purposes.").arg(
        Arg::new("port")
            .short('p')
            .long("port")
            .value_name("PORT")
            .help("TCP port to listen on")
            .default_value("9999")
            .value_parser(value_parser!(u16))
    ).arg(
        Arg::new("name")
            .required(true)
    )
}

pub fn handle(matches: &ArgMatches, settings: &Settings) -> anyhow::Result<()> {
    let port: u16 = *matches.get_one("port").unwrap_or(&9999);
    let name = matches.get_one::<String>("name")
        .map(|s| s.as_str())
        .unwrap();
    
    println!("Opened test spaceport. {} can be accessed at localhost:{}", name, port);
    println!("Spacecraft can dock at {} using the credentials:", name);
    println!("- user: test_user");
    println!("- pass: 12345");

    open_spaceport(name, port, settings)?;

    Ok(())
}

fn open_spaceport(name: &str, port: u16, settings: &Settings) -> anyhow::Result<()> {
    tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()?
        .block_on(async move {
            let _ = Spaceport::open(name, port, settings).await;
            Ok::<(), anyhow::Error>(())
        })?;
        
    Ok(())
}