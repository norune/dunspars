use crate::api::api_client;
use crate::api::{
    AbilityFetcher, EvolutionFetcher, FetchResource, GameFetcher, MoveFetcher, PokemonFetcher,
    SpeciesFetcher, TypeFetcher,
};
use crate::models::resource::{InsertRow, MetaRow, SelectRow};
use crate::VERSION;

use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{bail, Result};
use rusqlite::{Connection, Result as SqlResult};

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
    fn build_dir(&self) -> Result<()> {
        if let Some(dir) = self.get_path().parent() {
            if !path_exists(dir) {
                fs::create_dir_all(dir)?;
            }
        }
        Ok(())
    }

    fn get_path(&self) -> &PathBuf;
    fn path() -> PathBuf;
}

fn path_exists(path: &Path) -> bool {
    if let Ok(exists) = path.try_exists() {
        exists
    } else {
        false
    }
}

pub struct DatabaseFile {
    path: PathBuf,
}
impl Default for DatabaseFile {
    fn default() -> Self {
        Self { path: Self::path() }
    }
}
impl DatabaseFile {
    pub fn connect(&self) -> Result<Connection> {
        self.build_dir()?;
        let db = Connection::open(&self.path)?;

        let meta = MetaRow::select_by_name("version", &db);
        if let Ok(db_version) = meta {
            if db_version.value == VERSION {
                Ok(db)
            } else {
                bail!(
                    "Database version '{}' mismatch. Run `dunspars setup` again.",
                    db_version.value
                );
            }
        } else {
            bail!("Database not set up. Run `dunspars setup` first.");
        }
    }

    pub async fn build_db(&self, db: &mut Connection) -> Result<()> {
        let api = api_client();
        fs::remove_file(&self.path)?;
        self.create_schema(db)?;

        println!("retrieving games");
        let games = GameFetcher::fetch_resource(&api, db).await?;
        self.populate_table(games, db)?;

        println!("retrieving moves");
        let moves = MoveFetcher::fetch_resource(&api, db).await?;
        self.populate_table(moves, db)?;

        println!("retrieving types");
        let types = TypeFetcher::fetch_resource(&api, db).await?;
        self.populate_table(types, db)?;

        println!("retrieving abilities");
        let abilities = AbilityFetcher::fetch_resource(&api, db).await?;
        self.populate_table(abilities, db)?;

        println!("retrieving species");
        let species = SpeciesFetcher::fetch_resource(&api, db).await?;
        println!("retrieving evolution");
        let evolutions = EvolutionFetcher::fetch_resource(&species, &api, db).await?;
        self.populate_table(species, db)?;
        self.populate_table(evolutions, db)?;

        println!("retrieving pokemon");
        let pokemon = PokemonFetcher::fetch_resource(&api, db).await?;
        self.populate_table(pokemon, db)?;

        let meta = vec![MetaRow {
            name: String::from("version"),
            value: String::from(VERSION),
        }];
        self.populate_table(meta, db)?;

        Ok(())
    }

    fn create_schema(&self, db: &Connection) -> SqlResult<()> {
        db.execute_batch(include_str!("sql/create_schema.sql"))
    }

    fn populate_table(&self, entries: Vec<impl InsertRow>, db: &mut Connection) -> SqlResult<()> {
        let transaction = db.transaction()?;
        for entry in entries {
            entry.insert(&transaction)?;
        }
        transaction.commit()
    }
}
impl File for DatabaseFile {
    fn path() -> PathBuf {
        app_directory_data("resource.db")
    }

    fn get_path(&self) -> &PathBuf {
        &self.path
    }
}
