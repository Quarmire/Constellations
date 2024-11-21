use clap::{Arg, Command};
use dotenv::dotenv;
use celestia::{commands, settings};

pub fn main() -> anyhow::Result<()> {
    dotenv().ok();

    // Mobile version: Citadel CLI
    let mut command = Command::new("Celestia CLI")
        .arg(
            Arg::new("config")
                .short('c')
                .long("config")
                .help("Configuration file location")
                .default_value("config.json"),
        );

    command = commands::configure(command);

    let matches = command.get_matches();

    let config_location = matches
        .get_one("config")
        .map(|s: &String| s.as_str());

    let settings = settings::Settings::new(config_location, "CELESTIA")?;

    commands::handle(&matches, &settings)?;

    Ok(())
}