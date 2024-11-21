mod test;

use clap::{ArgMatches, Command};

use crate::settings::Settings;

pub fn configure(command: Command) -> Command {
    command
        .subcommand(test::configure())
        .arg_required_else_help(false)
}

pub fn handle(matches: &ArgMatches, settings: &Settings) -> anyhow::Result<()> {
    if let Some((cmd, matches)) = matches.subcommand() {
        match cmd {
            test::COMMAND_NAME => test::handle(matches, settings)?,
            &_ => {}
        }
    }

    Ok(())
}