use path_utils::determine_cwd;
use std::path::PathBuf;
use error::DerpyError;
use clap::ArgMatches;
use log::Log;

mod init;
mod add;
mod acquire;

pub struct CommandContext<'a> {
    pub matches: ArgMatches<'a>,
    pub path: PathBuf,
    pub log: Log,
}

impl<'a> CommandContext<'a> {
    pub fn from_args(matches: ArgMatches<'a>) -> Result<Self, DerpyError> {
        let path = determine_cwd(matches.value_of("path"))?;
        let log = Log::from(matches.occurrences_of("verbosity"));

        Ok(Self {
            matches,
            path,
            log,
        })
    }
}

pub use self::init::cli_init;
pub use self::add::cli_add;
pub use self::acquire::cli_acquire;
