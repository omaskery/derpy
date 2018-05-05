use derpyfile::{DerpyFile, load_config, save_config};
use acquire::{acquire, AcquireMode, AcquireOutcome};
use consts::{CONFIG_FILE, CONFIG_LOCK_FILE};
use cmds::CommandContext;
use error::DerpyError;

pub fn cli_acquire(context: CommandContext) -> Result<(), DerpyError> {
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
