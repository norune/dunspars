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
use rusqlite::{Connection, OpenFlags, Result as SqlResult};
use rustemon::client::RustemonClient;

#[derive(Default)]
pub struct ConfigBuilder {
    game: Option<String>,
    color_enabled: Option<bool>,
    db_dir: Option<PathBuf>,
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

    #[allow(dead_code)]
    pub fn db_dir(mut self, db_dir: PathBuf) -> Self {
        self.db_dir = Some(db_dir);
        self
    }

    pub fn build(self) -> Result<Config> {
        let color_enabled = self.color_enabled.unwrap_or(false);
        let db_dir = self.db_dir.unwrap_or(app_data_directory(""));

        Ok(Config {
            game: self.game,
            color_enabled,
            db_dir,
        })
    }
}

#[derive(Clone)]
pub struct Config {
    pub game: Option<String>,
    pub color_enabled: bool,
    pub db_dir: PathBuf,
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

pub fn app_cache_directory(target_dir: &str) -> PathBuf {
    app_directory(AppDirectories::Cache, target_dir)
}
pub fn app_data_directory(target_dir: &str) -> PathBuf {
    app_directory(AppDirectories::Data, target_dir)
}
pub fn app_config_directory(target_dir: &str) -> PathBuf {
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
    fn file_name() -> &'static str;
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
impl DatabaseFile {
    pub fn new(mut dir: PathBuf) -> Self {
        dir.push(Self::file_name());
        Self { path: dir }
    }

    pub fn connect(&self) -> Result<Connection> {
        let mut flags = OpenFlags::default();
        flags.set(OpenFlags::SQLITE_OPEN_READ_WRITE, false);
        flags.set(OpenFlags::SQLITE_OPEN_CREATE, false);
        flags.set(OpenFlags::SQLITE_OPEN_READ_ONLY, true);

        let open = Connection::open_with_flags(&self.path, flags);
        if let Ok(db) = open {
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
                bail!("Database malformed. Run `dunspars setup` again.")
            }
        } else {
            bail!("Database not set up. Run `dunspars setup` first.")
        }
    }

    pub async fn build_db(&self, writer: &mut impl std::io::Write) -> Result<()> {
        self.build_dir()?;
        if path_exists(&self.path) {
            fs::remove_file(&self.path)?;
        }

        let api = api_client();
        let mut db = Connection::open(&self.path)?;

        let start = std::time::Instant::now();

        self.create_schema(&db)?;

        // Games must always be retrieved first as game-to-generation
        // conversion data is needed for the other tables.
        writeln!(writer, "retrieving games")?;
        self.fetch_and_populate::<GameFetcher>(&api, &mut db)
            .await?;

        writeln!(writer, "retrieving moves")?;
        self.fetch_and_populate::<MoveFetcher>(&api, &mut db)
            .await?;

        writeln!(writer, "retrieving types")?;
        self.fetch_and_populate::<TypeFetcher>(&api, &mut db)
            .await?;

        writeln!(writer, "retrieving abilities")?;
        self.fetch_and_populate::<AbilityFetcher>(&api, &mut db)
            .await?;

        writeln!(writer, "retrieving species")?;
        self.fetch_and_populate::<SpeciesFetcher>(&api, &mut db)
            .await?;

        writeln!(writer, "retrieving evolution")?;
        self.fetch_and_populate::<EvolutionFetcher>(&api, &mut db)
            .await?;

        writeln!(writer, "retrieving pokemon")?;
        self.fetch_and_populate::<PokemonFetcher>(&api, &mut db)
            .await?;

        self.populate_meta(&mut db)?;

        let duration = start.elapsed();
        writeln!(writer, "setup time: {}s", duration.as_secs())?;

        Ok(())
    }

    fn create_schema(&self, db: &Connection) -> SqlResult<()> {
        db.execute_batch(include_str!("sql/create_schema.sql"))
    }

    async fn fetch_and_populate<T: FetchResource>(
        &self,
        api: &RustemonClient,
        db: &mut Connection,
    ) -> Result<()> {
        let rows = T::fetch_resource(api, db).await?;
        self.populate_table(rows, db)?;
        Ok(())
    }

    fn populate_table(&self, entries: Vec<impl InsertRow>, db: &mut Connection) -> SqlResult<()> {
        let transaction = db.transaction()?;
        for entry in entries {
            entry.insert(&transaction)?;
        }
        transaction.commit()
    }

    fn populate_meta(&self, db: &mut Connection) -> SqlResult<()> {
        let meta = vec![MetaRow {
            name: String::from("version"),
            value: String::from(VERSION),
        }];
        self.populate_table(meta, db)
    }
}
impl File for DatabaseFile {
    fn file_name() -> &'static str {
        "resource.db"
    }

    fn get_path(&self) -> &PathBuf {
        &self.path
    }
}
