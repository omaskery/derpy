use subprocess::{ExitStatus, PopenError};
use std::collections::HashMap;
use strfmt::FmtError;
use serde_json;
use std::io;

use acquire::AcquireMode;
use vcs::VcsCommand;

#[derive(Fail, Debug)]
pub enum DerpyError {
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
    #[fail(display = "vcs command {:?} returned {:?}, stdout='{}', stderr='{}'", cmd, return_code, stdout, stderr)]
    VcsCommandFailed {
        cmd: VcsCommand,
        return_code: ExitStatus,
        stdout: String,
        stderr: String,
    },
    #[fail(display = "acquire mode for dependency '{}' set to {:?} but no repository found", dependency, acquire_mode)]
    NonsenseAcquireMode {
        dependency: String,
        acquire_mode: AcquireMode,
    },
    #[fail(display = "failed to expand macros: {} (source text: {}, macros: {:?})", error, source_text, macros)]
    MacroExpansionFailure {
        source_text: String,
        macros: HashMap<String, String>,
        error: FmtError,
    },
    #[fail(display = "error invoking subprocess: {} (command: {:?})", error, cmd)]
    SubprocessError {
        cmd: VcsCommand,
        error: PopenError,
    },
    #[fail(display = "unable to determine current directory: {:?}", error)]
    UnableToDetermineCurrentDir {
        error: io::Error,
    },
    #[fail(display = "unable to change current directory: {:?}", error)]
    UnableToChangeDir {
        error: io::Error,
    },
    #[fail(display = "unable to determine current exe path: {:?}", error)]
    UnableToDetermineCurrentExePath {
        error: io::Error,
    },
    #[fail(display = "failed to create directory: {:?}", error)]
    FailedToCreateDirectory {
        error: io::Error,
    },
    #[fail(display = "unable to open VCS info file: {:?}", error)]
    UnableToOpenVcsInfo {
        error: io::Error,
    },
    #[fail(display = "unable to read VCS info file: {:?}", error)]
    UnableToReadVcsInfo {
        error: io::Error,
    },
    #[fail(display = "unable to decode VCS info file: {:?}", error)]
    UnableToDecodeVcsInfo {
        error: serde_json::Error,
    },
    #[fail(display = "unable to open config file: {:?}", error)]
    UnableToOpenConfig {
        error: io::Error,
    },
    #[fail(display = "unable to read config file: {:?}", error)]
    UnableToReadConfig {
        error: io::Error,
    },
    #[fail(display = "unable to decode config file: {:?}", error)]
    UnableToDecodeConfig {
        error: serde_json::Error,
    },
    #[fail(display = "unable to create config file: {:?}", error)]
    UnableToCreateConfig {
        error: io::Error,
    },
    #[fail(display = "unable to encode config file: {:?}", error)]
    UnableToEncodeConfig {
        error: serde_json::Error,
    },
    #[fail(display = "unable to write to config file: {:?}", error)]
    UnableToWriteConfig {
        error: io::Error,
    },
}
