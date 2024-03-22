use super::{app_config_directory, AppFile, YamlFile};
use crate::models::resource::CustomPokemonParams;

use anyhow::Result;

use std::path::PathBuf;

#[derive(Default, serde::Serialize, serde::Deserialize)]
pub struct TrainerFile {
    trainers: Vec<Trainer>,
    version: String,
}
impl TrainerFile {
    pub fn from_file() -> Result<Self> {
        let path = Self::path();
        if Self::path_exists(&path) {
            Ok(Self::parse()?)
        } else {
            Ok(Self::default())
        }
    }
}
impl AppFile for TrainerFile {
    fn path() -> PathBuf {
        app_config_directory("trainers.yaml")
    }
}
impl YamlFile for TrainerFile {
    type YamlData = Self;
}

#[derive(serde::Serialize, serde::Deserialize)]
struct Trainer {
    name: String,
    pokemon: Vec<CustomPokemonParams>,
}
