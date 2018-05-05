use std::fs::{File, OpenOptions};
use std::collections::BTreeMap;
use std::io::{Read, Write};
use std::path::Path;
use serde_json;

use dependency::Dependency;
use error::DerpyError;

#[derive(Serialize, Deserialize)]
pub struct DerpyFile {
    pub dependencies: BTreeMap<String, Dependency>,
}

impl Default for DerpyFile {
    fn default() -> Self {
        Self {
            dependencies: BTreeMap::new(),
        }
    }
}

pub fn load_config<P: AsRef<Path>>(path: P) -> Result<DerpyFile, DerpyError> {
    let mut contents = String::new();
    let mut file = match File::open(path) {
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

pub fn save_config<P: AsRef<Path>>(config: &DerpyFile, path: P) -> Result<(), DerpyError> {
    let file = OpenOptions::new()
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

