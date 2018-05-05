use std::collections::{BTreeMap, HashMap};
use std::path::PathBuf;

#[derive(Serialize, Deserialize, Clone)]
pub struct Dependency {
    pub name: String,
    pub vcs: String,
    pub url: String,
    pub version: String,
    pub target: String,
    pub options: BTreeMap<String, String>,
}

impl Dependency {
    pub fn get_full_path(&self) -> PathBuf {
        PathBuf::from(&self.target).join(&self.name)
    }

    pub fn build_macro_map(&self) -> HashMap<String, String> {
        let mut result = HashMap::new();
        result.insert("DEP_NAME".into(), self.name.clone());
        result.insert("DEP_URL".into(), self.url.clone());
        result.insert("DEP_VERSION".into(), self.version.clone());
        for (key, value) in self.options.iter() {
            result.insert(format!("DEP_OPT_{}", key), value.clone());
        }
        result
    }
}
