extern crate serde;
extern crate serde_json;
extern crate failure;
extern crate clap;

#[macro_use]
extern crate serde_derive;
#[macro_use]
extern crate failure_derive;

use std::collections::BTreeMap;
use std::io::{Read, Write};
use failure::Error;

const VCS_INFO_DIR: &str = "vcs_info/";
const DEPENDENCY_DIR: &str = "deps/";
const CONFIG_FILE: &str = "derpy.json";
const CONFIG_LOCK_FILE: &str = "derpy.lock.json";

#[derive(Fail, Debug)]
enum DerpyError {
    #[fail(display = "invalid arguments: {}", reason)]
    InvalidArguments {
        reason: String,
    },
    #[fail(display = "already initialised in this directory")]
    AlreadyInitialised,
    #[fail(display = "dependency '{}' already exists", name)]
    DependencyAlreadyExists {
        name: String,
    },
    #[fail(display = "version control system '{}' unknown", name)]
    UnknownVcs {
        name: String,
    },
}

#[derive(Serialize, Deserialize)]
struct Dependency {
    name: String,
    vcs: String,
    url: String,
    version: String,
    target: String,
}

#[derive(Serialize, Deserialize)]
struct DerpyFile {
    dependencies: BTreeMap<String, Dependency>,
}

impl Default for DerpyFile {
    fn default() -> Self {
        Self {
            dependencies: BTreeMap::new(),
        }
    }
}

#[derive(Serialize, Deserialize)]
struct VcsInfo {
    name: String,
    get_version: String,
    default_version: String,
    acquire: Vec<String>,
    checkout: Vec<String>,
    upgrade: Vec<String>,
    get_version_of: String,
}

struct CommandContext {
    path: std::path::PathBuf,
    verbosity: u64,
}

fn install_dir() -> Result<std::path::PathBuf, Error> {
    let dir = if cfg!(debug_assertions) {
        std::env::current_dir()?
    } else {
        std::env::current_exe()?
    };

    Ok(dir)
}

fn load_vcs_info(vcs_name: &str) -> Result<Option<VcsInfo>, Error> {
    let full_path = install_dir()?
        .join(VCS_INFO_DIR)
        .join(vcs_name)
        .with_extension("json");

    if full_path.is_file() {
        let mut contents = String::new();
        let mut file = std::fs::File::open(full_path)?;
        file.read_to_string(&mut contents)?;
        Ok(Some(serde_json::from_str(&contents)?))
    } else {
        Ok(None)
    }
}

fn load_config<P: AsRef<std::path::Path>>(path: P) -> Result<DerpyFile, Error> {
    let mut contents = String::new();
    let mut file = std::fs::File::open(path)?;
    file.read_to_string(&mut contents)?;
    Ok(serde_json::from_str(&contents)?)
}

fn save_config<P: AsRef<std::path::Path>>(config: &DerpyFile, path: P) -> Result<(), Error> {
    let mut file = std::fs::OpenOptions::new()
        .write(true)
        .truncate(true)
        .create(true)
        .open(path)?;
    let contents = serde_json::to_string(config)?;
    Ok(file.write_all(contents.as_bytes())?)
}

fn parse_option_key_value(text: String) -> Result<(String, String), String> {
    let parts = text.splitn(2, ":")
        .collect::<Vec<_>>();

    if parts.len() == 2 {
        Ok((parts[0].into(), parts[1].into()))
    } else {
        Err("key value pair must be two strings separated by a ':' character".into())
    }
}

fn validate_option_key_value(text: String) -> Result<(), String> {
    match parse_option_key_value(text) {
        Ok(_) => Ok(()),
        Err(e) => Err(e),
    }
}

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
            .arg(Arg::with_name("all")
                .long("all")
                .conflicts_with("dependencies")
                .help("indicates that all dependencies should be upgraded"))
            .arg(Arg::with_name("dependencies")
                .multiple(true)
                .conflicts_with("all")
                .help("specifies dependencies to upgrade")))
        .get_matches();

    match run_cli(matches) {
        Err(e) => println!("error: {}", e),
        _ => {},
    }
}

fn determine_cwd(override_path: Option<&str>) -> Result<std::path::PathBuf, Error> {
    let path = match override_path {
        Some(path) => {
            let path: std::path::PathBuf = path.into();
            if path.is_absolute() {
                path
            } else {
                std::env::current_dir()?.join(path)
            }
        },
        _ => std::env::current_dir()?,
    };

    Ok(path)
}

fn run_cli(matches: clap::ArgMatches) -> Result<(), Error> {
    let context = CommandContext {
        path: determine_cwd(matches.value_of("path"))?,
        verbosity: matches.occurrences_of("verbosity"),
    };

    match matches.subcommand() {
        ("init", Some(init_matches)) => cmd_init(context, init_matches),
        ("add", Some(add_matches)) => cmd_add(context, add_matches),
        ("acquire", Some(acquire_matches)) => cmd_acquire(context, acquire_matches),
        ("upgrade", Some(upgrade_matches)) => cmd_upgrade(context, upgrade_matches),
        ("", None) => {
            Err(DerpyError::InvalidArguments {
                reason: "no subcommand was used".into(),
            }.into())
        },
        _ => unreachable!(),
    }
}

fn cmd_init(context: CommandContext, matches: &clap::ArgMatches) -> Result<(), Error> {
    let config_path = context.path.join(CONFIG_FILE);
    if config_path.is_file() {
        return Err(DerpyError::AlreadyInitialised.into());
    }
    Ok(save_config(&Default::default(), config_path)?)
}

fn cmd_add(context: CommandContext, matches: &clap::ArgMatches) -> Result<(), Error> {
    let vcs = matches.value_of("vcs").unwrap().to_string();
    let name = matches.value_of("name").unwrap().to_string();
    let url = matches.value_of("url").unwrap().to_string();
    let version = matches.value_of("version");
    let target = matches.value_of("target").unwrap_or(DEPENDENCY_DIR).to_string();

    let vcs_info = match load_vcs_info(&vcs)? {
        Some(info) => info,
        None => return Err(DerpyError::UnknownVcs { name: vcs.into() }.into()),
    };

    let version = version.map_or_else(|| vcs_info.default_version, |v| v.into() );

    let dependency = Dependency {
        name: name.clone(),
        vcs,
        url,
        version,
        target,
    };

    let config_path = context.path.join(CONFIG_FILE);
    let mut config = load_config(&config_path)?;

    if config.dependencies.contains_key(&name) {
        return Err(DerpyError::DependencyAlreadyExists { name: name.into() }.into());
    }

    config.dependencies.insert(name, dependency);

    save_config(&config, &config_path)?;

    Ok(())
}

fn cmd_acquire(context: CommandContext, matches: &clap::ArgMatches) -> Result<(), Error> {
    println!("acquire: {:?}", matches);
    Ok(())
}

fn cmd_upgrade(context: CommandContext, matches: &clap::ArgMatches) -> Result<(), Error> {
    println!("upgrade: {:?}", matches);
    Ok(())
}
