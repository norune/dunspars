pub mod config;
pub mod custom;
pub mod database;

use std::fs;
use std::path::{Path, PathBuf};

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

pub trait AppFile {
    fn build_dir() -> Result<()> {
        if let Some(dir) = Self::path().parent() {
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

    fn path() -> PathBuf;
}

pub trait YamlFile: AppFile {
    type YamlData: serde::Serialize + serde::de::DeserializeOwned;

    fn parse() -> Result<Self::YamlData> {
        Self::build_dir()?;
        let file_string = fs::read_to_string(Self::path())?;
        let result = serde_yaml::from_str(&file_string)?;
        Ok(result)
    }

    fn save(data: Self::YamlData) -> Result<()> {
        Self::build_dir()?;
        let stringified_data = serde_yaml::to_string(&data)?;
        fs::write(Self::path(), stringified_data)?;
        Ok(())
    }
}
