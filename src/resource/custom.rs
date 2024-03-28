use super::{app_config_directory, AppFile, YamlFile};

use std::path::PathBuf;

#[derive(serde::Serialize, serde::Deserialize)]
pub struct CustomFile {
    path: PathBuf,
}
impl CustomFile {
    pub fn new(path: PathBuf) -> Self {
        Self { path }
    }
}
impl AppFile for CustomFile {
    fn path(&self) -> &PathBuf {
        &self.path
    }
}
impl YamlFile for CustomFile {
    type YamlData = CustomCollection;
}
impl Default for CustomFile {
    fn default() -> Self {
        Self::new(app_config_directory("custom.yaml"))
    }
}

#[derive(Default, Debug, serde::Serialize, serde::Deserialize)]
pub struct CustomCollection {
    pokemon: Vec<CustomPokemon>,
}
impl CustomCollection {
    pub fn find_pokemon(&self, nickname: &str) -> Option<&CustomPokemon> {
        self.pokemon
            .iter()
            .find(|p| p.nickname.to_lowercase() == nickname.to_lowercase())
    }
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct CustomPokemon {
    pub nickname: String,
    pub base: String,
    pub generation: u8,
    pub moves: Vec<String>,
    pub types: Option<(String, Option<String>)>,
}
