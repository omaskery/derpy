use subprocess::{Popen, PopenConfig, Redirection};
use std::env::{current_dir, set_current_dir};
use std::collections::HashMap;
use dependency::Dependency;
use error::DerpyError;
use std::fmt::Debug;
use std::path::Path;
use strfmt::Format;
use std::vec::Vec;
use log::Log;

pub type VcsCommand = Vec<String>;
pub type VcsCommandList = Vec<VcsCommand>;

fn do_in_dir<T, P: AsRef<Path> + Debug, F: FnOnce() -> Result<T, DerpyError>>(log: &Log, path: P, f: F) -> Result<T, DerpyError> {
    let initial_dir = match current_dir() {
        Ok(dir) => dir,
        Err(e) => return Err(DerpyError::UnableToDetermineCurrentDir {
            error: e,
        }),
    };
    log.verbose(format!("entering dir {:?} -> {:?}", &initial_dir, &path));
    if let Err(e) = set_current_dir(&path) {
        return Err(DerpyError::UnableToChangeDir {
            error: e,
        });
    }
    let result = f()?;
    log.verbose(format!("leaving dir {:?} -> {:?}", &path, &initial_dir));
    if let Err(e) = set_current_dir(&initial_dir) {
        return Err(DerpyError::UnableToChangeDir {
            error: e,
        });
    }
    Ok(result)
}

fn expand_vcs_command(cmd: &VcsCommand, macros: &HashMap<String, String>) -> Result<VcsCommand, DerpyError> {
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

fn expand_vcs_command_list(list: &VcsCommandList, macros: &HashMap<String, String>) -> Result<VcsCommandList, DerpyError> {
    let mut result = VcsCommandList::new();
    for cmd in list.iter() {
        result.push(expand_vcs_command(cmd, macros)?);
    }
    Ok(result)
}

#[derive(Serialize, Deserialize)]
pub struct VcsInfo {
    name: String,
    get_version: VcsCommand,
    default_version: String,
    acquire: VcsCommandList,
    checkout: VcsCommandList,
    upgrade: VcsCommandList,
    get_version_of: VcsCommand,
}

impl VcsInfo {
    pub fn get_name(&self) -> &str { &self.name }

    pub fn get_version(&self, log: &Log) -> Result<String, DerpyError> {
        let (stdout, _) = Self::run_cmd(log, &self.get_version)?;
        Ok(stdout.trim().into())
    }

    pub fn get_default_version(&self) -> &str { &self.default_version }

    pub fn acquire(&self, log: &Log, dependency: &Dependency) -> Result<(), DerpyError> {
        let cmd = expand_vcs_command_list(&self.acquire, &dependency.build_macro_map())?;
        do_in_dir(log, &dependency.target, || Self::run_cmd_sequence(log, &cmd))?;
        Ok(())
    }

    pub fn checkout(&self, log: &Log, dependency: &Dependency, at_version: &str) -> Result<(), DerpyError> {
        let mut macros = dependency.build_macro_map();
        macros.insert("DEP_VERSION".into(), at_version.into());
        let cmd = expand_vcs_command_list(&self.checkout, &macros)?;
        do_in_dir(log, dependency.get_full_path(), || Self::run_cmd_sequence(log, &cmd))?;
        Ok(())
    }

    pub fn upgrade(&self, log: &Log, dependency: &Dependency) -> Result<(), DerpyError> {
        let cmd = expand_vcs_command_list(&self.upgrade, &dependency.build_macro_map())?;
        do_in_dir(log, dependency.get_full_path(), || Self::run_cmd_sequence(log, &cmd))
    }

    pub fn get_version_of(&self, log: &Log, dependency: &Dependency) -> Result<String, DerpyError> {
        let cmd = expand_vcs_command(&self.get_version_of, &dependency.build_macro_map())?;
        let (stdout, _) = do_in_dir(log, dependency.get_full_path(), || Self::run_cmd(log, &cmd))?;
        Ok(stdout.trim().into())
    }

    fn run_cmd(log: &Log, cmd: &VcsCommand) -> Result<(String, String), DerpyError> {
        log.info(format!("running command: {:?}", cmd));
        let p = Popen::create(cmd, PopenConfig {
            stdout: Redirection::Pipe,
            stderr: Redirection::Pipe,
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
