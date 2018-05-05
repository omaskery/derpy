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
mod error;

use std::collections::BTreeMap;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use strfmt::Format;

use dependency::Dependency;
use error::DerpyError;

const VCS_INFO_DIR: &str = "vcs_info/";
const DEPENDENCY_DIR: &str = "deps/";
const CONFIG_FILE: &str = "derpy.json";
const CONFIG_LOCK_FILE: &str = "derpy.lock.json";

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

type VcsCommand = Vec<String>;
type VcsCommandList = Vec<VcsCommand>;

fn expand_vcs_command(cmd: &VcsCommand, macros: &std::collections::HashMap<String, String>) -> Result<VcsCommand, DerpyError> {
    let mut result = VcsCommand::new();
    for token in cmd.iter() {
        result.push(match token.format(&macros) {
            Ok(formatted) => formatted,
            Err(e) => return Err(DerpyError::MacroExpansionFailure {
                source_text: token.clone(),
                macros: macros.clone(),
                error: e,
            }),
        });
    }
    Ok(result)
}

fn expand_vcs_command_list(list: &VcsCommandList, macros: &std::collections::HashMap<String, String>) -> Result<VcsCommandList, DerpyError> {
    let mut result = VcsCommandList::new();
    for cmd in list.iter() {
        result.push(expand_vcs_command(cmd, macros)?);
    }
    Ok(result)
}

#[derive(Serialize, Deserialize)]
struct VcsInfo {
    name: String,
    get_version: VcsCommand,
    default_version: String,
    acquire: VcsCommandList,
    checkout: VcsCommandList,
    upgrade: VcsCommandList,
    get_version_of: VcsCommand,
}

impl VcsInfo {
    fn get_name(&self) -> &str { &self.name }

    fn get_version(&self, log: &Log) -> Result<String, DerpyError> {
        let (stdout, _) = Self::run_cmd(log, &self.get_version)?;
        Ok(stdout.trim().into())
    }

    fn get_default_version(&self) -> &str { &self.default_version }

    fn acquire(&self, log: &Log, dependency: &Dependency) -> Result<(), DerpyError> {
        let cmd = expand_vcs_command_list(&self.acquire, &dependency.build_macro_map())?;
        do_in_dir(log, &dependency.target, || Self::run_cmd_sequence(log, &cmd))?;
        Ok(())
    }

    fn checkout(&self, log: &Log, dependency: &Dependency, at_version: &str) -> Result<(), DerpyError> {
        let mut macros = dependency.build_macro_map();
        macros.insert("DEP_VERSION".into(), at_version.into());
        let cmd = expand_vcs_command_list(&self.checkout, &macros)?;
        do_in_dir(log, dependency.get_full_path(), || Self::run_cmd_sequence(log, &cmd))?;
        Ok(())
    }

    fn upgrade(&self, log: &Log, dependency: &Dependency) -> Result<(), DerpyError> {
        let cmd = expand_vcs_command_list(&self.upgrade, &dependency.build_macro_map())?;
        do_in_dir(log, dependency.get_full_path(), || Self::run_cmd_sequence(log, &cmd))
    }

    fn get_version_of(&self, log: &Log, dependency: &Dependency) -> Result<String, DerpyError> {
        let cmd = expand_vcs_command(&self.get_version_of, &dependency.build_macro_map())?;
        let (stdout, _) = do_in_dir(log, dependency.get_full_path(), || Self::run_cmd(log, &cmd))?;
        Ok(stdout.trim().into())
    }

    fn run_cmd(log: &Log, cmd: &VcsCommand) -> Result<(String, String), DerpyError> {
        log.info(format!("running command: {:?}", cmd));
        let p = subprocess::Popen::create(cmd, subprocess::PopenConfig {
            stdout: subprocess::Redirection::Pipe,
            stderr: subprocess::Redirection::Pipe,
            ..Default::default()
        });
        let mut p = match p {
            Ok(p) => p,
            Err(e) => return Err(DerpyError::SubprocessError {
                cmd: cmd.clone(),
                error: e,
            }),
        };

        let (stdout, stderr) = match p.communicate(None) {
            Ok(result) => result,
            Err(e) => return Err(DerpyError::SubprocessError {
                cmd: cmd.clone(),
                error: e,
            }),
        };
        let (stdout, stderr) = (stdout.unwrap(), stderr.unwrap());

        let return_code = match p.wait() {
            Ok(result) => result,
            Err(e) => return Err(DerpyError::SubprocessError {
                cmd: cmd.clone(),
                error: e,
            }),
        };

        if return_code.success() == false {
            return Err(DerpyError::VcsCommandFailed { cmd: cmd.clone(), return_code, stdout, stderr }.into());
        }

        Ok((stdout, stderr))
    }

    fn run_cmd_sequence(log: &Log, sequence: &VcsCommandList) -> Result<(), DerpyError> {
        for cmd in sequence.iter() {
            let _output = Self::run_cmd(log, cmd)?;
        }
        Ok(())
    }
}

#[derive(Copy, Clone, Debug)]
enum Verbosity {
    None,
    Info,
    Verbose,
}

struct Log {
    verbosity: Verbosity,
}

impl Log {
    fn log(&self, verbosity: Verbosity, text: String) {
        if self.verbosity as usize >= verbosity as usize {
            println!("{}", text);
        }
    }

    fn verbose(&self, text: String) {
        self.log(Verbosity::Verbose, text)
    }

    fn info(&self, text: String) {
        self.log(Verbosity::Info, text)
    }
}

struct CommandContext {
    path: PathBuf,
    log: Log,
}

fn do_in_dir<T, P: AsRef<Path> + std::fmt::Debug, F: FnOnce() -> Result<T, DerpyError>>(log: &Log, path: P, f: F) -> Result<T, DerpyError> {
    let initial_dir = match std::env::current_dir() {
        Ok(dir) => dir,
        Err(e) => return Err(DerpyError::UnableToDetermineCurrentDir {
            error: e,
        }),
    };
    log.verbose(format!("entering dir {:?} -> {:?}", &initial_dir, &path));
    if let Err(e) = std::env::set_current_dir(&path) {
        return Err(DerpyError::UnableToChangeDir {
            error: e,
        });
    }
    let result = f()?;
    log.verbose(format!("leaving dir {:?} -> {:?}", &path, &initial_dir));
    if let Err(e) = std::env::set_current_dir(&initial_dir) {
        return Err(DerpyError::UnableToChangeDir {
            error: e,
        });
    }
    Ok(result)
}

fn install_dir() -> Result<PathBuf, DerpyError> {
    let dir = if cfg!(debug_assertions) {
        match std::env::current_dir() {
            Ok(dir) => dir,
            Err(e) => return Err(DerpyError::UnableToDetermineCurrentDir {
                error: e,
            }),
        }
    } else {
        match std::env::current_exe() {
            Ok(dir) => dir,
            Err(e) => return Err(DerpyError::UnableToDetermineCurrentExePath {
                error: e,
            })
        }
    };

    Ok(dir)
}

fn ensure_dir<P: AsRef<Path>>(path: P) -> Result<(), DerpyError> {
    if let Err(e) = std::fs::create_dir_all(path) {
        return Err(DerpyError::FailedToCreateDirectory {
            error: e,
        })
    }
    Ok(())
}

fn load_vcs_info(vcs_name: &str) -> Result<Option<VcsInfo>, DerpyError> {
    let full_path = install_dir()?
        .join(VCS_INFO_DIR)
        .join(vcs_name)
        .with_extension("json");

    if full_path.is_file() {
        let mut contents = String::new();
        let mut file = match std::fs::File::open(full_path) {
            Ok(file) => file,
            Err(e) => return Err(DerpyError::UnableToOpenVcsInfo {
                error: e,
            }),
        };
        if let Err(e) = file.read_to_string(&mut contents) {
            return Err(DerpyError::UnableToReadVcsInfo {
                error: e,
            });
        }
        match serde_json::from_str(&contents) {
            Ok(info) => Ok(Some(info)),
            Err(e) => Err(DerpyError::UnableToDecodeVcsInfo {
                error: e,
            }),
        }
    } else {
        Ok(None)
    }
}

fn load_config<P: AsRef<Path>>(path: P) -> Result<DerpyFile, DerpyError> {
    let mut contents = String::new();
    let mut file = match std::fs::File::open(path) {
        Ok(file) => file,
        Err(e) => return Err(DerpyError::UnableToOpenConfig {
            error: e,
        }),
    };
    if let Err(e) = file.read_to_string(&mut contents) {
        return Err(DerpyError::UnableToReadConfig {
            error: e,
        });
    }
    match serde_json::from_str(&contents) {
        Ok(config) => Ok(config),
        Err(e) => Err(DerpyError::UnableToDecodeConfig {
            error: e,
        })
    }
}

fn save_config<P: AsRef<Path>>(config: &DerpyFile, path: P) -> Result<(), DerpyError> {
    let file = std::fs::OpenOptions::new()
        .write(true)
        .truncate(true)
        .create(true)
        .open(path);
    let mut file = match file {
        Ok(file) => file,
        Err(e) => return Err(DerpyError::UnableToCreateConfig {
            error: e,
        }),
    };
    let contents = match serde_json::to_string_pretty(config) {
        Ok(contents) => contents,
        Err(e) => return Err(DerpyError::UnableToEncodeConfig {
            error: e,
        }),
    };
    match file.write_all(contents.as_bytes()) {
        Ok(_) => Ok(()),
        Err(e) => Err(DerpyError::UnableToWriteConfig {
            error: e,
        }),
    }
}

fn parse_option_key_value(text: &str) -> Result<(String, String), String> {
    let parts = text.splitn(2, ":")
        .collect::<Vec<_>>();

    if parts.len() == 2 {
        Ok((parts[0].into(), parts[1].into()))
    } else {
        Err("key value pair must be two strings separated by a ':' character".into())
    }
}

fn validate_option_key_value(text: String) -> Result<(), String> {
    match parse_option_key_value(&text) {
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

fn determine_cwd(override_path: Option<&str>) -> Result<PathBuf, DerpyError> {
    let path = match override_path {
        Some(path) => {
            let path: PathBuf = path.into();
            if path.is_absolute() {
                path
            } else {
                match std::env::current_dir() {
                    Ok(dir) => dir.join(path),
                    Err(e) => return Err(DerpyError::UnableToDetermineCurrentDir {
                        error: e,
                    }),
                }
            }
        },
        _ => match std::env::current_dir() {
            Ok(dir) => dir,
            Err(e) => return Err(DerpyError::UnableToDetermineCurrentDir {
                error: e,
            }),
        },
    };

    Ok(path)
}

fn run_cli(matches: clap::ArgMatches) -> Result<(), DerpyError> {
    let context = CommandContext {
        path: determine_cwd(matches.value_of("path"))?,
        log: Log {
            verbosity: match matches.occurrences_of("verbosity") {
                0 => Verbosity::None,
                1 => Verbosity::Info,
                2 => Verbosity::Verbose,
                _ => Verbosity::Verbose,
            },
        },
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

fn cmd_init(context: CommandContext, _matches: &clap::ArgMatches) -> Result<(), DerpyError> {
    let config_path = context.path.join(CONFIG_FILE);
    if config_path.is_file() {
        return Err(DerpyError::AlreadyInitialised.into());
    }
    Ok(save_config(&Default::default(), config_path)?)
}

fn cmd_add(context: CommandContext, matches: &clap::ArgMatches) -> Result<(), DerpyError> {
    let vcs = matches.value_of("vcs").unwrap().to_string();
    let name = matches.value_of("name").unwrap().to_string();
    let url = matches.value_of("url").unwrap().to_string();
    let version = matches.value_of("version");
    let target = matches.value_of("target").unwrap_or(DEPENDENCY_DIR).to_string();

    let vcs_info = match load_vcs_info(&vcs)? {
        Some(info) => info,
        None => return Err(DerpyError::UnknownVcs { name: vcs.into() }.into()),
    };

    let _vcs_version = match vcs_info.get_version(&context.log) {
        Ok(version) => {
            context.log.verbose(format!("detected {} at version '{}'", vcs_info.get_name(), version));
            Some(version)
        },
        Err(_) => {
            println!("warning: unable to determine version of {}, is it installed?", vcs_info.get_name());
            None
        },
    };

    let version = version.map_or_else(|| vcs_info.get_default_version().into(), |v| v.into() );

    let options = match matches.values_of("options") {
        Some(values) => {
            values.map(|option| parse_option_key_value(option).unwrap())
                .collect::<BTreeMap<_,_>>()
        },
        _ => BTreeMap::new(),
    };

    let dependency = Dependency {
        name: name.clone(),
        vcs,
        url,
        version,
        target,
        options,
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

enum AcquireOutcome {
    Acquired {
        at_version: String,
    },
    Restored {
        from_version: String,
        to_version: String,
    },
    UpgradedTo {
        from_version: String,
        to_version: String,
    },
    NoChange {
        current_version: String,
    },
    Ignored {
        at_version: String,
    },
}

#[derive(Debug)]
pub enum AcquireMode {
    Acquire,
    LockTo {
        version: String,
    },
    Upgrade,
}

fn acquire(context: &CommandContext, dep: &Dependency, acquire_mode: AcquireMode) -> Result<AcquireOutcome, DerpyError> {
    let vcs = match load_vcs_info(&dep.vcs)? {
        Some(vcs) => vcs,
        None => return Err(DerpyError::UnknownVcs { name: dep.vcs.clone() }.into()),
    };

    ensure_dir(&dep.target)?;

    let current_version = if dep.get_full_path().is_dir() {
        Some(vcs.get_version_of(&context.log, dep)?)
    } else {
        None
    };

    if let Some(version) = current_version {
        match acquire_mode {
            AcquireMode::Acquire => {
                Ok(AcquireOutcome::Ignored {
                    at_version: version,
                })
            },
            AcquireMode::LockTo { version: locked_version } => {
                if version != locked_version {
                    vcs.checkout(&context.log, dep, &locked_version)?;
                    Ok(AcquireOutcome::Restored {
                        to_version: locked_version,
                        from_version: version,
                    })
                } else {
                    Ok(AcquireOutcome::NoChange {
                        current_version: version,
                    })
                }
            },
            AcquireMode::Upgrade => {
                vcs.upgrade(&context.log, dep)?;

                let new_version = vcs.get_version_of(&context.log, dep)?;
                if new_version != version {
                    Ok(AcquireOutcome::UpgradedTo {
                        from_version: version,
                        to_version: new_version,
                    })
                } else {
                    Ok(AcquireOutcome::NoChange {
                        current_version: version,
                    })
                }
            },
        }
    } else {
        if let AcquireMode::Acquire = acquire_mode {
            vcs.acquire(&context.log, dep)?;

            Ok(AcquireOutcome::Acquired {
                at_version: vcs.get_version_of(&context.log, dep)?,
            })
        } else {
            Err(DerpyError::NonsenseAcquireMode {
                acquire_mode,
            }.into())
        }
    }
}

fn cmd_acquire(context: CommandContext, _matches: &clap::ArgMatches) -> Result<(), DerpyError> {
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
        let new_lock_version = match acquire(&context, dep, acquire_mode)? {
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

fn cmd_upgrade(context: CommandContext, matches: &clap::ArgMatches) -> Result<(), DerpyError> {
    let config_path = context.path.join(CONFIG_FILE);
    let config = load_config(&config_path)?;

    let lock_path = context.path.join(CONFIG_LOCK_FILE);
    let mut lock = if lock_path.is_file() {
        load_config(&lock_path)?
    } else {
        DerpyFile::default()
    };
    let mut lock_file_updated = false;

    let to_upgrade = if matches.is_present("all") {
        config.dependencies.clone()
    } else {
        let to_upgrade_names = matches.values_of("dependencies").unwrap()
            .map(|s| s.into())
            .collect::<Vec<_>>();
        config.dependencies.iter()
            .filter(|pair| to_upgrade_names.contains(pair.0))
            .map(|pair| (pair.0.clone(), pair.1.clone()))
            .collect::<BTreeMap<String, Dependency>>()
    };

    for (name, dep) in to_upgrade {
        let new_lock_version = match acquire(&context, &dep, AcquireMode::Upgrade)? {
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

