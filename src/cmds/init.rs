use derpyfile::save_config;
use cmds::CommandContext;
use consts::CONFIG_FILE;
use error::DerpyError;

pub fn init(context: CommandContext) -> Result<(), DerpyError> {
    let config_path = context.path.join(CONFIG_FILE);
    if config_path.is_file() {
        return Err(DerpyError::AlreadyInitialised.into());
    }
    Ok(save_config(&Default::default(), config_path)?)
}

