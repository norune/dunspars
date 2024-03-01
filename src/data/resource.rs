use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

use anyhow::{bail, Result};

use rustemon::client::RustemonClient;

use crate::data::api::utils::{self, capture_gen_url};
use crate::data::Game;

pub enum ResourceResult {
    Valid,
    Invalid(Vec<String>),
}

pub trait Resource: Sized {
    fn get_matches(&self, value: &str) -> Vec<String> {
        self.resource()
            .iter()
            .filter_map(|r| {
                let close_enough = if !r.is_empty() && !value.is_empty() {
                    let first_r = r.chars().next().unwrap();
                    let first_value = value.chars().next().unwrap();

                    // Only perform spellcheck on first character match; potentially expensive
                    first_r == first_value && strsim::levenshtein(r, value) < 4
                } else {
                    false
                };

                if r.contains(value) || close_enough {
                    Some(r.clone())
                } else {
                    None
                }
            })
            .collect::<Vec<String>>()
    }

    fn check(&self, value: &str) -> ResourceResult {
        let matches = self.get_matches(value);
        if matches.iter().any(|m| *m == value) {
            ResourceResult::Valid
        } else {
            ResourceResult::Invalid(matches)
        }
    }

    fn validate(&self, value: &str) -> Result<String> {
        let value = value.to_lowercase();
        match self.check(&value) {
            ResourceResult::Valid => Ok(value),
            ResourceResult::Invalid(matches) => bail!(Self::invalid_message(&value, &matches)),
        }
    }

    fn invalid_message(value: &str, matches: &[String]) -> String {
        let resource_name = Self::label();
        let mut message = format!("{resource_name} '{value}' not found.");

        if matches.len() > 20 {
            message += " Potential matches found; too many to display.";
        } else if !matches.is_empty() {
            message += &format!(" Potential matches: {}.", matches.join(" "));
        }

        message
    }

    fn resource(&self) -> Vec<String>;
    fn label() -> &'static str;
}

#[derive(Debug)]
pub struct PokemonResource {
    resource: Vec<String>,
}
impl PokemonResource {
    pub async fn try_new(client: &RustemonClient) -> Result<Self> {
        let resource = utils::get_all_pokemon(client).await?;
        Ok(Self { resource })
    }
}
impl Resource for PokemonResource {
    fn resource(&self) -> Vec<String> {
        self.resource.clone()
    }

    fn label() -> &'static str {
        "Pokémon"
    }
}

#[derive(Debug)]
pub struct TypeResource {
    resource: Vec<String>,
}
impl TypeResource {
    pub async fn try_new(client: &RustemonClient) -> Result<Self> {
        let resource = utils::get_all_types(client).await?;
        Ok(Self { resource })
    }
}
impl Resource for TypeResource {
    fn resource(&self) -> Vec<String> {
        self.resource.clone()
    }

    fn label() -> &'static str {
        "Type"
    }
}

#[derive(Debug)]
pub struct MoveResource {
    resource: Vec<String>,
}
impl MoveResource {
    pub async fn try_new(client: &RustemonClient) -> Result<Self> {
        let resource = utils::get_all_moves(client).await?;
        Ok(Self { resource })
    }
}
impl Resource for MoveResource {
    fn resource(&self) -> Vec<String> {
        self.resource.clone()
    }

    fn label() -> &'static str {
        "Move"
    }
}

#[derive(Debug)]
pub struct AbilityResource {
    resource: Vec<String>,
}
impl AbilityResource {
    pub async fn try_new(client: &RustemonClient) -> Result<Self> {
        let resource = utils::get_all_abilities(client).await?;
        Ok(Self { resource })
    }
}
impl Resource for AbilityResource {
    fn resource(&self) -> Vec<String> {
        self.resource.clone()
    }

    fn label() -> &'static str {
        "Ability"
    }
}

#[derive(Debug)]
pub struct GameResource {
    resource: HashMap<String, Game>,
}
impl GameResource {
    pub fn try_new() -> Result<Self> {
        let mut resource = HashMap::new();

        let data_dir = app_directory_data("resources/games.yaml");
        let game_data: Vec<Game> = serde_yaml::from_str(&fs::read_to_string(data_dir)?)?;

        for game in game_data {
            resource.insert(game.name.clone(), game);
        }

        Ok(Self { resource })
    }
}
impl Resource for GameResource {
    fn resource(&self) -> Vec<String> {
        let mut games = self.resource.iter().map(|r| r.1).collect::<Vec<&Game>>();
        games.sort_by_key(|g| g.order);

        games
            .iter()
            .map(|g| g.name.clone())
            .collect::<Vec<String>>()
    }

    fn label() -> &'static str {
        "Game"
    }
}
pub trait GetGeneration {
    fn get_gen(&self, game: &str) -> u8;
    fn get_gen_from_url(&self, url: &str) -> u8;
}

impl GetGeneration for GameResource {
    fn get_gen(&self, game: &str) -> u8 {
        self.resource.get(game).unwrap().generation
    }

    fn get_gen_from_url(&self, url: &str) -> u8 {
        capture_gen_url(url).unwrap()
    }
}

enum AppDirectories {
    Cache,
    Data,
    Config,
}

impl std::fmt::Display for AppDirectories {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AppDirectories::Cache => write!(f, "Cache"),
            AppDirectories::Data => write!(f, "Data"),
            AppDirectories::Config => write!(f, "Config"),
        }
    }
}

pub fn app_directory_cache(target_dir: &str) -> PathBuf {
    app_directory(AppDirectories::Cache, target_dir)
}
pub fn app_directory_data(target_dir: &str) -> PathBuf {
    app_directory(AppDirectories::Data, target_dir)
}
pub fn app_directory_config(target_dir: &str) -> PathBuf {
    app_directory(AppDirectories::Config, target_dir)
}

fn app_directory(base_dir: AppDirectories, target_dir: &str) -> PathBuf {
    let base_path_buf = match base_dir {
        AppDirectories::Cache => dirs::cache_dir(),
        AppDirectories::Data => dirs::data_local_dir(),
        AppDirectories::Config => dirs::config_local_dir(),
    };
    let mut directory = base_path_buf.unwrap_or_else(|| panic!("{base_dir} directory not found"));

    directory.push(format!("dunspars/{target_dir}"));
    directory
}
