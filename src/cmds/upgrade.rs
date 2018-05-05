use derpyfile::{DerpyFile, load_config, save_config};
use acquire::{acquire, AcquireMode, AcquireOutcome};
use consts::{CONFIG_FILE, CONFIG_LOCK_FILE};
use std::collections::BTreeMap;
use dependency::Dependency;
use cmds::CommandContext;
use error::DerpyError;

pub fn cli_upgrade(context: CommandContext) -> Result<(), DerpyError> {
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

