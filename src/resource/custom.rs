use super::{app_config_directory, AppFile, YamlFile};

use anyhow::Result;

use std::path::PathBuf;

#[derive(Default, Debug, serde::Serialize, serde::Deserialize)]
pub struct CustomCollection {
    pokemon: Vec<CustomPokemon>,
}
impl CustomCollection {
    pub fn from_file() -> Result<Self> {
        let path = Self::path();
        if Self::path_exists(&path) {
            Ok(Self::parse()?)
        } else {
            Ok(Self::default())
        }
    }

    pub fn find_pokemon(&self, nickname: &str) -> Option<&CustomPokemon> {
        self.pokemon
            .iter()
            .find(|p| p.nickname.to_lowercase() == nickname.to_lowercase())
    }
}
impl AppFile for CustomCollection {
    fn path() -> PathBuf {
        app_config_directory("custom.yaml")
    }
}
impl YamlFile for CustomCollection {
    type YamlData = Self;
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct CustomPokemon {
    pub nickname: String,
    pub base: String,
    pub generation: u8,
    pub moves: Vec<String>,
    pub types: Option<(String, Option<String>)>,
}
