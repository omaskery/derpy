extern crate serde;
extern crate serde_json;
extern crate failure;
extern crate clap;
extern crate subprocess;
extern crate strfmt;

#[macro_use]
extern crate serde_derive;
#[macro_use]
extern crate failure_derive;

mod dependency;
mod path_utils;
mod arg_utils;
mod derpyfile;
mod acquire;
mod consts;
mod error;
mod cmds;
mod vcs;
mod log;

use arg_utils::validate_option_key_value;
use error::DerpyError;

fn main() {
    use clap::{Arg, SubCommand};

    let matches = clap::App::new("derpy")
        .version("0.0.1")
        .author("Oliver Maskery <omaskery@googlemail.com>")
        .about("derpy is a simple language & vcs agnostic derpendency manager :)")
        .arg(Arg::with_name("path")
            .short("p")
            .long("path")
            .help("path to treat as current working directory")
            .takes_value(true))
        .arg(Arg::with_name("verbosity")
            .short("v")
            .long("verbose")
            .help("increases verbosity of output")
            .multiple(true))
        .subcommand(SubCommand::with_name("init")
            .about("initialises derpy in the current directory"))
        .subcommand(SubCommand::with_name("add")
            .about("adds a dependency to the current project")
            .arg(Arg::with_name("vcs")
                .takes_value(true)
                .required(true)
                .help("the version control system the dependency uses"))
            .arg(Arg::with_name("name")
                .takes_value(true)
                .required(true)
                .help("the name of the dependency (also folder name in target directory)"))
            .arg(Arg::with_name("url")
                .takes_value(true)
                .required(true)
                .help("url the dependency lives at"))
            .arg(Arg::with_name("version")
                .long("version")
                .takes_value(true)
                .help("the version of the dependency to fetch (branch, commit, revision, etc.)"))
            .arg(Arg::with_name("options")
                .long("option")
                .takes_value(true)
                .multiple(true)
                .validator(validate_option_key_value)
                .help("specifies KEY:VALUE options to associate with the dependency")))
        .subcommand(SubCommand::with_name("acquire")
            .about("ensures all required dependencies are fetched to the current (locked) version"))
        .subcommand(SubCommand::with_name("upgrade")
            .about("like acquire but ignores the lockfile, allowing dependencies to update")
            .group(clap::ArgGroup::with_name("deps")
                .required(true)
                .args(&["all", "dependencies"]))
            .arg(Arg::with_name("all")
                .long("all")
                .help("indicates that all dependencies should be upgraded"))
            .arg(Arg::with_name("dependencies")
                .multiple(true)
                .help("specifies dependencies to upgrade")))
        .get_matches();

    match run_cli(matches) {
        Err(e) => println!("error: {}", e),
        _ => {},
    }
}

fn run_cli(matches: clap::ArgMatches) -> Result<(), DerpyError> {
    let cmd_name = matches.subcommand_name().map(|s| s.to_string());

    let context = cmds::CommandContext::from_args(matches)?;

    match cmd_name {
        Some(name) => {
            match name.as_str() {
                "init" => cmds::cli_init(context),
                "add" => cmds::cli_add(context),
                "acquire" => cmds::cli_acquire(context),
                "upgrade" => cmds::cli_upgrade(context),
                _ => unreachable!(),
            }
        },
        None => {
            Err(DerpyError::InvalidArguments {
                reason: "no subcommand was used".into(),
            }.into())
        },
    }
}
