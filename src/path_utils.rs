use std::env::{current_dir, current_exe};
use std::path::{Path, PathBuf};
use std::fs::create_dir_all;
use error::DerpyError;

pub fn install_dir() -> Result<PathBuf, DerpyError> {
    let dir = if cfg!(debug_assertions) {
        match current_dir() {
            Ok(dir) => dir,
            Err(e) => return Err(DerpyError::UnableToDetermineCurrentDir {
                error: e,
            }),
        }
    } else {
        match current_exe() {
            Ok(dir) => dir,
            Err(e) => return Err(DerpyError::UnableToDetermineCurrentExePath {
                error: e,
            })
        }
    };

    Ok(dir)
}

pub fn ensure_dir<P: AsRef<Path>>(path: P) -> Result<(), DerpyError> {
    if let Err(e) = create_dir_all(path) {
        return Err(DerpyError::FailedToCreateDirectory {
            error: e,
        })
    }
    Ok(())
}

pub fn determine_cwd(override_path: Option<&str>) -> Result<PathBuf, DerpyError> {
    let path = match override_path {
        Some(path) => {
            let path: PathBuf = path.into();
            if path.is_absolute() {
                path
            } else {
                match current_dir() {
                    Ok(dir) => dir.join(path),
                    Err(e) => return Err(DerpyError::UnableToDetermineCurrentDir {
                        error: e,
                    }),
                }
            }
        },
        _ => match current_dir() {
            Ok(dir) => dir,
            Err(e) => return Err(DerpyError::UnableToDetermineCurrentDir {
                error: e,
            }),
        },
    };

    Ok(path)
}

