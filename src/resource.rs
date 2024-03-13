use crate::api::api_client;
use crate::api::{
    AbilityFetcher, EvolutionFetcher, FetchResource, GameFetcher, MoveFetcher, PokemonFetcher,
    SpeciesFetcher, TypeFetcher,
};
use crate::models::resource::InsertRow;

use std::fs;
use std::path::{Path, PathBuf};

use anyhow::Result;
use rusqlite::{Connection, Result as SqlResult};
use rustemon::client::RustemonClient;

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

    fn get_path(&self) -> &PathBuf;
    fn dir() -> PathBuf;
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

    pub async fn build_db(&mut self) -> Result<()> {
        self.create_schema()?;

        println!("retrieving games");
        let games = GameFetcher::fetch_resource(&self.api_client, &self.db).await?;
        self.populate_table(games)?;

        println!("retrieving moves");
        let moves = MoveFetcher::fetch_resource(&self.api_client, &self.db).await?;
        self.populate_table(moves)?;

        println!("retrieving types");
        let types = TypeFetcher::fetch_resource(&self.api_client, &self.db).await?;
        self.populate_table(types)?;

        println!("retrieving abilities");
        let abilities = AbilityFetcher::fetch_resource(&self.api_client, &self.db).await?;
        self.populate_table(abilities)?;

        println!("retrieving species");
        let species = SpeciesFetcher::fetch_resource(&self.api_client, &self.db).await?;
        println!("retrieving evolution");
        let evolutions =
            EvolutionFetcher::fetch_resource(&species, &self.api_client, &self.db).await?;
        self.populate_table(species)?;
        self.populate_table(evolutions)?;

        println!("retrieving pokemon");
        let pokemon = PokemonFetcher::fetch_resource(&self.api_client, &self.db).await?;
        self.populate_table(pokemon)?;

        Ok(())
    }

    fn create_schema(&self) -> SqlResult<()> {
        self.db.execute_batch(include_str!("sql/create_schema.sql"))
    }

    fn populate_table(&mut self, entries: Vec<impl InsertRow>) -> SqlResult<()> {
        let transaction = self.db.transaction()?;
        for entry in entries {
            entry.insert(&transaction)?;
        }
        transaction.commit()
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
