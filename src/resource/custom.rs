use super::{app_config_directory, AppFile, YamlFile};
use crate::models::resource::{CustomPokemon, Resource};

use anyhow::Result;

use std::path::PathBuf;

#[derive(Default, serde::Serialize, serde::Deserialize)]
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
}
impl AppFile for CustomCollection {
    fn path() -> PathBuf {
        app_config_directory("custom.yaml")
    }
}
impl YamlFile for CustomCollection {
    type YamlData = Self;
}
impl Resource<CustomPokemon> for CustomCollection {
    fn resource(&self) -> Vec<String> {
        self.pokemon.iter().map(|p| p.nickname.clone()).collect()
    }

    fn label() -> &'static str {
        "Custom Pokémon"
    }
}
