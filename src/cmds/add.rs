use consts::{CONFIG_FILE, DEPENDENCY_DIR};
use derpyfile::{load_config, save_config};
use arg_utils::parse_option_key_value;
use std::collections::BTreeMap;
use dependency::Dependency;
use cmds::CommandContext;
use vcs::load_vcs_info;
use error::DerpyError;

pub fn cli_add(context: CommandContext) -> Result<(), DerpyError> {
    let vcs = context.matches.value_of("vcs").unwrap().to_string();
    let name = context.matches.value_of("name").unwrap().to_string();
    let url = context.matches.value_of("url").unwrap().to_string();
    let version = context.matches.value_of("version");
    let target = context.matches.value_of("target").unwrap_or(DEPENDENCY_DIR).to_string();

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

    let options = match context.matches.values_of("options") {
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
