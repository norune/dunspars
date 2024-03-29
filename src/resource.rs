pub mod config;
pub mod custom;
pub mod database;

use config::ConfigFile;

use std::fs;
use std::path::{Path, PathBuf};
use std::str::FromStr;

use anyhow::Result;

enum AppDirectories {
    Data,
    Config,
}

impl std::fmt::Display for AppDirectories {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AppDirectories::Data => write!(f, "Data"),
            AppDirectories::Config => write!(f, "Config"),
        }
    }
}

pub fn app_data_directory(target_path: &str) -> PathBuf {
    app_directory(AppDirectories::Data, target_path)
}
pub fn app_config_directory(target_path: &str) -> PathBuf {
    app_directory(AppDirectories::Config, target_path)
}

fn app_directory(base_dir: AppDirectories, target_path: &str) -> PathBuf {
    let base_path_buf = match base_dir {
        AppDirectories::Data => dirs::data_local_dir(),
        AppDirectories::Config => dirs::config_local_dir(),
    };
    let mut directory = base_path_buf.unwrap_or_else(|| panic!("{base_dir} directory not found"));

    directory.push(format!("dunspars/{target_path}"));
    directory
}

pub trait AppFile: Default {
    fn build_dir(&self) -> Result<()> {
        if let Some(dir) = self.path().parent() {
            if !Self::path_exists(dir) {
                fs::create_dir_all(dir)?;
            }
        }
        Ok(())
    }

    fn path_exists(path: &Path) -> bool {
        if let Ok(exists) = path.try_exists() {
            exists
        } else {
            false
        }
    }

    fn path(&self) -> &PathBuf;
}

pub trait YamlFile: AppFile {
    type YamlData: serde::Serialize + serde::de::DeserializeOwned + Default;

    fn read(&self) -> Result<Self::YamlData> {
        self.build_dir()?;
        if let Ok(file_string) = fs::read_to_string(self.path()) {
            let parsed_data = serde_yaml::from_str(&file_string)?;
            Ok(parsed_data)
        } else {
            Ok(Self::YamlData::default())
        }
    }

    fn save(&self, data: Self::YamlData) -> Result<()> {
        self.build_dir()?;
        let stringified_data = serde_yaml::to_string(&data)?;
        fs::write(self.path(), stringified_data)?;
        Ok(())
    }
}

#[derive(Default)]
pub struct ConfigBuilder {
    game: Option<String>,
    color_enabled: Option<bool>,
    config_path: Option<PathBuf>,
    db_path: Option<PathBuf>,
    custom_path: Option<PathBuf>,
}
impl ConfigBuilder {
    pub fn from_file(path: Option<PathBuf>) -> Result<Self> {
        let mut builder = ConfigBuilder::default();

        let config_file = if let Some(path) = path {
            builder = builder.config_path(path.clone());
            ConfigFile::new(path)
        } else {
            ConfigFile::default()
        };
        let config = config_file.read()?;

        if let Some(color) = config.get_value("color") {
            if let Ok(color) = color.parse::<bool>() {
                builder = builder.color_enabled(color);
            }
        }

        if let Some(game) = config.get_value("game") {
            builder = builder.game(String::from(game));
        }

        if let Some(db_path) = config.get_value("db_path") {
            if let Ok(path) = PathBuf::from_str(db_path) {
                builder = builder.db_path(path);
            }
        }

        if let Some(custom_path) = config.get_value("custom_path") {
            if let Ok(path) = PathBuf::from_str(custom_path) {
                builder = builder.custom_path(path);
            }
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

    pub fn config_path(mut self, path: PathBuf) -> Self {
        self.config_path = Some(path);
        self
    }

    pub fn db_path(mut self, path: PathBuf) -> Self {
        self.db_path = Some(path);
        self
    }

    pub fn custom_path(mut self, path: PathBuf) -> Self {
        self.custom_path = Some(path);
        self
    }

    pub fn build(self) -> Result<Config> {
        Ok(Config {
            game: self.game,
            color_enabled: self.color_enabled,
            config_path: self.config_path,
            db_path: self.db_path,
            custom_path: self.custom_path,
        })
    }
}

#[derive(Clone)]
pub struct Config {
    pub game: Option<String>,
    pub color_enabled: Option<bool>,
    pub config_path: Option<PathBuf>,
    pub db_path: Option<PathBuf>,
    pub custom_path: Option<PathBuf>,
}
