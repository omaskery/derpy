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

use std::collections::BTreeMap;

use derpyfile::{DerpyFile, load_config, save_config};
use acquire::{acquire, AcquireMode, AcquireOutcome};
use consts::{CONFIG_LOCK_FILE, CONFIG_FILE};
use arg_utils::validate_option_key_value;
use dependency::Dependency;
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
                "init" => cmds::init(context),
                "add" => cmds::add(context),
                "acquire" => cmd_acquire(context),
                "upgrade" => cmd_upgrade(context),
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

fn cmd_acquire(context: cmds::CommandContext) -> Result<(), DerpyError> {
    let config_path = context.path.join(CONFIG_FILE);
    let config = load_config(&config_path)?;

    let lock_path = context.path.join(CONFIG_LOCK_FILE);
    let mut lock = if lock_path.is_file() {
        load_config(&lock_path)?
    } else {
        DerpyFile::default()
    };
    let mut lock_file_updated = false;

    for (name, dep) in config.dependencies.iter() {
        let acquire_mode = match lock.dependencies.get(name).map(|d| d.version.clone()) {
            Some(version) => AcquireMode::LockTo { version },
            _ => AcquireMode::Acquire,
        };
        let new_lock_version = match acquire(&context.log, dep, acquire_mode)? {
            AcquireOutcome::Acquired { at_version } => {
                println!("- acquired '{}' at version {}", name, at_version);
                Some(at_version)
            },
            AcquireOutcome::Restored { from_version, to_version } => {
                println!("- restored '{}' to {} from {}", name, to_version, from_version);
                None
            },
            AcquireOutcome::UpgradedTo { from_version, to_version } => {
                println!("- upgraded '{}' to {} from {}", name, to_version, from_version);
                Some(to_version)
            },
            AcquireOutcome::NoChange { current_version } => {
                println!("- '{}' up to date at version {}", name, current_version);
                None
            },
            AcquireOutcome::Ignored { at_version } => {
                println!("- warning: ignored '{}' - left at version {}", name, at_version);
                println!("  (dependency {} present but has no lock file entry)", name);
                None
            },
        };

        if let Some(version) = new_lock_version {
            let mut dependency = dep.clone();
            lock_file_updated = true;
            dependency.version = version;
            lock.dependencies.insert(dep.name.clone(), dependency);
        }
    }

    if lock_file_updated {
        save_config(&lock, &lock_path)?;
        println!("lock file updated");
    }

    Ok(())
}

fn cmd_upgrade(context: cmds::CommandContext) -> Result<(), DerpyError> {
    let config_path = context.path.join(CONFIG_FILE);
    let config = load_config(&config_path)?;

    let lock_path = context.path.join(CONFIG_LOCK_FILE);
    let mut lock = if lock_path.is_file() {
        load_config(&lock_path)?
    } else {
        DerpyFile::default()
    };
    let mut lock_file_updated = false;

    let to_upgrade = if context.matches.is_present("all") {
        config.dependencies.clone()
    } else {
        let to_upgrade_names = context.matches.values_of("dependencies").unwrap()
            .map(|s| s.into())
            .collect::<Vec<_>>();
        config.dependencies.iter()
            .filter(|pair| to_upgrade_names.contains(pair.0))
            .map(|pair| (pair.0.clone(), pair.1.clone()))
            .collect::<BTreeMap<String, Dependency>>()
    };

    for (name, dep) in to_upgrade {
        let new_lock_version = match acquire(&context.log, &dep, AcquireMode::Upgrade)? {
            AcquireOutcome::Acquired { at_version } => {
                println!("- acquired '{}' at version {}", name, at_version);
                Some(at_version)
            },
            AcquireOutcome::Restored { from_version, to_version } => {
                println!("- restored '{}' to {} from {}", name, to_version, from_version);
                None
            },
            AcquireOutcome::UpgradedTo { from_version, to_version } => {
                println!("- upgraded '{}' to {} from {}", name, to_version, from_version);
                Some(to_version)
            },
            AcquireOutcome::NoChange { current_version } => {
                println!("- '{}' up to date at version {}", name, current_version);
                None
            },
            AcquireOutcome::Ignored { at_version } => {
                println!("- warning: ignored '{}' - left at version {}", name, at_version);
                println!("  (dependency {} present but has no lock file entry)", name);
                None
            },
        };

        if let Some(version) = new_lock_version {
            let mut dependency = dep.clone();
            lock_file_updated = true;
            dependency.version = version;
            lock.dependencies.insert(dep.name.clone(), dependency);
        }
    }

    if lock_file_updated {
        save_config(&lock, &lock_path)?;
        println!("lock file updated");
    }

    Ok(())
}

