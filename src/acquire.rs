use dependency::Dependency;
use path_utils::ensure_dir;
use vcs::load_vcs_info;
use error::DerpyError;
use log::Log;

pub enum AcquireOutcome {
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

pub fn acquire(log: &Log, dep: &Dependency, acquire_mode: AcquireMode) -> Result<AcquireOutcome, DerpyError> {
    let vcs = match load_vcs_info(&dep.vcs)? {
        Some(vcs) => vcs,
        None => return Err(DerpyError::UnknownVcs { name: dep.vcs.clone() }.into()),
    };

    ensure_dir(&dep.target)?;

    let current_version = if dep.get_full_path().is_dir() {
        Some(vcs.get_version_of(&log, dep)?)
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
                    vcs.checkout(&log, dep, &locked_version)?;
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
                vcs.upgrade(&log, dep)?;

                let new_version = vcs.get_version_of(&log, dep)?;
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
            vcs.acquire(&log, dep)?;

            Ok(AcquireOutcome::Acquired {
                at_version: vcs.get_version_of(&log, dep)?,
            })
        } else {
            Err(DerpyError::NonsenseAcquireMode {
                acquire_mode,
            }.into())
        }
    }
}

