use super::{app_config_directory, AppFile, YamlFile};
use crate::cli::utils::is_color_enabled;

use std::collections::HashMap;
use std::path::PathBuf;

use anyhow::Result;

#[derive(Default)]
pub struct ConfigFile {
    config: HashMap<String, String>,
}
impl ConfigFile {
    pub fn from_file() -> Result<Self> {
        let path = Self::path();
        if Self::path_exists(&path) {
            let config = Self::parse()?;
            Ok(Self { config })
        } else {
            Ok(Self::default())
        }
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

    pub fn save(self) -> Result<()> {
        <Self as YamlFile>::save(self.config)?;
        Ok(())
    }
}
impl AppFile for ConfigFile {
    fn path() -> PathBuf {
        app_config_directory("config.yaml")
    }
}
impl YamlFile for ConfigFile {
    type YamlData = HashMap<String, String>;
}

#[derive(Default)]
pub struct ConfigBuilder {
    game: Option<String>,
    color_enabled: Option<bool>,
}
impl ConfigBuilder {
    pub fn from_file() -> Result<Self> {
        let file = ConfigFile::from_file()?;
        let mut builder = ConfigBuilder::default();

        if let Some(color) = file.get_value("color") {
            if let Ok(color) = color.parse::<bool>() {
                builder = builder.color_enabled(color);
            }
        }
        if let Some(game) = file.get_value("game") {
            builder = builder.game(String::from(game));
        }

        Ok(builder)
    }
}
impl ConfigBuilder {
    pub fn game(mut self, game: String) -> Self {
        self.game = Some(game);
        self
    }

    pub fn color_enabled(mut self, color_enabled: bool) -> Self {
        self.color_enabled = Some(color_enabled);
        self
    }

    pub fn build(self) -> Result<Config> {
        let color_enabled = self.color_enabled.unwrap_or(is_color_enabled());

        Ok(Config {
            game: self.game,
            color_enabled,
        })
    }
}

#[derive(Clone)]
pub struct Config {
    pub game: Option<String>,
    pub color_enabled: bool,
}
