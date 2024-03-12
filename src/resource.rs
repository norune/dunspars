use crate::api::api_client;
use crate::api::convert::{
    AbilityFetcher, EvolutionFetcher, FetchEntries, FetchIdentifiers, GameFetcher, MoveFetcher,
    PokemonFetcher, SpeciesFetcher, TypeFetcher,
};
use crate::api::utils::{self, capture_gen_url, get_all_game_data};
use crate::models::resource::InsertRow;
use crate::models::Game;

use std::collections::{HashMap, HashSet};
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use anyhow::{bail, Result};
use rusqlite::{Connection, Result as SqlResult};
use rustemon::client::RustemonClient;

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

        let resource_file = GameResourceFile::try_new()?;
        let game_data: Vec<Game> = resource_file.read_and_parse()?;

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

pub trait File {
    fn build_dir() -> Result<PathBuf> {
        let dir = Self::dir();
        if !path_exists(&dir) {
            fs::create_dir_all(&dir)?;
        }
        Ok(dir)
    }

    fn write(&self, data: &str) -> io::Result<()> {
        fs::write(self.get_path(), data)
    }

    fn read(&self) -> io::Result<String> {
        fs::read_to_string(self.get_path())
    }

    fn get_path(&self) -> &PathBuf;
    fn dir() -> PathBuf;
}

#[allow(async_fn_in_trait)]
pub trait ResourceFile<T: serde::Serialize + serde::de::DeserializeOwned>: File {
    async fn build_if_missing(&self, overwrite: bool) -> Result<()> {
        if overwrite || !path_exists(self.get_path()) {
            let data = Self::get_resource_data().await?;
            let stringified_data = serde_yaml::to_string(&data)?;
            self.write(&stringified_data)?;
        }

        Ok(())
    }

    fn read_and_parse(&self) -> Result<T> {
        let file_data = self.read()?;
        let resource_data: T = serde_yaml::from_str(&file_data)?;
        Ok(resource_data)
    }

    async fn get_resource_data() -> Result<T>;
}

pub struct GameResourceFile {
    path: PathBuf,
}
impl GameResourceFile {
    pub fn try_new() -> Result<Self> {
        let mut path = Self::build_dir()?;
        path.push("games.yaml");
        Ok(Self { path })
    }
}
impl File for GameResourceFile {
    fn dir() -> PathBuf {
        app_directory_data("resources/")
    }

    fn get_path(&self) -> &PathBuf {
        &self.path
    }
}
impl ResourceFile<Vec<Game>> for GameResourceFile {
    async fn get_resource_data() -> Result<Vec<Game>> {
        get_all_game_data().await
    }
}

fn path_exists(path: &Path) -> bool {
    if let Ok(exists) = path.try_exists() {
        exists
    } else {
        false
    }
}

pub struct DatabaseFile {
    pub db: Connection,
    api_client: RustemonClient,
    path: PathBuf,
}
impl DatabaseFile {
    pub fn try_new(overwrite: bool) -> Result<Self> {
        let mut path = Self::build_dir()?;
        path.push("resource.db");

        if overwrite && path_exists(&path) {
            fs::remove_file(&path)?;
        }

        let db = Connection::open(&path)?;
        let api_client = api_client();

        Ok(Self {
            path,
            db,
            api_client,
        })
    }

    pub async fn build_db(&self) -> Result<()> {
        self.create_schema()?;

        println!("retrieving games");
        let game_names = GameFetcher::fetch_all_identifiers(&self.api_client).await?;
        let games = GameFetcher::fetch_all_entries(game_names, &self.api_client, &self.db).await?;
        self.populate_table(games)?;

        println!("retrieving moves");
        let move_names = MoveFetcher::fetch_all_identifiers(&self.api_client).await?;
        let moves = MoveFetcher::fetch_all_entries(move_names, &self.api_client, &self.db).await?;
        self.populate_table(moves)?;

        println!("retrieving types");
        let type_names = TypeFetcher::fetch_all_identifiers(&self.api_client).await?;
        let types = TypeFetcher::fetch_all_entries(type_names, &self.api_client, &self.db).await?;
        self.populate_table(types)?;

        println!("retrieving abilities");
        let ability_names = AbilityFetcher::fetch_all_identifiers(&self.api_client).await?;
        let abilities =
            AbilityFetcher::fetch_all_entries(ability_names, &self.api_client, &self.db).await?;
        self.populate_table(abilities)?;

        println!("retrieving species");
        let species_names = SpeciesFetcher::fetch_all_identifiers(&self.api_client).await?;
        let species =
            SpeciesFetcher::fetch_all_entries(species_names, &self.api_client, &self.db).await?;

        // rustemon::evolution::evolution_chain::get_all_entries() is broken.
        // Retrieve them instead via references from 'pokemon-species' endpoint.
        let mut evolution_ids = HashSet::new();
        species.iter().for_each(|s| {
            if let Some(evolution_id) = s.evolution_id {
                evolution_ids.insert(evolution_id);
            }
        });
        self.populate_table(species)?;

        println!("retrieving evolution");
        let evolutions = EvolutionFetcher::fetch_all_entries(
            evolution_ids.into_iter().collect(),
            &self.api_client,
            &self.db,
        )
        .await?;
        self.populate_table(evolutions)?;

        println!("retrieving pokemon");
        let pokemon_names = PokemonFetcher::fetch_all_identifiers(&self.api_client).await?;
        let pokemon =
            PokemonFetcher::fetch_all_entries(pokemon_names, &self.api_client, &self.db).await?;
        self.populate_table(pokemon)?;

        Ok(())
    }

    fn create_schema(&self) -> SqlResult<()> {
        self.db.execute_batch(include_str!("sql/create_schema.sql"))
    }

    fn populate_table(&self, entries: Vec<impl InsertRow>) -> Result<()> {
        for entry in entries {
            entry.insert(&self.db)?;
        }
        Ok(())
    }
}
impl File for DatabaseFile {
    fn dir() -> PathBuf {
        app_directory_data("")
    }

    fn get_path(&self) -> &PathBuf {
        &self.path
    }
}
