use super::{app_config_directory, AppFile, YamlFile};

use std::collections::HashMap;
use std::path::PathBuf;

pub struct ConfigFile {
    path: PathBuf,
}
impl ConfigFile {
    pub fn new(path: PathBuf) -> Self {
        Self { path }
    }
}
impl AppFile for ConfigFile {
    fn path(&self) -> &PathBuf {
        &self.path
    }
}
impl YamlFile for ConfigFile {
    type YamlData = ConfigCollection;
}
impl Default for ConfigFile {
    fn default() -> Self {
        Self::new(app_config_directory("config.yaml"))
    }
}

#[derive(Default, Debug, serde::Serialize, serde::Deserialize)]
pub struct ConfigCollection {
    config: HashMap<String, String>,
}
impl ConfigCollection {
    pub fn get_collection(&self) -> &HashMap<String, String> {
        &self.config
    }

    pub fn get_value(&self, key: &str) -> Option<&String> {
        self.config.get(key)
    }

    pub fn set_value(&mut self, key: &str, value: &str) -> Option<String> {
        self.config.insert(String::from(key), String::from(value))
    }

    pub fn unset_value(&mut self, key: &str) -> Option<String> {
        self.config.remove(key)
    }
}
